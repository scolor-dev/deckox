use deckox_protocol::StorageMount;
use std::time::Duration;
use tokio::process::Command;

use crate::error::AgentError;

pub async fn read_storage() -> Result<Vec<StorageMount>, AgentError> {
    if !cfg!(target_os = "linux") {
        return Err(AgentError::unavailable(
            "this endpoint requires a Linux host",
        ));
    }

    let output = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("df").args(["-B1", "-P", "-T"]).output(),
    )
    .await
    .map_err(|_| AgentError::internal("df command timed out"))?
    .map_err(|error| AgentError::internal(format!("failed to execute df: {error}")))?;

    if !output.status.success() {
        return Err(AgentError::internal(format!(
            "df failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    parse_df(&String::from_utf8_lossy(&output.stdout))
}

fn parse_df(input: &str) -> Result<Vec<StorageMount>, AgentError> {
    let mounts = input
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 7 {
                return None;
            }

            let total = fields[2].parse::<u64>().ok()?;
            let used = fields[3].parse::<u64>().ok()?;
            let available = fields[4].parse::<u64>().ok()?;
            let usage_percent = fields[5].trim_end_matches('%').parse::<f64>().ok()?;

            Some(StorageMount {
                filesystem: fields[0].to_owned(),
                filesystem_type: fields[1].to_owned(),
                mount_point: fields[6..].join(" "),
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                usage_percent,
            })
        })
        .collect::<Vec<_>>();

    if mounts.is_empty() {
        return Err(AgentError::internal("df returned no parseable filesystems"));
    }

    Ok(mounts)
}

#[cfg(test)]
mod tests {
    use super::parse_df;

    #[test]
    fn parses_gnu_df_output() {
        let mounts = parse_df(
            "Filesystem Type 1-blocks Used Available Capacity Mounted on\n\
             /dev/sda2 ext4 1000000 400000 600000 40% /\n\
             /dev/sda1 vfat 1000 200 800 20% /boot/efi\n",
        )
        .expect("valid df output");

        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].filesystem_type, "ext4");
        assert!((mounts[0].usage_percent - 40.0).abs() < f64::EPSILON);
        assert_eq!(mounts[1].mount_point, "/boot/efi");
    }
}
