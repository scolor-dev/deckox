use std::os::unix::fs::MetadataExt;
use std::{
    env,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::AgentError;

const DEFAULT_CONFIG_PATH: &str = "/etc/deckox/agent.toml";
const DEFAULT_SOCKET_PATH: &str = "/run/deckox/agent.sock";

#[derive(Debug, Default, Deserialize)]
pub struct AgentConfig {
    pub socket: Option<PathBuf>,
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub software: SoftwareConfig,
    #[serde(default)]
    pub services: ServicesConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct SystemConfig {
    #[serde(default)]
    pub allow_reboot: bool,
    #[serde(default)]
    pub allow_update: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SoftwareConfig {
    #[serde(default)]
    pub allowed: Vec<String>,
    /// When `true` (the default), packages the admin installed on purpose are
    /// managed without being listed in `allowed`.
    #[serde(default = "default_auto_adopt")]
    pub auto_adopt: bool,
    /// Packages the admin removed from management by hand; they are never
    /// adopted automatically again.
    #[serde(default)]
    pub dismissed: Vec<String>,
}

const fn default_auto_adopt() -> bool {
    true
}

impl Default for SoftwareConfig {
    fn default() -> Self {
        Self {
            allowed: Vec::new(),
            auto_adopt: true,
            dismissed: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ServicesConfig {
    #[serde(default)]
    pub allowed: Vec<String>,
}

impl AgentConfig {
    pub fn resolve_path() -> PathBuf {
        PathBuf::from(
            env::var("DECKOX_AGENT_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_owned()),
        )
    }

    pub fn load() -> Result<Self, AgentError> {
        let path = Self::resolve_path();

        let mut config = match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).map_err(|error| {
                AgentError::internal(format!("invalid config {}: {error}", path.display()))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                return Err(AgentError::internal(format!(
                    "failed to read config {}: {error}",
                    path.display()
                )));
            }
        };

        if let Ok(socket) = env::var("DECKOX_AGENT_SOCKET") {
            config.socket = Some(PathBuf::from(socket));
        }

        Ok(config)
    }

    pub fn socket_path(&self) -> PathBuf {
        self.socket
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET_PATH))
    }
}

/// Rewrites only the `[services]` table of the config file at `path` to the
/// given allowlist, leaving everything before it (including comments)
/// untouched. Used so the web-managed allowlist survives an Agent restart
/// without a full TOML round-trip, which would strip the shipped config's
/// explanatory comments.
///
/// Assumes `[services]` is the last table in the file, matching
/// `packaging/config/agent.toml`'s layout: anything already written after
/// it would be silently replaced. Writes via a temp file in the same
/// directory plus a rename, so a crash mid-write cannot corrupt the config
/// that is already on disk.
pub fn write_allowed_services(path: &Path, allowed: &[String]) -> Result<(), AgentError> {
    rewrite_config(path, "toml.tmp", |original| {
        replace_services_section(original, &render_services_section(allowed))
    })
}

/// Rewrites only the `[software]` table of the config file at `path`,
/// leaving everything else untouched — the same purpose as
/// [`write_allowed_services`], but `[software]` is *not* the last table in
/// `packaging/config/agent.toml` (`[services]` follows it), so this replaces
/// only up to the next `[...]` header instead of assuming the rest of the
/// file belongs to this table. Writes via a temp file plus a rename, same as
/// [`write_allowed_services`].
pub fn write_software_settings(path: &Path, settings: &SoftwareConfig) -> Result<(), AgentError> {
    rewrite_config(path, "software.toml.tmp", |original| {
        replace_bounded_section(original, "software", &render_software_section(settings))
    })
}

/// Reads the config, applies `transform`, and swaps the result in through a
/// temp file plus a rename. The file's mode and owner are carried over so a
/// rewrite by the Agent never loosens the permissions the installer set.
fn rewrite_config(
    path: &Path,
    temp_extension: &str,
    transform: impl FnOnce(&str) -> String,
) -> Result<(), AgentError> {
    let original = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(AgentError::internal(format!(
                "failed to read config {}: {error}",
                path.display()
            )));
        }
    };
    let metadata = std::fs::metadata(path).ok();

    let temp_path = path.with_extension(temp_extension);
    std::fs::write(&temp_path, transform(&original)).map_err(|error| {
        AgentError::internal(format!("failed to write {}: {error}", temp_path.display()))
    })?;
    if let Some(metadata) = metadata {
        let _ = std::fs::set_permissions(&temp_path, metadata.permissions());
        let _ = std::os::unix::fs::chown(&temp_path, Some(metadata.uid()), Some(metadata.gid()));
    }
    std::fs::rename(&temp_path, path).map_err(|error| {
        AgentError::internal(format!(
            "failed to replace {} with the updated config: {error}",
            path.display()
        ))
    })
}

/// Each entry here has already been confirmed, at the time it was added, to
/// resolve from the host's own already-configured package repositories —
/// there is no fixed catalog to cross-reference.
fn render_string_list(section: &mut String, key: &str, values: &[String]) {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.dedup();
    if sorted.is_empty() {
        let _ = writeln!(section, "{key} = []");
    } else {
        let _ = writeln!(section, "{key} = [");
        for id in &sorted {
            let _ = writeln!(section, "  \"{id}\",");
        }
        section.push_str("]\n");
    }
}

fn render_software_section(settings: &SoftwareConfig) -> String {
    let mut section = String::from(
        "# Package state can always be read. Install, remove, and upgrade are permitted\n\
         # only for package names that are managed: those listed in `allowed`, plus (when\n\
         # `auto_adopt` is true) packages installed on purpose that are not in\n\
         # `dismissed`. Names in `allowed` were confirmed, when added, to resolve from\n\
         # this host's own configured package repositories or to be installed already\n\
         # (never a newly added third-party repository). Managed from the web admin's\n\
         # Software screen; hand edits are kept as long as this table is followed only\n\
         # by [services] (or nothing).\n\
         [software]\n",
    );
    let _ = writeln!(section, "auto_adopt = {}", settings.auto_adopt);
    render_string_list(&mut section, "allowed", &settings.allowed);
    render_string_list(&mut section, "dismissed", &settings.dismissed);
    section
}

/// Replaces the `[table_name]` table in `original` with `new_section`,
/// touching nothing before the header and nothing from the next top-level
/// `[...]` header onward (or nothing after, when `table_name` is the last
/// table or absent). Unlike [`replace_services_section`] — which assumes
/// `[services]` is always the last table — this supports a managed table
/// that has another managed table after it.
fn replace_bounded_section(original: &str, table_name: &str, new_section: &str) -> String {
    let Some(start) = table_header_offset(original, table_name) else {
        let mut result = original.trim_end().to_owned();
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(new_section);
        return result;
    };

    let header_line_end = original[start..]
        .find('\n')
        .map_or(original.len(), |offset| start + offset + 1);
    let end = table_end(original, header_line_end);

    let mut result = strip_trailing_comment_block(&original[..start]);
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(new_section);
    if end < original.len() {
        result.push('\n');
        result.push_str(&original[end..]);
    }
    result
}

fn render_services_section(allowed: &[String]) -> String {
    let mut sorted = allowed.to_vec();
    sorted.sort();

    let mut section = String::from(
        "# Service state can always be read. Start, stop, and restart are permitted only\n\
         # for service IDs listed here. Deckox's own services are always rejected.\n\
         # Managed from the web admin's Services screen; hand edits are kept as long as\n\
         # this stays the last table in the file.\n\
         [services]\n",
    );
    if sorted.is_empty() {
        section.push_str("allowed = []\n");
    } else {
        section.push_str("allowed = [\n");
        for id in &sorted {
            let _ = writeln!(section, "  \"{id}\",");
        }
        section.push_str("]\n");
    }
    section
}

fn replace_services_section(original: &str, new_section: &str) -> String {
    let prefix =
        table_header_offset(original, "services").map_or(original, |index| &original[..index]);
    let mut prefix = strip_trailing_comment_block(prefix);
    if !prefix.is_empty() {
        prefix.push_str("\n\n");
    }
    prefix.push_str(new_section);
    prefix
}

/// Byte offset of the line that is the `[name]` table header. Matching whole
/// lines matters: the generated comments above the tables mention `[services]`
/// in prose, and a plain substring search would cut the file there.
fn table_header_offset(text: &str, name: &str) -> Option<usize> {
    let header = format!("[{name}]");
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with(&header) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Drops the comment block directly above a table header. Each rewritten
/// section brings its own comment, so keeping the old one would duplicate it
/// on every write.
fn strip_trailing_comment_block(prefix: &str) -> String {
    let mut lines: Vec<&str> = prefix.trim_end().lines().collect();
    while lines
        .last()
        .is_some_and(|line| line.trim_start().starts_with('#'))
    {
        lines.pop();
    }
    lines.join("\n").trim_end().to_owned()
}

/// Where the table starting at `from` ends: at the next table header, minus
/// the comment block that belongs to that header (which must survive).
fn table_end(text: &str, from: usize) -> usize {
    let mut offset = from;
    let mut comment_start: Option<usize> = None;
    for line in text[from..].split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            return comment_start.unwrap_or(offset);
        }
        if trimmed.starts_with('#') {
            comment_start.get_or_insert(offset);
        } else {
            comment_start = None;
        }
        offset += line.len();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::{
        AgentConfig, SoftwareConfig, render_services_section, render_software_section,
        replace_bounded_section, replace_services_section,
    };

    fn settings(allowed: &[String]) -> SoftwareConfig {
        SoftwareConfig {
            allowed: allowed.to_vec(),
            ..SoftwareConfig::default()
        }
    }

    #[test]
    fn parses_allowed_services() {
        let config: AgentConfig = toml::from_str(
            r#"
socket = "/tmp/deckox.sock"

[system]
allow_reboot = true

[software]
allowed = ["nginx"]

[services]
allowed = ["nginx.service", "postgresql.service"]
"#,
        )
        .expect("config should parse");

        assert_eq!(config.services.allowed.len(), 2);
        assert_eq!(config.software.allowed, vec!["nginx".to_owned()]);
        assert!(config.system.allow_reboot);
        assert!(!config.system.allow_update);
        assert_eq!(
            config
                .socket
                .expect("socket should exist")
                .to_string_lossy(),
            "/tmp/deckox.sock"
        );
    }

    #[test]
    fn parses_config_without_a_software_table() {
        let config: AgentConfig =
            toml::from_str("socket = \"/tmp/deckox.sock\"\n\n[services]\nallowed = []\n")
                .expect("config should parse");

        assert!(config.software.allowed.is_empty());
    }

    #[test]
    fn replace_services_section_keeps_everything_before_it() {
        let original = "socket = \"/run/deckox/agent.sock\"\n\n\
             # a comment worth keeping\n\
             [system]\n\
             allow_reboot = true\n\n\
             [services]\n\
             allowed = [\"old.service\"]\n";

        let updated = replace_services_section(
            original,
            &render_services_section(&["nginx.service".to_owned()]),
        );

        assert!(updated.contains("# a comment worth keeping"));
        assert!(updated.contains("allow_reboot = true"));
        assert!(updated.contains("\"nginx.service\""));
        assert!(!updated.contains("old.service"));
    }

    #[test]
    fn replace_services_section_appends_when_absent() {
        let updated = replace_services_section(
            "socket = \"/run/deckox/agent.sock\"\n",
            &render_services_section(&[]),
        );

        assert!(updated.starts_with("socket = \"/run/deckox/agent.sock\""));
        assert!(updated.contains("[services]"));
        assert!(updated.contains("allowed = []"));
    }

    #[test]
    fn render_services_section_sorts_and_quotes_ids() {
        let rendered = render_services_section(&["b.service".to_owned(), "a.service".to_owned()]);
        let a_index = rendered.find("\"a.service\"").expect("a.service listed");
        let b_index = rendered.find("\"b.service\"").expect("b.service listed");
        assert!(a_index < b_index, "allowlist should be sorted");
    }

    #[test]
    fn replace_bounded_section_leaves_a_later_table_untouched() {
        let original = "socket = \"/tmp/deckox.sock\"\n\n\
             [system]\n\
             allow_reboot = true\n\n\
             [software]\n\
             allowed = [\"docker\"]\n\n\
             [services]\n\
             allowed = [\"old.service\"]\n";

        let updated = replace_bounded_section(
            original,
            "software",
            &render_software_section(&settings(&["nginx".to_owned()])),
        );

        assert!(updated.contains("allow_reboot = true"));
        assert!(
            updated.contains("  \"nginx\",\n"),
            "nginx should be listed: {updated}"
        );
        assert!(
            !updated.contains("  \"docker\",\n"),
            "docker should no longer be listed: {updated}"
        );
        assert!(
            updated.contains("[services]\nallowed = [\"old.service\"]"),
            "the [services] table after [software] must survive untouched: {updated}"
        );
    }

    #[test]
    fn replace_bounded_section_appends_when_absent() {
        let updated = replace_bounded_section(
            "socket = \"/tmp/deckox.sock\"\n",
            "software",
            &render_software_section(&settings(&[])),
        );

        assert!(updated.starts_with("socket = \"/tmp/deckox.sock\""));
        assert!(updated.contains("[software]"));
        assert!(updated.contains("allowed = []"));
    }

    #[test]
    fn replace_bounded_section_replaces_a_final_table_like_replace_services_section() {
        let original = "socket = \"/tmp/deckox.sock\"\n\n[software]\nallowed = [\"docker\"]\n";

        let updated = replace_bounded_section(
            original,
            "software",
            &render_software_section(&settings(&["docker".to_owned(), "nginx".to_owned()])),
        );

        assert!(updated.contains("\"docker\""));
        assert!(updated.contains("\"nginx\""));
    }

    #[test]
    fn software_settings_round_trip_including_auto_adopt_and_dismissed() {
        let rendered = render_software_section(&SoftwareConfig {
            allowed: vec!["nginx".to_owned()],
            auto_adopt: false,
            dismissed: vec!["docker-ce-cli".to_owned(), "docker-ce-cli".to_owned()],
        });
        let parsed: AgentConfig = toml::from_str(&rendered).expect("rendered section parses");
        assert_eq!(parsed.software.allowed, vec!["nginx".to_owned()]);
        assert!(!parsed.software.auto_adopt);
        assert_eq!(parsed.software.dismissed, vec!["docker-ce-cli".to_owned()]);
    }

    #[test]
    fn software_auto_adopt_defaults_to_on() {
        let parsed: AgentConfig = toml::from_str("[software]\nallowed = []\n").expect("parses");
        assert!(parsed.software.auto_adopt);
        assert!(parsed.software.dismissed.is_empty());
    }

    #[test]
    fn render_software_section_sorts_and_quotes_ids() {
        let rendered =
            render_software_section(&settings(&["nginx".to_owned(), "docker".to_owned()]));
        let docker_index = rendered.find("\"docker\"").expect("docker listed");
        let nginx_index = rendered.find("\"nginx\"").expect("nginx listed");
        assert!(docker_index < nginx_index, "allowlist should be sorted");
    }

    #[test]
    fn a_later_tables_comment_survives_rewriting_the_table_before_it() {
        let once = apply_software(SHIPPED_CONFIG, &["nginx"]);
        assert!(
            once.contains("# Service state can always be read"),
            "the comment above [services] must be kept\n{once}"
        );
    }

    const SHIPPED_CONFIG: &str = include_str!("../../../packaging/config/agent.toml");

    fn apply_software(original: &str, allowed: &[&str]) -> String {
        let allowed: Vec<String> = allowed.iter().map(|name| (*name).to_owned()).collect();
        replace_bounded_section(
            original,
            "software",
            &render_software_section(&settings(&allowed)),
        )
    }

    #[test]
    fn shipped_config_survives_repeated_rewrites_of_both_tables() {
        let mut config = SHIPPED_CONFIG.to_owned();
        for round in 0..3 {
            config = apply_software(&config, &["nginx", "docker-ce"]);
            config = replace_services_section(
                &config,
                &render_services_section(&["nginx.service".to_owned()]),
            );
            let parsed: AgentConfig = toml::from_str(&config)
                .unwrap_or_else(|error| panic!("round {round}: {error}\n{config}"));
            assert_eq!(
                parsed.software.allowed,
                vec!["docker-ce".to_owned(), "nginx".to_owned()],
                "round {round}: the [software] table must survive a [services] rewrite\n{config}"
            );
            assert_eq!(parsed.services.allowed, vec!["nginx.service".to_owned()]);
            assert_eq!(
                config.matches("\n[software]\n").count(),
                1,
                "round {round}\n{config}"
            );
            assert_eq!(
                config.matches("\n[services]\n").count(),
                1,
                "round {round}\n{config}"
            );
        }
    }

    #[test]
    fn rewriting_a_table_does_not_pile_up_its_comment_block() {
        let once = apply_software(SHIPPED_CONFIG, &["nginx"]);
        let twice = apply_software(&once, &["nginx", "git"]);
        assert_eq!(
            once.matches("Package state can always be read").count(),
            twice.matches("Package state can always be read").count(),
            "the generated comment must not be duplicated on every write\n{twice}"
        );
        assert!(twice.contains("allow_reboot"), "[system] must be kept");
    }

    #[test]
    fn rewriting_keeps_the_files_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("deckox-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("agent.toml");
        std::fs::write(&path, SHIPPED_CONFIG).expect("write config");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).expect("chmod");

        super::write_allowed_services(&path, &["nginx.service".to_owned()]).expect("rewrite");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o640);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
