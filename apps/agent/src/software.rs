use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{CommandResult, CommandStatus, SoftwarePackage};
use tokio::{process::Command, sync::RwLock};
use tracing::warn;

use crate::{config, error::AgentError};

static COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// There is no fixed software catalog. An admin adds any package by name;
/// the only gate is that [`SoftwareManager::allow`] must first confirm the
/// name resolves from the host's own already-configured package
/// repositories — Deckox never adds a new (e.g. third-party) repository or
/// signing key to make a name resolve. Support for other package managers
/// (pacman, zypper, ...) can be added as further variants without changing
/// [`SoftwareManager`]'s public shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
}

impl PackageManager {
    async fn installed_version(self, name: &str) -> Option<String> {
        match self {
            Self::Apt => apt_installed_version(name).await,
            Self::Dnf => dnf_installed_version(name).await,
        }
    }

    /// The version the package manager would currently install — used both
    /// to display "what's available" and, by [`SoftwareManager::allow`], as
    /// the existence check: `None` means the name is not known to any of
    /// the host's configured repositories.
    async fn candidate_version(self, name: &str) -> Option<String> {
        match self {
            Self::Apt => apt_candidate_version(name).await,
            Self::Dnf => dnf_candidate_version(name).await,
        }
    }

    async fn install(self, name: &str) -> Result<(), AgentError> {
        match self {
            Self::Apt => apt_install(name).await,
            Self::Dnf => dnf_install(name).await,
        }
    }

    /// Deliberately a plain removal, never a purge: configuration and data
    /// are left in place wherever the backend distinguishes the two (e.g.
    /// apt's `remove` vs `purge`). dnf has no such distinction — removing a
    /// package there can also remove its configuration.
    async fn remove(self, name: &str) -> Result<(), AgentError> {
        match self {
            Self::Apt => apt_remove(name).await,
            Self::Dnf => dnf_remove(name).await,
        }
    }

    async fn upgrade(self, name: &str) -> Result<(), AgentError> {
        match self {
            Self::Apt => apt_upgrade(name).await,
            Self::Dnf => dnf_upgrade(name).await,
        }
    }
}

/// Detects which package manager this host uses by trying to run each
/// candidate's `--version`. [`Command::output`] fails with `NotFound` when
/// the binary isn't on `PATH` (no shell is involved, so this never risks
/// running anything other than exactly `<binary> --version`), which is all
/// that's needed to tell the two apart.
pub async fn detect_package_manager() -> Option<PackageManager> {
    if Command::new("apt-get")
        .arg("--version")
        .output()
        .await
        .is_ok()
    {
        return Some(PackageManager::Apt);
    }
    if Command::new("dnf").arg("--version").output().await.is_ok() {
        return Some(PackageManager::Dnf);
    }
    None
}

#[derive(Clone)]
pub struct SoftwareManager {
    allowed: Arc<RwLock<HashSet<String>>>,
    config_path: PathBuf,
    package_manager: Option<PackageManager>,
}

impl SoftwareManager {
    pub fn new(
        allowed: Vec<String>,
        config_path: PathBuf,
        package_manager: Option<PackageManager>,
    ) -> Result<Self, AgentError> {
        let mut validated = HashSet::new();
        for name in allowed {
            validate_package_name(&name)?;
            validated.insert(name);
        }
        Ok(Self {
            allowed: Arc::new(RwLock::new(validated)),
            config_path,
            package_manager,
        })
    }

    /// Adds `name` to the management allowlist, but only after confirming
    /// it resolves from the host's already-configured repositories — the
    /// dynamic equivalent of a catalog membership check. Persists the same
    /// way [`crate::services::ServiceManager::allow`] does for services.
    pub async fn allow(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        validate_package_name(name)?;
        let backend = self.backend()?;
        if backend.candidate_version(name).await.is_none() {
            return Err(AgentError::not_found(format!(
                "{name} was not found in the configured package repositories"
            )));
        }

        {
            let mut allowed = self.allowed.write().await;
            if !allowed.contains(name) {
                let mut updated: Vec<String> = allowed.iter().cloned().collect();
                updated.push(name.to_owned());
                config::write_allowed_software(&self.config_path, &updated)?;
                allowed.insert(name.to_owned());
            }
        }

        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{name} added to the software management allowlist")),
        })
    }

    /// Removes `name` from the management allowlist. Idempotent.
    pub async fn disallow(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        validate_package_name(name)?;

        {
            let mut allowed = self.allowed.write().await;
            if allowed.contains(name) {
                let updated: Vec<String> = allowed
                    .iter()
                    .filter(|existing| existing.as_str() != name)
                    .cloned()
                    .collect();
                config::write_allowed_software(&self.config_path, &updated)?;
                allowed.remove(name);
            }
        }

        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!(
                "{name} removed from the software management allowlist"
            )),
        })
    }

    /// Reports state for every allow-listed package — there is no wider
    /// catalog or host-wide package list to browse, only what an admin has
    /// already vetted and added.
    pub async fn list(&self) -> Result<Vec<SoftwarePackage>, AgentError> {
        ensure_linux()?;
        let backend = self.backend()?;
        let mut names: Vec<String> = self.allowed.read().await.iter().cloned().collect();
        names.sort();

        let mut packages = Vec::with_capacity(names.len());
        for name in names {
            let installed_version = backend.installed_version(&name).await;
            let installed = installed_version.is_some();
            let available_version = backend.candidate_version(&name).await;
            let upgradable =
                installed && available_version.is_some() && available_version != installed_version;

            packages.push(SoftwarePackage {
                name,
                installed,
                installed_version,
                available_version,
                upgradable,
            });
        }
        Ok(packages)
    }

    pub async fn install(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        let backend = self.ensure_allowed(name).await?;
        backend.install(name).await?;
        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{name} installed")),
        })
    }

    pub async fn remove(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        let backend = self.ensure_allowed(name).await?;
        backend.remove(name).await?;
        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{name} removed")),
        })
    }

    pub async fn upgrade(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        let backend = self.ensure_allowed(name).await?;
        if backend.installed_version(name).await.is_none() {
            return Err(AgentError::conflict(format!("{name} is not installed")));
        }
        backend.upgrade(name).await?;
        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{name} upgraded")),
        })
    }

    fn backend(&self) -> Result<PackageManager, AgentError> {
        self.package_manager.ok_or_else(|| {
            AgentError::unavailable(
                "no supported package manager (apt or dnf) was found on this host",
            )
        })
    }

    async fn ensure_allowed(&self, name: &str) -> Result<PackageManager, AgentError> {
        validate_package_name(name)?;
        if !self.allowed.read().await.contains(name) {
            return Err(AgentError::forbidden(format!(
                "software is not in the management allowlist: {name}"
            )));
        }
        self.backend()
    }
}

fn ensure_linux() -> Result<(), AgentError> {
    if cfg!(target_os = "linux") {
        Ok(())
    } else {
        Err(AgentError::unavailable(
            "software management requires a Linux host",
        ))
    }
}

/// A conservative character class covering both Debian's and RPM's package
/// naming rules (Debian: lowercase alphanumerics plus `+-.`; RPM names are
/// looser and occasionally mixed-case). Not a security boundary by itself —
/// [`std::process::Command`] never invokes a shell, so there is no
/// injection risk from any string reaching it as a single argument — but it
/// rejects obviously-wrong input (spaces, path separators, empty strings)
/// with a clear error before anything is shelled out to.
fn validate_package_name(name: &str) -> Result<(), AgentError> {
    let valid = !name.is_empty()
        && name.len() <= 100
        && name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+-._".contains(&byte));

    if valid {
        Ok(())
    } else {
        Err(AgentError::bad_request("invalid package name"))
    }
}

fn command_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = COMMAND_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("cmd-{timestamp}-{sequence}")
}

// --- apt / dpkg backend -----------------------------------------------

/// `dpkg`'s three-word `Status` field (want/flag/status, e.g.
/// `"install ok installed"`) ends in `"installed"` only when the package is
/// actually present — `"unknown ok not-installed"` ends in the single word
/// `"not-installed"`, which does not equal `"installed"`.
fn is_installed_status(status: &str) -> bool {
    status.split_whitespace().last() == Some("installed")
}

/// Reads `dpkg`'s record for `package`, if any. `None` covers both "dpkg
/// has never heard of this package" (the common case before it is ever
/// installed) and any command failure — this is a read-only status probe,
/// so it degrades instead of failing the whole listing.
async fn dpkg_status(package: &str) -> Option<(String, String)> {
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("dpkg-query")
            .args(["-W", "-f=${Status}\t${Version}\n", package])
            .output(),
    )
    .await;

    let output = match result {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            warn!(%error, "failed to execute dpkg-query");
            return None;
        }
        Err(_) => {
            warn!("dpkg-query timed out");
            return None;
        }
    };
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?;
    let (status, version) = line.split_once('\t')?;
    Some((status.to_owned(), version.to_owned()))
}

async fn apt_installed_version(package: &str) -> Option<String> {
    let (status, version) = dpkg_status(package).await?;
    is_installed_status(&status).then_some(version)
}

/// Reads the version `apt install` would currently pick, from its existing
/// local cache — deliberately never runs `apt update`, matching
/// `diagnostics::read_upgradable_package_count`'s read-only convention.
async fn apt_candidate_version(package: &str) -> Option<String> {
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("apt-cache").args(["policy", package]).output(),
    )
    .await;

    let output = match result {
        Ok(Ok(output)) if output.status.success() => output,
        Ok(Ok(_)) => {
            warn!("apt-cache policy failed");
            return None;
        }
        Ok(Err(error)) => {
            warn!(%error, "failed to execute apt-cache");
            return None;
        }
        Err(_) => {
            warn!("apt-cache policy timed out");
            return None;
        }
    };
    parse_candidate_version(&String::from_utf8_lossy(&output.stdout))
}

fn parse_candidate_version(output: &str) -> Option<String> {
    for line in output.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("Candidate:") {
            let version = rest.trim();
            return if version.is_empty() || version == "(none)" {
                None
            } else {
                Some(version.to_owned())
            };
        }
    }
    None
}

async fn apt_install(package: &str) -> Result<(), AgentError> {
    apt_get(&["update"], 120).await?;
    apt_get(&["install", "-y", package], 180).await?;
    Ok(())
}

async fn apt_remove(package: &str) -> Result<(), AgentError> {
    apt_get(&["remove", "-y", package], 180).await?;
    Ok(())
}

async fn apt_upgrade(package: &str) -> Result<(), AgentError> {
    apt_get(&["update"], 120).await?;
    apt_get(&["install", "--only-upgrade", "-y", package], 180).await?;
    Ok(())
}

/// Runs `apt-get` non-interactively with a longer timeout than the
/// short-lived `systemctl`/`journalctl` calls in `services.rs` — package
/// installs can legitimately take tens of seconds.
async fn apt_get(args: &[&str], timeout_seconds: u64) -> Result<String, AgentError> {
    let output = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        Command::new("apt-get")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .args(args)
            .output(),
    )
    .await
    .map_err(|_| AgentError::internal("apt-get command timed out"))?
    .map_err(|error| AgentError::internal(format!("failed to execute apt-get: {error}")))?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AgentError::internal(if message.is_empty() {
            "apt-get command failed".to_owned()
        } else {
            message
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// --- dnf / rpm backend --------------------------------------------------

async fn dnf_installed_version(package: &str) -> Option<String> {
    dnf_list(package).await.0
}

async fn dnf_candidate_version(package: &str) -> Option<String> {
    let (installed, available) = dnf_list(package).await;
    available.or(installed)
}

/// `dnf --quiet list <package>` prints an `Installed Packages` section when
/// the package is present, an `Available Packages` section when a version
/// (installed or not) is offered by a configured repo, or fails with a
/// non-zero exit and no useful output when the name is unknown to every
/// configured repo. Returns `(installed_version, available_version)`; both
/// `None` means dnf does not know this package at all.
async fn dnf_list(package: &str) -> (Option<String>, Option<String>) {
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("dnf")
            .args(["--quiet", "list", package])
            .output(),
    )
    .await;

    let output = match result {
        Ok(Ok(output)) if output.status.success() => output,
        Ok(Ok(_)) => return (None, None),
        Ok(Err(error)) => {
            warn!(%error, "failed to execute dnf");
            return (None, None);
        }
        Err(_) => {
            warn!("dnf list timed out");
            return (None, None);
        }
    };
    parse_dnf_list(&String::from_utf8_lossy(&output.stdout))
}

fn parse_dnf_list(output: &str) -> (Option<String>, Option<String>) {
    let mut installed = None;
    let mut available = None;
    let mut section: Option<&str> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        match trimmed {
            "Installed Packages" => {
                section = Some("installed");
                continue;
            }
            "Available Packages" => {
                section = Some("available");
                continue;
            }
            "" => continue,
            _ => {}
        }
        let mut fields = trimmed.split_whitespace();
        let Some(_name_arch) = fields.next() else {
            continue;
        };
        let Some(version) = fields.next() else {
            continue;
        };
        match section {
            Some("installed") => installed = Some(version.to_owned()),
            Some("available") => available = Some(version.to_owned()),
            _ => {}
        }
    }
    (installed, available)
}

async fn dnf_install(package: &str) -> Result<(), AgentError> {
    dnf(&["makecache", "--refresh"], 120).await?;
    dnf(&["install", "-y", package], 180).await?;
    Ok(())
}

async fn dnf_remove(package: &str) -> Result<(), AgentError> {
    dnf(&["remove", "-y", package], 180).await?;
    Ok(())
}

async fn dnf_upgrade(package: &str) -> Result<(), AgentError> {
    dnf(&["makecache", "--refresh"], 120).await?;
    dnf(&["upgrade", "-y", package], 180).await?;
    Ok(())
}

async fn dnf(args: &[&str], timeout_seconds: u64) -> Result<String, AgentError> {
    let output = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        Command::new("dnf").args(args).output(),
    )
    .await
    .map_err(|_| AgentError::internal("dnf command timed out"))?
    .map_err(|error| AgentError::internal(format!("failed to execute dnf: {error}")))?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AgentError::internal(if message.is_empty() {
            "dnf command failed".to_owned()
        } else {
            message
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        PackageManager, SoftwareManager, is_installed_status, parse_candidate_version,
        parse_dnf_list, validate_package_name,
    };

    fn test_manager(allowed: Vec<String>) -> Result<SoftwareManager, crate::error::AgentError> {
        SoftwareManager::new(
            allowed,
            std::path::PathBuf::from("/tmp/deckox-agent-software-test.toml"),
            Some(PackageManager::Apt),
        )
    }

    #[test]
    fn accepts_typical_package_names() {
        for name in ["docker", "docker.io", "python3.11", "lib-ssl_dev+", "a"] {
            assert!(
                validate_package_name(name).is_ok(),
                "{name} should be valid"
            );
        }
    }

    #[test]
    fn rejects_malformed_package_names() {
        for name in ["", "-docker", "has space", "semi;colon", "../etc/passwd"] {
            assert!(
                validate_package_name(name).is_err(),
                "{name} should be rejected"
            );
        }
        let too_long = "a".repeat(101);
        assert!(validate_package_name(&too_long).is_err());
    }

    #[test]
    fn rejects_malformed_names_at_construction() {
        assert!(test_manager(vec!["docker".to_owned()]).is_ok());
        assert!(test_manager(vec!["; rm -rf /".to_owned()]).is_err());
    }

    #[tokio::test]
    async fn enforces_allowlist_by_exact_name() {
        let manager = test_manager(vec!["docker".to_owned()]).expect("allowlist is valid");
        assert!(manager.ensure_allowed("docker").await.is_ok());
        assert!(manager.ensure_allowed("nginx").await.is_err());
    }

    #[test]
    fn recognizes_the_installed_status_and_nothing_else() {
        assert!(is_installed_status("install ok installed"));
        assert!(!is_installed_status("deinstall ok config-files"));
        assert!(!is_installed_status("unknown ok not-installed"));
    }

    #[test]
    fn parses_the_apt_candidate_version_line() {
        let output = "docker.io:\n  Installed: (none)\n  Candidate: 20.10.24-0ubuntu1~20.04.1\n  Version table:\n";
        assert_eq!(
            parse_candidate_version(output).as_deref(),
            Some("20.10.24-0ubuntu1~20.04.1")
        );
    }

    #[test]
    fn treats_a_none_apt_candidate_as_unavailable() {
        let output = "unknown-package:\n  Installed: (none)\n  Candidate: (none)\n";
        assert_eq!(parse_candidate_version(output), None);
    }

    #[test]
    fn parses_dnf_list_with_an_upgrade_pending() {
        let output = "Installed Packages\n\
             git.x86_64                    2.39.3-1.el9                  @baseos\n\
             Available Packages\n\
             git.x86_64                    2.43.0-1.el9                  updates\n";
        let (installed, available) = parse_dnf_list(output);
        assert_eq!(installed.as_deref(), Some("2.39.3-1.el9"));
        assert_eq!(available.as_deref(), Some("2.43.0-1.el9"));
    }

    #[test]
    fn parses_dnf_list_when_already_up_to_date() {
        let output = "Installed Packages\n\
             git.x86_64                    2.43.0-1.el9                  @baseos\n";
        let (installed, available) = parse_dnf_list(output);
        assert_eq!(installed.as_deref(), Some("2.43.0-1.el9"));
        assert_eq!(available, None);
    }

    #[test]
    fn parses_dnf_list_when_not_installed() {
        let output = "Available Packages\n\
             docker-ce.x86_64               3:24.0.7-1.el9                docker-ce-stable\n";
        let (installed, available) = parse_dnf_list(output);
        assert_eq!(installed, None);
        assert_eq!(available.as_deref(), Some("3:24.0.7-1.el9"));
    }

    #[test]
    fn parses_dnf_list_for_an_unknown_package_as_empty() {
        assert_eq!(parse_dnf_list(""), (None, None));
    }
}
