use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use deckox_protocol::{CommandResult, CommandStatus, is_valid_release_tag};
use tokio::process::Command;

use crate::error::AgentError;

const MAX_SCRIPT_BYTES: usize = 256 * 1024;
const UPDATE_UNIT_NAME: &str = "deckox-update";
const SPAWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs a self-update by staging the installer script fetched by the Server
/// (which, unlike the root-privileged Agent, is allowed to reach the
/// network) and launching it as an independent transient systemd unit.
///
/// The installer restarts `deckox-agent.service` as one of its final steps,
/// so it must not run inside that unit's own cgroup: `systemctl restart`
/// would kill the whole cgroup, including the still-running installer,
/// before it can finish or roll back. `systemd-run` gives it a separate
/// unit and cgroup instead.
#[derive(Clone)]
pub struct UpdateManager {
    allow_update: bool,
    script_path: PathBuf,
}

impl UpdateManager {
    pub fn new(allow_update: bool, runtime_dir: &Path) -> Self {
        Self {
            allow_update,
            script_path: runtime_dir.join("update.sh"),
        }
    }

    pub const fn allowed(&self) -> bool {
        self.allow_update
    }

    pub async fn trigger(
        &self,
        target_version: &str,
        install_script: &str,
    ) -> Result<CommandResult, AgentError> {
        if !self.allow_update {
            return Err(AgentError::forbidden(
                "self-update is disabled in the Agent configuration",
            ));
        }
        if !cfg!(target_os = "linux") {
            return Err(AgentError::unavailable("self-update requires a Linux host"));
        }
        if !is_valid_release_tag(target_version) {
            return Err(AgentError::bad_request("invalid target version"));
        }
        if install_script.is_empty() || install_script.len() > MAX_SCRIPT_BYTES {
            return Err(AgentError::bad_request("invalid install script"));
        }

        write_script(&self.script_path, install_script)
            .map_err(|error| AgentError::internal(format!("failed to stage installer: {error}")))?;

        let output = tokio::time::timeout(
            SPAWN_TIMEOUT,
            Command::new("systemd-run")
                .arg("--collect")
                .arg("--no-block")
                .arg(format!("--unit={UPDATE_UNIT_NAME}"))
                .arg(format!("--setenv=DECKOX_VERSION={target_version}"))
                .arg("--")
                .arg("/bin/sh")
                .arg(&self.script_path)
                .output(),
        )
        .await
        .map_err(|_| AgentError::internal("update launch timed out"))?
        .map_err(|error| AgentError::internal(format!("failed to execute systemd-run: {error}")))?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(AgentError::conflict(if message.is_empty() {
                "update launch failed".to_owned()
            } else {
                message
            }));
        }

        Ok(CommandResult {
            command_id: format!("update-{target_version}"),
            status: CommandStatus::Accepted,
            message: Some(format!("update to {target_version} accepted")),
        })
    }
}

fn write_script(path: &Path, content: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::UpdateManager;

    #[tokio::test]
    async fn rejects_trigger_when_disabled() {
        let manager = UpdateManager::new(false, std::path::Path::new("/tmp"));
        assert!(manager.trigger("v1.2.3", "#!/bin/sh\n").await.is_err());
    }

    #[tokio::test]
    async fn rejects_invalid_version() {
        let manager = UpdateManager::new(true, std::path::Path::new("/tmp"));
        assert!(
            manager
                .trigger("not-a-version", "#!/bin/sh\n")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_empty_script() {
        let manager = UpdateManager::new(true, std::path::Path::new("/tmp"));
        assert!(manager.trigger("v1.2.3", "").await.is_err());
    }
}
