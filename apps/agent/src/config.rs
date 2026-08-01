use std::{env, path::PathBuf};

use serde::Deserialize;

use crate::error::AgentError;

const DEFAULT_CONFIG_PATH: &str = "/etc/deckox/agent.toml";
const DEFAULT_SOCKET_PATH: &str = "/run/deckox/agent.sock";

#[derive(Debug, Default, Deserialize)]
pub struct AgentConfig {
    pub socket: Option<PathBuf>,
    #[serde(default)]
    pub services: ServicesConfig,
    #[serde(default)]
    pub ssh: SshConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct ServicesConfig {
    #[serde(default)]
    pub allowed: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SshConfig {
    pub managed_user: Option<String>,
}

impl AgentConfig {
    pub fn load() -> Result<Self, AgentError> {
        let path = PathBuf::from(
            env::var("DECKOX_AGENT_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_owned()),
        );

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

#[cfg(test)]
mod tests {
    use super::AgentConfig;

    #[test]
    fn parses_allowed_services() {
        let config: AgentConfig = toml::from_str(
            r#"
socket = "/tmp/deckox.sock"

[services]
allowed = ["nginx.service", "postgresql.service"]

[ssh]
managed_user = "operator"
"#,
        )
        .expect("config should parse");

        assert_eq!(config.services.allowed.len(), 2);
        assert_eq!(config.ssh.managed_user.as_deref(), Some("operator"));
        assert_eq!(
            config
                .socket
                .expect("socket should exist")
                .to_string_lossy(),
            "/tmp/deckox.sock"
        );
    }
}
