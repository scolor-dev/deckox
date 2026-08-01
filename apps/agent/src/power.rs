use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{CommandResult, CommandStatus, SystemCapabilities};
use tokio::process::Command;

use crate::error::AgentError;

static COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct PowerManager {
    allow_reboot: bool,
}

impl PowerManager {
    pub const fn new(allow_reboot: bool) -> Self {
        Self { allow_reboot }
    }

    pub const fn capabilities(&self) -> SystemCapabilities {
        SystemCapabilities {
            reboot_allowed: self.allow_reboot,
        }
    }

    pub async fn reboot(&self) -> Result<CommandResult, AgentError> {
        if !self.allow_reboot {
            return Err(AgentError::forbidden(
                "system reboot is disabled in the Agent configuration",
            ));
        }
        if !cfg!(target_os = "linux") {
            return Err(AgentError::unavailable(
                "system reboot requires a Linux host",
            ));
        }

        let output = tokio::time::timeout(
            Duration::from_secs(10),
            Command::new("systemctl")
                .args(["--no-block", "reboot"])
                .output(),
        )
        .await
        .map_err(|_| AgentError::internal("system reboot command timed out"))?
        .map_err(|error| AgentError::internal(format!("failed to execute systemctl: {error}")))?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(AgentError::internal(if message.is_empty() {
                "system reboot command failed".to_owned()
            } else {
                message
            }));
        }

        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Accepted,
            message: Some("system reboot accepted".to_owned()),
        })
    }
}

fn command_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = COMMAND_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("reboot-{timestamp}-{sequence}")
}

#[cfg(test)]
mod tests {
    use super::PowerManager;

    #[test]
    fn reports_reboot_capability() {
        assert!(PowerManager::new(true).capabilities().reboot_allowed);
        assert!(!PowerManager::new(false).capabilities().reboot_allowed);
    }

    #[tokio::test]
    async fn rejects_reboot_when_disabled() {
        assert!(PowerManager::new(false).reboot().await.is_err());
    }
}
