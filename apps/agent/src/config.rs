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

#[derive(Debug, Default, Deserialize)]
pub struct SoftwareConfig {
    #[serde(default)]
    pub allowed: Vec<String>,
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

    let updated = replace_services_section(&original, &render_services_section(allowed));

    let temp_path = path.with_extension("toml.tmp");
    std::fs::write(&temp_path, updated).map_err(|error| {
        AgentError::internal(format!("failed to write {}: {error}", temp_path.display()))
    })?;
    std::fs::rename(&temp_path, path).map_err(|error| {
        AgentError::internal(format!(
            "failed to replace {} with the updated config: {error}",
            path.display()
        ))
    })
}

/// Rewrites only the `[software]` table of the config file at `path`,
/// leaving everything else untouched — the same purpose as
/// [`write_allowed_services`], but `[software]` is *not* the last table in
/// `packaging/config/agent.toml` (`[services]` follows it), so this replaces
/// only up to the next `[...]` header instead of assuming the rest of the
/// file belongs to this table. Writes via a temp file plus a rename, same as
/// [`write_allowed_services`].
pub fn write_allowed_software(path: &Path, allowed: &[String]) -> Result<(), AgentError> {
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

    let updated = replace_bounded_section(&original, "software", &render_software_section(allowed));

    let temp_path = path.with_extension("software.toml.tmp");
    std::fs::write(&temp_path, updated).map_err(|error| {
        AgentError::internal(format!("failed to write {}: {error}", temp_path.display()))
    })?;
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
fn render_software_section(allowed: &[String]) -> String {
    let mut sorted = allowed.to_vec();
    sorted.sort();

    let mut section = String::from(
        "# Package state can always be read. Install, remove, and upgrade are permitted\n\
         # only for package names listed here. Each name was confirmed, when added, to\n\
         # resolve from this host's own configured package repositories (never a newly\n\
         # added third-party one). Managed from the web admin's Software screen; hand\n\
         # edits are kept as long as this table is followed only by [services] (or\n\
         # nothing).\n\
         [software]\n",
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

/// Replaces the `[table_name]` table in `original` with `new_section`,
/// touching nothing before the header and nothing from the next top-level
/// `[...]` header onward (or nothing after, when `table_name` is the last
/// table or absent). Unlike [`replace_services_section`] — which assumes
/// `[services]` is always the last table — this supports a managed table
/// that has another managed table after it.
fn replace_bounded_section(original: &str, table_name: &str, new_section: &str) -> String {
    let header = format!("[{table_name}]");
    let Some(start) = original.find(&header) else {
        let mut result = original.trim_end().to_owned();
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(new_section);
        return result;
    };

    let search_from = start + header.len();
    let end = original[search_from..]
        .match_indices('\n')
        .map(|(offset, _)| search_from + offset + 1)
        .find(|&line_start| original[line_start..].starts_with('['))
        .unwrap_or(original.len());

    let mut result = original[..start].trim_end().to_owned();
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
    let prefix = original
        .find("[services]")
        .map_or(original, |index| &original[..index]);
    let mut prefix = prefix.trim_end().to_owned();
    if !prefix.is_empty() {
        prefix.push_str("\n\n");
    }
    prefix.push_str(new_section);
    prefix
}

#[cfg(test)]
mod tests {
    use super::{
        AgentConfig, render_services_section, render_software_section, replace_bounded_section,
        replace_services_section,
    };

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
            &render_software_section(&["nginx".to_owned()]),
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
            &render_software_section(&[]),
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
            &render_software_section(&["docker".to_owned(), "nginx".to_owned()]),
        );

        assert!(updated.contains("\"docker\""));
        assert!(updated.contains("\"nginx\""));
    }

    #[test]
    fn render_software_section_sorts_and_quotes_ids() {
        let rendered = render_software_section(&["nginx".to_owned(), "docker".to_owned()]);
        let docker_index = rendered.find("\"docker\"").expect("docker listed");
        let nginx_index = rendered.find("\"nginx\"").expect("nginx listed");
        assert!(docker_index < nginx_index, "allowlist should be sorted");
    }
}
