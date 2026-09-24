//! `deckox-server doctor` — a one-shot health check for an SSH console, for
//! when the Web UI is not reachable. Every check prints one `[ OK ]`,
//! `[WARN]` or `[FAIL]` line; the command fails only when something is
//! actually broken, so it can be used in scripts.

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use deckox_protocol::AgentStatus;

use crate::{agent_client::AgentClient, auth, request_context::RequestId};

const DEFAULT_AGENT_CONFIG: &str = "/etc/deckox/agent.toml";
const VERSION_FILE: &str = "/usr/local/share/deckox/VERSION";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
pub struct Check {
    pub level: Level,
    pub name: &'static str,
    pub detail: String,
}

impl Check {
    fn new(level: Level, name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            level,
            name,
            detail: detail.into(),
        }
    }

    fn render(&self) -> String {
        let tag = match self.level {
            Level::Ok => "[ OK ]",
            Level::Warn => "[WARN]",
            Level::Fail => "[FAIL]",
        };
        format!("{tag} {:<13} {}", self.name, self.detail)
    }
}

pub async fn run() -> Result<(), String> {
    let mut checks = vec![check_version(
        env!("CARGO_PKG_VERSION"),
        std::fs::read_to_string(VERSION_FILE).ok().as_deref(),
    )];
    checks.extend(check_units());
    checks.push(check_agent().await);
    checks.push(check_agent_security().await);
    checks.extend(check_agent_config(&agent_config_path()));
    checks.push(check_server_config(&crate::modules::config_path()));
    checks.push(check_web());
    checks.push(check_account());

    for check in &checks {
        println!("{}", check.render());
    }
    let failures = checks
        .iter()
        .filter(|check| check.level == Level::Fail)
        .count();
    let warnings = checks
        .iter()
        .filter(|check| check.level == Level::Warn)
        .count();
    println!();
    if failures > 0 {
        return Err(format!(
            "{failures} check(s) failed, {warnings} warning(s). See the [FAIL] lines above."
        ));
    }
    println!("No failures ({warnings} warning(s)).");
    Ok(())
}

fn agent_config_path() -> PathBuf {
    PathBuf::from(
        env::var("DECKOX_AGENT_CONFIG").unwrap_or_else(|_| DEFAULT_AGENT_CONFIG.to_owned()),
    )
}

/// Compares the running binary with the installed version marker, which the
/// installer writes last: a mismatch means an update stopped half-way or the
/// service was not restarted after one.
pub fn check_version(binary: &str, marker: Option<&str>) -> Check {
    let Some(marker) = marker.and_then(|text| text.lines().next()) else {
        return Check::new(
            Level::Warn,
            "version",
            format!("{binary} (no version marker found)"),
        );
    };
    let marker = marker.trim().trim_start_matches('v');
    if marker == binary {
        Check::new(Level::Ok, "version", binary)
    } else {
        Check::new(
            Level::Warn,
            "version",
            format!("binary {binary} differs from installed marker {marker}; restart the services"),
        )
    }
}

fn check_units() -> Vec<Check> {
    ["deckox-agent.service", "deckox-server.service"]
        .into_iter()
        .map(|unit| {
            let name = if unit.contains("agent") {
                "agent unit"
            } else {
                "server unit"
            };
            match Command::new("systemctl").args(["is-active", unit]).output() {
                Ok(output) => {
                    let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    if state == "active" {
                        Check::new(Level::Ok, name, format!("{unit} is active"))
                    } else {
                        Check::new(
                            Level::Fail,
                            name,
                            format!("{unit} is {state}; see: journalctl -u {unit} -n 50"),
                        )
                    }
                }
                Err(_) => Check::new(Level::Warn, name, "systemctl is not available here"),
            }
        })
        .collect()
}

async fn check_agent() -> Check {
    let socket = PathBuf::from(
        env::var("DECKOX_AGENT_SOCKET").unwrap_or_else(|_| crate::DEFAULT_AGENT_SOCKET.to_owned()),
    );
    let request_id = RequestId(format!("cli-{}", hex::encode(rand::random::<[u8; 8]>())));
    match AgentClient::new(socket.clone())
        .get_json::<AgentStatus>("/v1/status", &request_id)
        .await
    {
        Ok(status) => Check::new(
            Level::Ok,
            "agent",
            format!(
                "{} ({} {})",
                status.hostname, status.operating_system, status.architecture
            ),
        ),
        Err(error) => Check::new(
            Level::Fail,
            "agent",
            format!("cannot reach the Agent at {}: {error}", socket.display()),
        ),
    }
}

/// Whether the Agent can check the admin password itself (see
/// `GET /v1/security`). Without a hash it refuses every operation that asks
/// for the password, so this is a failure, not a warning.
async fn check_agent_security() -> Check {
    let socket = PathBuf::from(
        env::var("DECKOX_AGENT_SOCKET").unwrap_or_else(|_| crate::DEFAULT_AGENT_SOCKET.to_owned()),
    );
    let request_id = RequestId(format!("cli-{}", hex::encode(rand::random::<[u8; 8]>())));
    match AgentClient::new(socket)
        .get_json::<serde_json::Value>("/v1/security", &request_id)
        .await
    {
        Ok(status) => describe_security(&status),
        Err(error) => Check::new(
            Level::Warn,
            "agent password",
            format!("could not read the Agent's security status: {error}"),
        ),
    }
}

pub fn describe_security(status: &serde_json::Value) -> Check {
    let confirm = status["confirm"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    if status["verifier_provisioned"] != true {
        return Check::new(
            Level::Fail,
            "agent password",
            "the Agent has no admin password to check; run: printf '%s' 'NEW-PASSWORD' | sudo deckox-server reset-password",
        );
    }
    if let Some(seconds) = status["locked_for_seconds"].as_u64() {
        return Check::new(
            Level::Warn,
            "agent password",
            format!("locked for {seconds}s after repeated wrong passwords"),
        );
    }
    Check::new(
        Level::Ok,
        "agent password",
        format!("the Agent checks the password itself for: {confirm}"),
    )
}

/// Reads agent.toml the way the Agent will: it must parse, and each table
/// should be present. A missing table is a warning (defaults apply), a
/// syntax error is a failure (the Agent refuses to start).
/// The install profile whose module set `disabled` matches, or `custom` when
/// the list was edited by hand. `develop` shares the `all` module set.
fn profile_of(disabled: &[&str]) -> &'static str {
    let mut sorted = disabled.to_vec();
    sorted.sort_unstable();
    match sorted.as_slice() {
        [] => "all",
        ["schedules", "software"] => "normal",
        ["backups", "power", "schedules", "software", "update"] => "lite",
        _ => "custom",
    }
}

pub fn check_agent_config(path: &Path) -> Vec<Check> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            return vec![Check::new(
                Level::Warn,
                "agent.toml",
                format!(
                    "cannot read {} (run as root or as a member of the deckox group)",
                    path.display()
                ),
            )];
        }
        Err(error) => {
            return vec![Check::new(
                Level::Fail,
                "agent.toml",
                format!("cannot read {}: {error}", path.display()),
            )];
        }
    };
    let value: toml::Value = match toml::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return vec![Check::new(
                Level::Fail,
                "agent.toml",
                format!(
                    "{} is not valid TOML and the Agent will not start: {error}",
                    path.display()
                ),
            )];
        }
    };

    let count = |table: &str, key: &str| {
        value
            .get(table)
            .and_then(|table| table.get(key))
            .and_then(toml::Value::as_array)
            .map_or(0, Vec::len)
    };
    let flag = |table: &str, key: &str, default: bool| {
        value
            .get(table)
            .and_then(|table| table.get(key))
            .and_then(toml::Value::as_bool)
            .unwrap_or(default)
    };
    let disabled: Vec<&str> = value
        .get("modules")
        .and_then(|table| table.get("disabled"))
        .and_then(toml::Value::as_array)
        .map(|items| items.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();
    let disabled_modules = if disabled.is_empty() {
        "none".to_owned()
    } else {
        disabled.join(", ")
    };
    let profile = profile_of(&disabled);
    let mut checks = vec![Check::new(
        Level::Ok,
        "agent.toml",
        format!(
            "profile {}, reboot {}, update {}, {} allowed service(s), {} managed package(s), auto_adopt {}, modules off: {}",
            profile,
            if flag("system", "allow_reboot", false) {
                "allowed"
            } else {
                "off"
            },
            if flag("system", "allow_update", false) {
                "allowed"
            } else {
                "off"
            },
            count("services", "allowed"),
            count("software", "allowed"),
            if flag("software", "auto_adopt", true) {
                "on"
            } else {
                "off"
            },
            disabled_modules,
        ),
    )];
    for table in ["system", "software", "services"] {
        if value.get(table).is_none() {
            checks.push(Check::new(
                Level::Warn,
                "agent.toml",
                format!("[{table}] is missing, defaults apply (compare with the backups: ls /etc/deckox)"),
            ));
        }
    }
    checks
}

pub fn check_server_config(path: &Path) -> Check {
    match crate::modules::disabled_from_file(path) {
        Ok(disabled) => {
            let from_environment = env::var("DECKOX_DISABLED_MODULES")
                .is_ok_and(|value| value.split(',').any(|name| !name.trim().is_empty()));
            let listed = if disabled.is_empty() {
                "none".to_owned()
            } else {
                disabled.join(", ")
            };
            let note = if from_environment {
                " (DECKOX_DISABLED_MODULES overrides it)"
            } else {
                ""
            };
            Check::new(
                Level::Ok,
                "server.toml",
                format!("modules off: {listed}{note}"),
            )
        }
        Err(error) => Check::new(Level::Fail, "server.toml", error),
    }
}

fn check_web() -> Check {
    let dir = PathBuf::from(
        env::var("DECKOX_WEB_DIR").unwrap_or_else(|_| crate::DEFAULT_WEB_DIR.to_owned()),
    );
    if dir.join("index.html").is_file() {
        Check::new(Level::Ok, "web files", dir.display().to_string())
    } else {
        Check::new(
            Level::Fail,
            "web files",
            format!("{}/index.html is missing", dir.display()),
        )
    }
}

fn check_account() -> Check {
    let Some(path) = auth::account_path_from_env() else {
        return Check::new(
            Level::Ok,
            "account",
            "password comes from DECKOX_ADMIN_PASSWORD_HASH (two-factor is not used)",
        );
    };
    match auth::load_admin_account(&path) {
        Ok(account) => Check::new(
            Level::Ok,
            "account",
            account.totp.map_or_else(
                || "admin account found, two-factor disabled".to_owned(),
                |totp| {
                    format!(
                        "admin account found, two-factor enabled ({} recovery code(s) left)",
                        totp.recovery_code_hashes.len()
                    )
                },
            ),
        ),
        Err(error) => Check::new(Level::Fail, "account", error),
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, check_agent_config, check_server_config, check_version, describe_security};

    fn config_file(contents: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "deckox-doctor-{}-{}",
            std::process::id(),
            contents.len()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("agent.toml");
        std::fs::write(&path, contents).expect("write");
        path
    }

    #[test]
    fn version_marker_must_match_the_binary() {
        assert_eq!(check_version("2.0.0", Some("2.0.0\n")).level, Level::Ok);
        assert_eq!(check_version("2.0.0", Some("v2.0.0")).level, Level::Ok);
        assert_eq!(check_version("2.0.0", Some("1.9.9")).level, Level::Warn);
        assert_eq!(check_version("2.0.0", None).level, Level::Warn);
    }

    #[test]
    fn a_healthy_config_reports_its_settings() {
        let path = config_file(
            "[system]\nallow_reboot = true\n[software]\nauto_adopt = false\nallowed = [\"git\"]\n[services]\nallowed = [\"a.service\", \"b.service\"]\n",
        );
        let checks = check_agent_config(&path);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].level, Level::Ok);
        assert!(
            checks[0].detail.contains("reboot allowed"),
            "{}",
            checks[0].detail
        );
        assert!(
            checks[0].detail.contains("2 allowed service(s)"),
            "{}",
            checks[0].detail
        );
        assert!(
            checks[0].detail.contains("auto_adopt off"),
            "{}",
            checks[0].detail
        );
    }

    #[test]
    fn the_install_profile_is_read_from_the_disabled_modules() {
        let detail = |disabled: &str| {
            let path = config_file(&format!(
                "[system]\n[software]\n[services]\n[modules]\ndisabled = [{disabled}]\n"
            ));
            check_agent_config(&path)[0].detail.clone()
        };
        assert!(detail("").contains("profile all,"));
        assert!(detail("\"software\", \"schedules\"").contains("profile normal,"));
        assert!(
            detail("\"update\", \"power\", \"backups\", \"schedules\", \"software\"")
                .contains("profile lite,")
        );
        assert!(detail("\"software\"").contains("profile custom,"));
    }

    #[test]
    fn server_toml_modules_are_reported_and_a_typo_fails() {
        let path = config_file("[modules]\ndisabled = [\"audit\"]\n");
        let check = check_server_config(&path);
        assert_eq!(check.level, Level::Ok);
        assert!(
            check.detail.contains("modules off: audit"),
            "{}",
            check.detail
        );
        let broken = config_file("[modules\ndisabled = [");
        assert_eq!(check_server_config(&broken).level, Level::Fail);
        let missing = check_server_config(std::path::Path::new("/nonexistent/server.toml"));
        assert_eq!(missing.level, Level::Ok);
    }

    #[test]
    fn missing_tables_warn_and_bad_syntax_fails() {
        let damaged = config_file("[services]\nallowed = [\"docker.service\"]\n");
        let checks = check_agent_config(&damaged);
        assert_eq!(
            checks
                .iter()
                .filter(|check| check.level == Level::Warn)
                .count(),
            2,
            "[system] and [software] are reported missing"
        );

        let broken = config_file("[services\nallowed = [");
        assert_eq!(check_agent_config(&broken)[0].level, Level::Fail);
        assert_eq!(
            check_agent_config(std::path::Path::new("/nonexistent/agent.toml"))[0].level,
            Level::Fail
        );
    }

    #[test]
    fn the_agents_password_check_is_reported() {
        use serde_json::json;

        let ok = describe_security(
            &json!({"verifier_provisioned": true, "locked_for_seconds": null, "confirm": ["system_reboot"]}),
        );
        assert_eq!(ok.level, Level::Ok);
        assert!(ok.detail.contains("system_reboot"));

        let missing = describe_security(
            &json!({"verifier_provisioned": false, "locked_for_seconds": null, "confirm": []}),
        );
        assert_eq!(missing.level, Level::Fail);
        assert!(missing.detail.contains("reset-password"));

        let locked = describe_security(
            &json!({"verifier_provisioned": true, "locked_for_seconds": 120, "confirm": []}),
        );
        assert_eq!(locked.level, Level::Warn);
    }
}
