use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use deckox_protocol::BackupSummary;

use crate::error::AgentError;

/// Matches `backup_root` in `packaging/scripts/install.sh`. Each entry is a
/// snapshot the installer took before replacing the running binaries.
const BACKUP_ROOT: &str = "/var/lib/deckox/backups";

/// Lists the installer's pre-update snapshots, newest first. An absent
/// backups directory (a host that has never updated) is an empty list, not
/// an error.
pub async fn list_backups() -> Result<Vec<BackupSummary>, AgentError> {
    if !cfg!(target_os = "linux") {
        return Err(AgentError::unavailable(
            "backup listing requires a Linux host",
        ));
    }

    let mut entries = match tokio::fs::read_dir(BACKUP_ROOT).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(AgentError::internal(format!(
                "failed to read backups directory: {error}"
            )));
        }
    };

    let mut backups = Vec::new();
    loop {
        let entry = entries.next_entry().await.map_err(|error| {
            AgentError::internal(format!("failed to read backups directory: {error}"))
        })?;
        let Some(entry) = entry else { break };

        if !entry.file_type().await.is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let created_at_ms = entry
            .metadata()
            .await
            .ok()
            .and_then(|metadata| metadata.created().or_else(|_| metadata.modified()).ok())
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        let size_bytes = directory_size(entry.path()).await;

        backups.push(BackupSummary {
            previous_version: previous_version_from_name(&name),
            name,
            created_at_ms,
            size_bytes,
        });
    }

    backups.sort_by_key(|backup| std::cmp::Reverse(backup.created_at_ms));
    Ok(backups)
}

/// Backup directories are named `{timestamp}-{previous_version}` (see
/// `install.sh`), where `previous_version` is `unknown` or digits and dots
/// only.
fn previous_version_from_name(name: &str) -> Option<String> {
    let (_, version) = name.split_once('-')?;
    (version != "unknown").then(|| version.to_owned())
}

async fn directory_size(path: PathBuf) -> u64 {
    tokio::task::spawn_blocking(move || directory_size_sync(&path))
        .await
        .unwrap_or(0)
}

fn directory_size_sync(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let Ok(metadata) = entry.metadata() else {
                return 0;
            };
            if metadata.is_dir() {
                directory_size_sync(&entry.path())
            } else {
                metadata.len()
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::previous_version_from_name;

    #[test]
    fn parses_previous_version_from_backup_directory_names() {
        assert_eq!(
            previous_version_from_name("20260910T120000Z-0.5.1"),
            Some("0.5.1".to_owned())
        );
        assert_eq!(previous_version_from_name("20260910T120000Z-unknown"), None);
        assert_eq!(previous_version_from_name("nohyphenatall"), None);
    }
}
