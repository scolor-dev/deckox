use std::{
    collections::HashSet,
    path::PathBuf,
    process::Output,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{CommandResult, CommandStatus, InstalledSoftware, SoftwarePackage};
use tokio::{
    process::Command,
    sync::{Mutex, RwLock},
};
use tracing::warn;

use crate::{
    config::{self, SoftwareConfig},
    error::AgentError,
};

static COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// There is no fixed software catalog. An admin adds any package by name;
/// the only gate is that [`SoftwareManager::allow`] must first confirm the
/// name resolves from the host's own already-configured package
/// repositories — Deckox never adds a new (e.g. third-party) repository or
/// signing key to make a name resolve. Covers every package manager used by
/// a systemd-based Linux distribution that Deckox's core (Agent) already
/// targets; Alpine is out of scope regardless, since it runs `OpenRC` rather
/// than systemd.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

impl PackageManager {
    async fn installed_version(self, name: &str) -> Option<String> {
        match self {
            Self::Apt => apt_installed_version(name).await,
            Self::Dnf => dnf_installed_version(name).await,
            Self::Pacman => pacman_installed_version(name).await,
            Self::Zypper => zypper_installed_version(name).await,
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
            Self::Pacman => pacman_candidate_version(name).await,
            Self::Zypper => zypper_candidate_version(name).await,
        }
    }

    /// Combined installed+available query for [`SoftwareManager::list`]'s
    /// per-package hot path. For dnf and zypper this is backed by a single
    /// command whose output already carries both fields, so fetching them
    /// through [`Self::installed_version`]/[`Self::candidate_version`]
    /// separately (as those two still do, for callers that only need one)
    /// would needlessly run that command twice per package.
    async fn status(self, name: &str) -> (Option<String>, Option<String>) {
        match self {
            Self::Apt => (
                apt_installed_version(name).await,
                apt_candidate_version(name).await,
            ),
            Self::Dnf => dnf_list(name).await,
            Self::Pacman => (
                pacman_installed_version(name).await,
                pacman_candidate_version(name).await,
            ),
            Self::Zypper => zypper_status(name).await,
        }
    }

    /// Every installed package as `(name, version)`, straight from the
    /// package manager's local database — no repository access, no index
    /// refresh. `None` means the query failed or timed out.
    async fn list_installed(self) -> Option<Vec<(String, String)>> {
        let output = match self {
            Self::Apt => {
                run_query(
                    "dpkg-query",
                    &["-W", "-f=${Package}\t${Version}\t${Status}\n"],
                )
                .await?
            }
            Self::Dnf | Self::Zypper => {
                run_query("rpm", &["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\n"]).await?
            }
            Self::Pacman => run_query("pacman", &["-Q"]).await?,
        };
        let text = String::from_utf8_lossy(&output.stdout);
        Some(match self {
            Self::Apt => parse_dpkg_installed(&text),
            Self::Dnf | Self::Zypper => parse_tab_pairs(&text),
            Self::Pacman => parse_space_pairs(&text),
        })
    }

    /// Names of the packages the admin installed on purpose, as opposed to
    /// ones pulled in as dependencies. `None` when the package manager
    /// cannot say (zypper keeps no such record) or the query failed, in
    /// which case nothing is adopted automatically.
    async fn list_user_installed(self) -> Option<HashSet<String>> {
        match self {
            Self::Apt => {
                let manual = run_query("apt-mark", &["showmanual"]).await?;
                let table = run_query(
                    "dpkg-query",
                    &[
                        "-W",
                        "-f=${Package}\t${Priority}\t${Essential}\t${Depends}\t${Recommends}\t${Status}\n",
                    ],
                )
                .await?;
                Some(parse_apt_user_installed(
                    &String::from_utf8_lossy(&manual.stdout),
                    &String::from_utf8_lossy(&table.stdout),
                ))
            }
            Self::Pacman => {
                let output = run_query("pacman", &["-Qqet"]).await?;
                Some(parse_name_lines(&String::from_utf8_lossy(&output.stdout)))
            }
            Self::Dnf => {
                let output = run_query_within(
                    "dnf",
                    &[
                        "--quiet",
                        "-C",
                        "repoquery",
                        "--userinstalled",
                        "--queryformat",
                        "%{name}\n",
                    ],
                    20,
                )
                .await?;
                Some(parse_name_lines(&String::from_utf8_lossy(&output.stdout)))
            }
            Self::Zypper => None,
        }
    }

    async fn install(self, name: &str) -> Result<(), AgentError> {
        match self {
            Self::Apt => apt_install(name).await,
            Self::Dnf => dnf_install(name).await,
            Self::Pacman => pacman_install(name).await,
            Self::Zypper => zypper_install(name).await,
        }
    }

    /// Deliberately a plain removal, never a purge: configuration and data
    /// are left in place wherever the backend distinguishes the two (e.g.
    /// apt's `remove` vs `purge`). dnf and zypper have no such distinction —
    /// removing a package there can also remove its configuration.
    async fn remove(self, name: &str) -> Result<(), AgentError> {
        match self {
            Self::Apt => apt_remove(name).await,
            Self::Dnf => dnf_remove(name).await,
            Self::Pacman => pacman_remove(name).await,
            Self::Zypper => zypper_remove(name).await,
        }
    }

    async fn upgrade(self, name: &str) -> Result<(), AgentError> {
        match self {
            Self::Apt => apt_upgrade(name).await,
            Self::Dnf => dnf_upgrade(name).await,
            Self::Pacman => pacman_upgrade(name).await,
            Self::Zypper => zypper_upgrade(name).await,
        }
    }
}

/// Detects which package manager this host uses by trying to run each
/// candidate's `--version`. [`Command::output`] fails with `NotFound` when
/// the binary isn't on `PATH` (no shell is involved, so this never risks
/// running anything other than exactly `<binary> --version`), which is all
/// that's needed to tell them apart — the four are mutually exclusive in
/// practice, so detection order does not matter.
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
    if Command::new("pacman")
        .arg("--version")
        .output()
        .await
        .is_ok()
    {
        return Some(PackageManager::Pacman);
    }
    if Command::new("zypper")
        .arg("--version")
        .output()
        .await
        .is_ok()
    {
        return Some(PackageManager::Zypper);
    }
    None
}

#[derive(Debug, Clone)]
struct Settings {
    allowed: HashSet<String>,
    dismissed: HashSet<String>,
    auto_adopt: bool,
}

type DetectionCache = Option<(Instant, HashSet<String>)>;

const DETECTION_TTL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct SoftwareManager {
    settings: Arc<RwLock<Settings>>,
    detected: Arc<Mutex<DetectionCache>>,
    config_path: PathBuf,
    package_manager: Option<PackageManager>,
}

fn validated_names(names: Vec<String>) -> Result<HashSet<String>, AgentError> {
    let mut validated = HashSet::new();
    for name in names {
        validate_package_name(&name)?;
        validated.insert(name);
    }
    Ok(validated)
}

impl SoftwareManager {
    pub fn new(
        config: SoftwareConfig,
        config_path: PathBuf,
        package_manager: Option<PackageManager>,
    ) -> Result<Self, AgentError> {
        Ok(Self {
            settings: Arc::new(RwLock::new(Settings {
                allowed: validated_names(config.allowed)?,
                dismissed: validated_names(config.dismissed)?,
                auto_adopt: config.auto_adopt,
            })),
            detected: Arc::new(Mutex::new(None)),
            config_path,
            package_manager,
        })
    }

    fn persist(&self, settings: &Settings) -> Result<(), AgentError> {
        config::write_software_settings(
            &self.config_path,
            &SoftwareConfig {
                allowed: settings.allowed.iter().cloned().collect(),
                auto_adopt: settings.auto_adopt,
                dismissed: settings.dismissed.iter().cloned().collect(),
            },
        )
    }

    /// Packages installed on purpose, cached briefly because listing and
    /// every action ask for it and the query can be slow (dnf).
    async fn detected(&self, backend: PackageManager) -> HashSet<String> {
        let mut cache = self.detected.lock().await;
        if let Some((taken, names)) = cache.as_ref()
            && taken.elapsed() < DETECTION_TTL
        {
            return names.clone();
        }
        let names: HashSet<String> = backend
            .list_user_installed()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|name| validate_package_name(name).is_ok())
            .collect();
        *cache = Some((Instant::now(), names.clone()));
        names
    }

    /// Packages managed only by automatic adoption: detected, not dismissed.
    async fn adopted(&self, backend: PackageManager) -> HashSet<String> {
        let (auto_adopt, dismissed) = {
            let settings = self.settings.read().await;
            (settings.auto_adopt, settings.dismissed.clone())
        };
        if !auto_adopt {
            return HashSet::new();
        }
        let mut names = self.detected(backend).await;
        names.retain(|name| !dismissed.contains(name));
        names
    }

    /// Adds `name` to the management allowlist, but only after confirming
    /// it resolves from the host's already-configured repositories or is
    /// already installed on this host (a manually installed package no
    /// repository knows about can still be managed) — the dynamic
    /// equivalent of a catalog membership check. Adding a name the admin
    /// had dismissed also lifts the dismissal.
    pub async fn allow(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        validate_package_name(name)?;
        let backend = self.backend()?;
        if backend.candidate_version(name).await.is_none()
            && backend.installed_version(name).await.is_none()
        {
            return Err(AgentError::not_found(format!(
                "{name} was not found in the configured package repositories or among the installed packages"
            )));
        }

        {
            let mut settings = self.settings.write().await;
            let mut updated = settings.clone();
            let added = updated.allowed.insert(name.to_owned());
            let lifted = updated.dismissed.remove(name);
            if added || lifted {
                self.persist(&updated)?;
                *settings = updated;
            }
        }

        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{name} added to the software management allowlist")),
        })
    }

    /// Removes `name` from management. A package that automatic adoption
    /// would otherwise pick up again is remembered as dismissed. Idempotent.
    pub async fn disallow(&self, name: &str) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        validate_package_name(name)?;
        let detected = match self.backend() {
            Ok(backend) => self.detected(backend).await.contains(name),
            Err(_) => false,
        };

        {
            let mut settings = self.settings.write().await;
            let mut updated = settings.clone();
            let removed = updated.allowed.remove(name);
            let dismissed =
                detected && updated.auto_adopt && updated.dismissed.insert(name.to_owned());
            if removed || dismissed {
                self.persist(&updated)?;
                *settings = updated;
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

    /// Reports state for every managed package: the allowlist plus, with
    /// `auto_adopt`, whatever the admin installed on purpose. There is no
    /// wider catalog to browse.
    pub async fn list(&self) -> Result<Vec<SoftwarePackage>, AgentError> {
        ensure_linux()?;
        let backend = self.backend()?;
        let allowed = self.settings.read().await.allowed.clone();
        let adopted = self.adopted(backend).await;
        let mut names: Vec<String> = allowed.union(&adopted).cloned().collect();
        names.sort();

        let mut packages = Vec::with_capacity(names.len());
        for name in names {
            let (installed_version, available_version) = backend.status(&name).await;
            let installed = installed_version.is_some();
            let upgradable =
                installed && available_version.is_some() && available_version != installed_version;

            packages.push(SoftwarePackage {
                auto: !allowed.contains(&name),
                name,
                installed,
                installed_version,
                available_version,
                upgradable,
            });
        }
        Ok(packages)
    }

    /// Every package installed on the host, flagged with whether Deckox
    /// already manages it. Read-only and independent of the allowlist, so an
    /// admin can find what is installed (for example under an unexpected
    /// package name) and add it. Names that could never pass
    /// [`validate_package_name`] are left out, since they could not be added.
    pub async fn list_installed(&self) -> Result<Vec<InstalledSoftware>, AgentError> {
        ensure_linux()?;
        let backend = self.backend()?;
        let installed = backend
            .list_installed()
            .await
            .ok_or_else(|| AgentError::internal("failed to read the installed package list"))?;
        let allowed = self.settings.read().await.allowed.clone();
        let adopted = self.adopted(backend).await;

        let mut packages: Vec<InstalledSoftware> = installed
            .into_iter()
            .filter(|(name, _)| validate_package_name(name).is_ok())
            .map(|(name, version)| InstalledSoftware {
                managed: allowed.contains(&name) || adopted.contains(&name),
                name,
                version,
            })
            .collect();
        packages.sort_by(|a, b| a.name.cmp(&b.name));
        packages.dedup_by(|a, b| a.name == b.name);
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
                "no supported package manager (apt, dnf, pacman, or zypper) was found on this host",
            )
        })
    }

    async fn ensure_allowed(&self, name: &str) -> Result<PackageManager, AgentError> {
        validate_package_name(name)?;
        let backend = self.backend()?;
        if self.settings.read().await.allowed.contains(name)
            || self.adopted(backend).await.contains(name)
        {
            Ok(backend)
        } else {
            Err(AgentError::forbidden(format!(
                "software is not in the management allowlist: {name}"
            )))
        }
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

/// A conservative character class covering Debian's, RPM's (Fedora/openSUSE),
/// and Arch's package naming rules alike (lowercase alphanumerics plus
/// `+-._`; RPM and Arch names are looser and occasionally mixed-case). Not a
/// security boundary by itself — [`std::process::Command`] never invokes a
/// shell, so there is no injection risk from any string reaching it as a
/// single argument — but it rejects obviously-wrong input (spaces, path
/// separators, empty strings) with a clear error before anything is shelled
/// out to.
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

// --- shared process helpers ---------------------------------------------

/// Runs `binary(args)` with a 5-second timeout, returning the process
/// output only on a successful (zero) exit. A non-zero exit is treated the
/// same as "no information" without logging — every backend's read-only
/// probes hit this routinely for "package not found" or "not installed",
/// which is an expected, frequent outcome, not an error. An execution
/// failure or a timeout is logged, since either would be a genuine surprise
/// once a backend has already been selected for this host by
/// [`detect_package_manager`].
async fn run_query(binary: &str, args: &[&str]) -> Option<Output> {
    run_query_within(binary, args, 5).await
}

async fn run_query_within(binary: &str, args: &[&str], timeout_seconds: u64) -> Option<Output> {
    let result = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        Command::new(binary).args(args).output(),
    )
    .await;
    match result {
        Ok(Ok(output)) if output.status.success() => Some(output),
        Ok(Ok(_)) => None,
        Ok(Err(error)) => {
            warn!(%error, package_manager = binary, "failed to execute package manager query");
            None
        }
        Err(_) => {
            warn!(package_manager = binary, "package manager query timed out");
            None
        }
    }
}

/// Runs `binary(args)` non-interactively, returning stdout on success. On
/// failure (non-zero exit, execution error, or timeout) returns stderr
/// trimmed, or a generic `"<binary> command failed/timed out"` message when
/// there is nothing to show — shared by every backend's install/remove/
/// upgrade actions, which differ only in binary, arguments, and any
/// environment variables the binary needs (apt-get's `DEBIAN_FRONTEND`).
async fn run_mutation(
    binary: &str,
    args: &[&str],
    env: &[(&str, &str)],
    timeout_seconds: u64,
) -> Result<String, AgentError> {
    let mut command = Command::new(binary);
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }

    let output = tokio::time::timeout(Duration::from_secs(timeout_seconds), command.output())
        .await
        .map_err(|_| AgentError::internal(format!("{binary} command timed out")))?
        .map_err(|error| AgentError::internal(format!("failed to execute {binary}: {error}")))?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AgentError::internal(if message.is_empty() {
            format!("{binary} command failed")
        } else {
            message
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Finds a `<label>:`-prefixed line (arbitrary whitespace between the label
/// and the colon, as package managers commonly pad for column alignment)
/// and returns its trimmed value, treating an empty value or the literal
/// `(none)` as absent. Shared shape behind apt-cache policy's `Candidate:`,
/// pacman's `-Si` `Version`, and zypper's `info` `Version` field.
fn parse_labeled_version(output: &str, label: &str) -> Option<String> {
    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix(label) else {
            continue;
        };
        let Some(value) = rest.trim_start().strip_prefix(':') else {
            continue;
        };
        let version = value.trim();
        return if version.is_empty() || version == "(none)" {
            None
        } else {
            Some(version.to_owned())
        };
    }
    None
}

// --- apt / dpkg backend -----------------------------------------------

/// `dpkg`'s three-word `Status` field (want/flag/status, e.g.
/// `"install ok installed"`) ends in `"installed"` only when the package is
/// actually present — `"unknown ok not-installed"` ends in the single word
/// `"not-installed"`, which does not equal `"installed"`.
/// A manually installed apt package and the package names it depends on or
/// recommends.
type Candidate = (String, HashSet<String>);

/// The packages the admin installed on purpose, reduced to the ones worth
/// managing on their own.
///
/// Starts from `apt-mark showmanual`, drops what the distribution treats as
/// the base system (`required` / `important` priority, `Essential`), then
/// drops any package another candidate depends on or recommends — Docker's
/// install command marks `docker-ce`, `docker-ce-cli`, the buildx and
/// compose plugins all as manual, but only `docker-ce` is something you
/// would act on. Multiarch names such as `libc6:i386` match by bare name.
fn parse_apt_user_installed(manual: &str, table: &str) -> HashSet<String> {
    let manual: HashSet<&str> = manual
        .lines()
        .map(|line| line.trim().split(':').next().unwrap_or_default())
        .filter(|name| !name.is_empty())
        .collect();

    let mut candidates: Vec<Candidate> = Vec::new();
    for line in table.lines() {
        let mut fields = line.splitn(6, '\t');
        let (
            Some(name),
            Some(priority),
            Some(essential),
            Some(depends),
            Some(recommends),
            Some(status),
        ) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        )
        else {
            continue;
        };
        if manual.contains(name)
            && is_installed_status(status)
            && !matches!(priority, "required" | "important")
            && essential != "yes"
        {
            let mut referenced = parse_dependency_names(depends);
            referenced.extend(parse_dependency_names(recommends));
            candidates.push((name.to_owned(), referenced));
        }
    }

    let referenced_by_others: HashSet<&str> = candidates
        .iter()
        .flat_map(|(owner, referenced)| {
            referenced
                .iter()
                .filter(move |name| name.as_str() != owner)
                .map(String::as_str)
        })
        .collect();
    candidates
        .iter()
        .filter(|(name, _)| !referenced_by_others.contains(name.as_str()))
        .map(|(name, _)| name.clone())
        .collect()
}

/// Package names in a dpkg `Depends` / `Recommends` field, including every
/// `a | b` alternative, without version constraints or `:any` suffixes.
fn parse_dependency_names(field: &str) -> HashSet<String> {
    field
        .split([',', '|'])
        .filter_map(|entry| {
            let name = entry.split(['(', ' ']).find(|part| !part.is_empty())?;
            Some(name.split(':').next().unwrap_or(name).to_owned())
        })
        .collect()
}

fn parse_name_lines(output: &str) -> HashSet<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Parses `dpkg-query -W -f='${Package}\t${Version}\t${Status}\n'`, keeping
/// only packages whose status ends in `installed` (so removed-but-configured
/// `deinstall ok config-files` entries are dropped).
fn parse_dpkg_installed(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next()?;
            let version = fields.next()?;
            let status = fields.next()?;
            (!name.is_empty() && is_installed_status(status))
                .then(|| (name.to_owned(), version.to_owned()))
        })
        .collect()
}

/// Parses `name<TAB>version` lines (`rpm -qa --qf`), skipping the
/// `gpg-pubkey` pseudo-packages rpm records for imported signing keys.
fn parse_tab_pairs(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let (name, version) = line.split_once('\t')?;
            (!name.is_empty() && name != "gpg-pubkey")
                .then(|| (name.to_owned(), version.trim().to_owned()))
        })
        .collect()
}

/// Parses `name version` lines (`pacman -Q`).
fn parse_space_pairs(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let (name, version) = line.split_once(' ')?;
            (!name.is_empty()).then(|| (name.to_owned(), version.trim().to_owned()))
        })
        .collect()
}

fn is_installed_status(status: &str) -> bool {
    status.split_whitespace().last() == Some("installed")
}

/// Reads `dpkg`'s record for `package`, if any. `None` covers both "dpkg
/// has never heard of this package" (the common case before it is ever
/// installed) and any command failure — this is a read-only status probe,
/// so it degrades instead of failing the whole listing.
async fn dpkg_status(package: &str) -> Option<(String, String)> {
    let output = run_query("dpkg-query", &["-W", "-f=${Status}\t${Version}\n", package]).await?;
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
    let output = run_query("apt-cache", &["policy", package]).await?;
    parse_candidate_version(&String::from_utf8_lossy(&output.stdout))
}

fn parse_candidate_version(output: &str) -> Option<String> {
    parse_labeled_version(output, "Candidate")
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
    run_mutation(
        "apt-get",
        args,
        &[("DEBIAN_FRONTEND", "noninteractive")],
        timeout_seconds,
    )
    .await
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
    let Some(output) = run_query("dnf", &["--quiet", "list", package]).await else {
        return (None, None);
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
    run_mutation("dnf", args, &[], timeout_seconds).await
}

// --- pacman backend (Arch Linux and derivatives) ------------------------

/// `pacman -Q <package>` prints a single `"<name> <version>"` line on
/// success (local package database only, no root needed) and exits
/// non-zero with nothing useful when the package is not installed.
async fn pacman_installed_version(package: &str) -> Option<String> {
    let output = run_query("pacman", &["-Q", package]).await?;
    let line = String::from_utf8_lossy(&output.stdout);
    let mut fields = line.lines().next()?.split_whitespace();
    let _name = fields.next()?;
    fields.next().map(str::to_owned)
}

/// `pacman -Si <package>` reads the local sync database (populated by a
/// previous `-Sy`) without refreshing it — deliberately read-only, matching
/// `apt_candidate_version`'s "never runs the refresh itself" convention.
/// Exits non-zero when the name is unknown to every configured repo.
async fn pacman_candidate_version(package: &str) -> Option<String> {
    let output = run_query("pacman", &["-Si", package]).await?;
    parse_labeled_version(&String::from_utf8_lossy(&output.stdout), "Version")
}

/// pacman intentionally never splits "sync the package database" from
/// "apply pending upgrades" into two steps the way apt/dnf split
/// update-then-install: running `-Sy` without `-u` is a well-known pacman
/// footgun that can leave a system in an unsupported "partial upgrade"
/// state, where some packages are refreshed against newer dependencies than
/// others. `-Syu <package>` syncs, upgrades every already-installed
/// package, and ensures `package` is installed/updated, all in one
/// invocation — chosen deliberately over a narrower per-package sync, at
/// the cost of a wider blast radius than the apt/dnf backends: a single
/// "install" or "upgrade" call on a pacman host upgrades the whole system,
/// not just the named package. The longer timeout reflects that a full
/// system upgrade can legitimately take much longer than a single-package
/// apt/dnf install.
async fn pacman_install(package: &str) -> Result<(), AgentError> {
    pacman(&["-Syu", "--noconfirm", package], 600).await?;
    Ok(())
}

/// A plain `-R`, never `-Rn`/`-Rns`: pacman's default removal already skips
/// deleting configuration files it considers user-modified, matching the
/// same "leave data in place where possible" philosophy as `apt_remove`.
async fn pacman_remove(package: &str) -> Result<(), AgentError> {
    pacman(&["-R", "--noconfirm", package], 180).await?;
    Ok(())
}

/// Identical to [`pacman_install`]: pacman has no narrower "upgrade just
/// this one package" operation that avoids the partial-upgrade risk
/// explained there, so upgrading and installing issue the same command.
async fn pacman_upgrade(package: &str) -> Result<(), AgentError> {
    pacman_install(package).await
}

async fn pacman(args: &[&str], timeout_seconds: u64) -> Result<String, AgentError> {
    run_mutation("pacman", args, &[], timeout_seconds).await
}

// --- zypper / rpm backend (openSUSE and derivatives) ---------------------

/// Reads the installed version straight from `rpm` rather than parsing
/// `zypper info`'s output for it: zypper's own `info` field for an
/// installed package is not reliably distinguishable from its "available"
/// counterpart across zypper releases (see [`zypper_status`]), whereas
/// `rpm -q` is an unambiguous, direct read of the local package database —
/// the same reasoning behind the apt backend reading `dpkg-query` instead
/// of `apt` itself for local state.
async fn zypper_installed_version(package: &str) -> Option<String> {
    let output = run_query(
        "rpm",
        &["-q", "--queryformat", "%{VERSION}-%{RELEASE}", package],
    )
    .await?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!version.is_empty()).then_some(version)
}

/// `zypper info <package>` succeeds (with a `Version:` field) whenever the
/// name is known to any configured repository, whether or not it is
/// installed, and exits non-zero when it is unknown to all of them — the
/// existence check `SoftwareManager::allow` relies on.
async fn zypper_candidate_version(package: &str) -> Option<String> {
    let output = run_query("zypper", &["--non-interactive", "info", package]).await?;
    parse_labeled_version(&String::from_utf8_lossy(&output.stdout), "Version")
}

/// Combined installed+available query used by [`PackageManager::status`].
/// When the package is not installed, `zypper info`'s `Version:` field is
/// unambiguously the candidate. When it is installed, that same field's
/// meaning is not reliable enough across zypper versions to also stand in
/// for "is a newer version available" — so the available version instead
/// comes from `zypper list-updates`, the command whose entire purpose is
/// exactly that question.
async fn zypper_status(package: &str) -> (Option<String>, Option<String>) {
    let installed = zypper_installed_version(package).await;
    let available = if installed.is_some() {
        zypper_update_candidate(package).await
    } else {
        zypper_candidate_version(package).await
    };
    (installed, available)
}

/// Reads the pending-upgrade version for an already-installed package from
/// `zypper list-updates`'s table. No matching row means the package is
/// already at its newest available version, mirroring dnf's convention
/// (`parse_dnf_list`) of `None` rather than repeating the installed
/// version.
async fn zypper_update_candidate(package: &str) -> Option<String> {
    let output = run_query("zypper", &["--non-interactive", "list-updates"]).await?;
    parse_zypper_list_updates(&String::from_utf8_lossy(&output.stdout), package)
}

/// `zypper list-updates`' table is pipe-delimited with a fixed column
/// order: `S | Repository | Name | Current Version | Available Version |
/// Arch`. Matches on the `Name` column (index 2) and returns the
/// `Available Version` column (index 4) of the first matching row.
fn parse_zypper_list_updates(output: &str, package: &str) -> Option<String> {
    for line in output.lines() {
        let columns: Vec<&str> = line.split('|').map(str::trim).collect();
        if columns.len() >= 5 && columns[2] == package {
            return Some(columns[4].to_owned());
        }
    }
    None
}

async fn zypper_install(package: &str) -> Result<(), AgentError> {
    zypper(&["--non-interactive", "refresh"], 120).await?;
    zypper(&["--non-interactive", "install", package], 180).await?;
    Ok(())
}

/// A plain `remove`, no `--clean-deps`: leaves now-unneeded dependencies
/// and configuration in place, matching `apt_remove`'s conservative
/// removal.
async fn zypper_remove(package: &str) -> Result<(), AgentError> {
    zypper(&["--non-interactive", "remove", package], 180).await?;
    Ok(())
}

async fn zypper_upgrade(package: &str) -> Result<(), AgentError> {
    zypper(&["--non-interactive", "refresh"], 120).await?;
    zypper(&["--non-interactive", "update", package], 180).await?;
    Ok(())
}

async fn zypper(args: &[&str], timeout_seconds: u64) -> Result<String, AgentError> {
    run_mutation("zypper", args, &[], timeout_seconds).await
}

#[cfg(test)]
mod tests {
    use super::{
        PackageManager, SoftwareManager, is_installed_status, parse_apt_user_installed,
        parse_candidate_version, parse_dependency_names, parse_dnf_list, parse_dpkg_installed,
        parse_labeled_version, parse_space_pairs, parse_tab_pairs, parse_zypper_list_updates,
        validate_package_name,
    };

    fn test_manager(allowed: Vec<String>) -> Result<SoftwareManager, crate::error::AgentError> {
        SoftwareManager::new(
            crate::config::SoftwareConfig {
                allowed,
                auto_adopt: false,
                dismissed: Vec::new(),
            },
            std::path::PathBuf::from("/tmp/deckox-agent-software-test.toml"),
            Some(PackageManager::Apt),
        )
    }

    #[test]
    fn adopts_only_top_level_manual_apt_packages() {
        let manual = "docker-ce\ndocker-ce-cli\ndocker-compose-plugin\ncontainerd.io\ngit\nbash\nlibc6:i386\n";
        let table = "docker-ce\toptional\t\tdocker-ce-cli, containerd.io, iptables | nftables\tdocker-compose-plugin\tinstall ok installed\n\
                     docker-ce-cli\toptional\t\t\t\tinstall ok installed\n\
                     docker-compose-plugin\toptional\t\t\t\tinstall ok installed\n\
                     containerd.io\toptional\t\tlibc6 (>= 2.34)\t\tinstall ok installed\n\
                     git\toptional\t\tlibc6 (>= 2.34), git-man (>> 1:2.43)\t\tinstall ok installed\n\
                     bash\trequired\tyes\tlibc6\t\tinstall ok installed\n\
                     libc6\trequired\tyes\t\t\tinstall ok installed\n\
                     removed-tool\toptional\t\t\t\tdeinstall ok config-files\n";
        let mut adopted: Vec<String> = parse_apt_user_installed(manual, table)
            .into_iter()
            .collect();
        adopted.sort();
        assert_eq!(adopted, ["docker-ce", "git"]);
    }

    #[test]
    fn extracts_dependency_names_without_versions_or_arch() {
        let mut names: Vec<String> =
            parse_dependency_names("libc6 (>= 2.34), python3:any, iptables | nftables")
                .into_iter()
                .collect();
        names.sort();
        assert_eq!(names, ["iptables", "libc6", "nftables", "python3"]);
        assert!(parse_dependency_names("").is_empty());
    }

    #[test]
    fn parses_dpkg_installed_packages_only() {
        let output = "docker-ce\t5:27.0.1-1\tinstall ok installed\n\
                      old-tool\t1.0\tdeinstall ok config-files\n\
                      git\t1:2.43.0\tinstall ok installed\n\
                      broken line\n";
        assert_eq!(
            parse_dpkg_installed(output),
            vec![
                ("docker-ce".to_owned(), "5:27.0.1-1".to_owned()),
                ("git".to_owned(), "1:2.43.0".to_owned()),
            ]
        );
    }

    #[test]
    fn parses_rpm_pairs_and_skips_gpg_keys() {
        let output = "bash\t5.2.26-3.fc40\ngpg-pubkey\t105ef944-65ca83d1\nnginx\t1.24.0-1\n";
        assert_eq!(
            parse_tab_pairs(output),
            vec![
                ("bash".to_owned(), "5.2.26-3.fc40".to_owned()),
                ("nginx".to_owned(), "1.24.0-1".to_owned()),
            ]
        );
    }

    #[test]
    fn parses_pacman_pairs() {
        assert_eq!(
            parse_space_pairs("base 3-2\nlinux 6.9.1.arch1-1\n\n"),
            vec![
                ("base".to_owned(), "3-2".to_owned()),
                ("linux".to_owned(), "6.9.1.arch1-1".to_owned()),
            ]
        );
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

    #[test]
    fn parses_the_pacman_si_version_field() {
        let output = "Repository      : core\n\
             Name            : git\n\
             Version         : 2.43.0-1\n\
             Description     : the fast distributed version control system\n\
             Architecture    : x86_64\n";
        assert_eq!(
            parse_labeled_version(output, "Version").as_deref(),
            Some("2.43.0-1")
        );
    }

    #[test]
    fn treats_a_none_pacman_candidate_as_unavailable() {
        let output = "Version         : (none)\n";
        assert_eq!(parse_labeled_version(output, "Version"), None);
    }

    #[test]
    fn does_not_match_a_label_appearing_only_inside_a_value() {
        let output = "Summary         : Fast Version Control System\n";
        assert_eq!(parse_labeled_version(output, "Version"), None);
    }

    #[test]
    fn parses_the_zypper_info_version_field() {
        let output = "Information for package git:\n\
             -----------------------------\n\
             Repository     : Main Repository\n\
             Name           : git\n\
             Version        : 2.43.0-1.2\n\
             Installed      : Yes\n\
             Status         : up-to-date\n";
        assert_eq!(
            parse_labeled_version(output, "Version").as_deref(),
            Some("2.43.0-1.2")
        );
    }

    #[test]
    fn parses_zypper_list_updates_for_a_matching_package() {
        let output = "S | Repository          | Name | Current Version | Available Version | Arch\n\
             --+---------------------+------+------------------+--------------------+-------\n\
             v | Main Repository     | git  | 2.39.0-1.1       | 2.43.0-1.2         | x86_64\n";
        assert_eq!(
            parse_zypper_list_updates(output, "git").as_deref(),
            Some("2.43.0-1.2")
        );
    }

    #[test]
    fn parses_zypper_list_updates_as_absent_when_package_is_not_listed() {
        let output = "S | Repository          | Name | Current Version | Available Version | Arch\n\
             --+---------------------+------+------------------+--------------------+-------\n";
        assert_eq!(parse_zypper_list_updates(output, "git"), None);
    }
}
