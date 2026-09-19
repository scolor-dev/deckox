use deckox_protocol::{StorageDisk, StorageMount, StoragePartition};
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tracing::warn;

use crate::error::AgentError;

/// Well-known system mount points, mirroring `is_standard_system` in
/// `services.rs`: a mount here is part of the base OS layout rather than an
/// application's own data volume (a Docker overlay mount under
/// `/var/lib/docker`, for example, does not match).
const STANDARD_MOUNT_POINTS: [&str; 6] = ["/", "/boot", "/boot/efi", "/home", "/var", "/tmp"];

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

/// Lists every disk with its partitions and stacked devices, joined with
/// `df` usage for whatever is mounted. `lsblk` failing (not installed, an
/// unsupported column on an old util-linux) yields an empty list rather than
/// an error, so the storage screen still shows the mount overview.
pub async fn read_disks() -> Result<Vec<StorageDisk>, AgentError> {
    if !cfg!(target_os = "linux") {
        return Err(AgentError::unavailable(
            "this endpoint requires a Linux host",
        ));
    }
    let mounts = read_storage().await.unwrap_or_default();

    let output = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("lsblk")
            .args([
                "-J",
                "-b",
                "-o",
                "NAME,KNAME,PATH,TYPE,SIZE,MODEL,ROTA,TRAN,RM,FSTYPE,LABEL",
            ])
            .output(),
    )
    .await;
    match output {
        Ok(Ok(output)) if output.status.success() => Ok(parse_lsblk(
            &String::from_utf8_lossy(&output.stdout),
            &mounts,
        )),
        Ok(Ok(output)) => {
            warn!(
                "lsblk failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            Ok(Vec::new())
        }
        Ok(Err(error)) => {
            warn!(%error, "failed to execute lsblk");
            Ok(Vec::new())
        }
        Err(_) => {
            warn!("lsblk timed out");
            Ok(Vec::new())
        }
    }
}

/// `lsblk -J` has changed value types across util-linux versions (sizes and
/// booleans arrive as numbers, strings or real booleans), so every field is
/// read through these tolerant helpers.
fn text(value: &Value, key: &str) -> Option<String> {
    let text = match value.get(key)? {
        Value::String(text) => text.trim().to_owned(),
        Value::Number(number) => number.to_string(),
        _ => return None,
    };
    (!text.is_empty()).then_some(text)
}

fn number(value: &Value, key: &str) -> u64 {
    match value.get(key) {
        Some(Value::Number(number)) => number.as_u64().unwrap_or(0),
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn flag(value: &Value, key: &str) -> Option<bool> {
    match value.get(key)? {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => match text.trim() {
            "1" | "true" => Some(true),
            "0" | "false" => Some(false),
            _ => None,
        },
        Value::Number(number) => Some(number.as_u64()? != 0),
        _ => None,
    }
}

fn collect_partitions(node: &Value, mounts: &[StorageMount], out: &mut Vec<StoragePartition>) {
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    for child in children {
        let path = text(child, "path")
            .or_else(|| text(child, "kname").map(|kname| format!("/dev/{kname}")))
            .unwrap_or_default();
        out.push(StoragePartition {
            name: text(child, "name").unwrap_or_default(),
            mount: mounts
                .iter()
                .find(|mount| mount.filesystem == path)
                .cloned(),
            path,
            kind: text(child, "type").unwrap_or_else(|| "part".to_owned()),
            size_bytes: number(child, "size"),
            filesystem_type: text(child, "fstype"),
            label: text(child, "label"),
        });
        collect_partitions(child, mounts, out);
    }
}

fn parse_lsblk(input: &str, mounts: &[StorageMount]) -> Vec<StorageDisk> {
    let Ok(root) = serde_json::from_str::<Value>(input) else {
        return Vec::new();
    };
    let Some(devices) = root.get("blockdevices").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut disks: Vec<StorageDisk> = devices
        .iter()
        .filter(|device| text(device, "type").as_deref() == Some("disk"))
        .filter_map(|device| {
            let name = text(device, "name")?;
            if name.starts_with("zram") || name.starts_with("ram") {
                return None;
            }
            let mut partitions = Vec::new();
            collect_partitions(device, mounts, &mut partitions);
            let path = text(device, "path").unwrap_or_else(|| format!("/dev/{name}"));
            Some(StorageDisk {
                name,
                path,
                model: text(device, "model"),
                size_bytes: number(device, "size"),
                transport: text(device, "tran"),
                rotational: flag(device, "rota"),
                removable: flag(device, "rm").unwrap_or(false),
                partitions,
            })
        })
        .collect();
    disks.sort_by(|a, b| a.name.cmp(&b.name));
    disks
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
            let mount_point = fields[6..].join(" ");
            let standard = STANDARD_MOUNT_POINTS.contains(&mount_point.as_str());

            Some(StorageMount {
                filesystem: fields[0].to_owned(),
                filesystem_type: fields[1].to_owned(),
                mount_point,
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                usage_percent,
                standard,
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
    use super::{parse_df, parse_lsblk};

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

    #[test]
    fn tags_only_well_known_system_mount_points_as_standard() {
        let mounts = parse_df(
            "Filesystem Type 1-blocks Used Available Capacity Mounted on\n\
             /dev/sda2 ext4 1000000 400000 600000 40% /\n\
             overlay overlay 1000000 400000 600000 40% /var/lib/docker\n",
        )
        .expect("valid df output");

        assert!(mounts[0].standard, "/ is a well-known system mount point");
        assert!(
            !mounts[1].standard,
            "an application data mount is not a system mount point"
        );
    }

    #[test]
    fn parses_lsblk_disks_with_stacked_devices_and_joins_df_usage() {
        let mounts = parse_df(
            "Filesystem Type 1-blocks Used Available Capacity Mounted on\n\
             /dev/sda1 ext4 1000000 400000 600000 40% /\n\
             /dev/mapper/vg-data xfs 5000000 1000000 4000000 20% /data\n",
        )
        .expect("valid df output");
        let disks = parse_lsblk(
            r#"{"blockdevices":[
                {"name":"sda","kname":"sda","path":"/dev/sda","type":"disk","size":500107862016,"model":"Samsung SSD 860 ","rota":false,"tran":"sata","rm":false,"fstype":null,"label":null,
                 "children":[
                   {"name":"sda1","kname":"sda1","path":"/dev/sda1","type":"part","size":100000000,"fstype":"ext4","label":"root"},
                   {"name":"sda2","kname":"sda2","path":"/dev/sda2","type":"part","size":400000000,"fstype":"LVM2_member",
                    "children":[{"name":"vg-data","kname":"dm-0","path":"/dev/mapper/vg-data","type":"lvm","size":400000000,"fstype":"xfs"}]}]},
                {"name":"sdb","kname":"sdb","path":"/dev/sdb","type":"disk","size":"2000398934016","model":null,"rota":"1","tran":"usb","rm":"1","children":[]},
                {"name":"loop0","kname":"loop0","path":"/dev/loop0","type":"loop","size":1000},
                {"name":"zram0","kname":"zram0","path":"/dev/zram0","type":"disk","size":1000}
            ]}"#,
            &mounts,
        );

        assert_eq!(disks.len(), 2, "loop and zram devices are not disks");
        assert_eq!(disks[0].name, "sda");
        assert_eq!(disks[0].model.as_deref(), Some("Samsung SSD 860"));
        assert_eq!(disks[0].rotational, Some(false));
        assert_eq!(disks[0].partitions.len(), 3);
        assert_eq!(
            disks[0].partitions[0]
                .mount
                .as_ref()
                .map(|mount| mount.mount_point.as_str()),
            Some("/")
        );
        assert!(
            disks[0].partitions[1].mount.is_none(),
            "LVM member is not mounted itself"
        );
        assert_eq!(disks[0].partitions[2].kind, "lvm");
        assert_eq!(
            disks[0].partitions[2]
                .mount
                .as_ref()
                .map(|mount| mount.mount_point.as_str()),
            Some("/data")
        );
        assert_eq!(disks[1].size_bytes, 2_000_398_934_016);
        assert_eq!(disks[1].rotational, Some(true));
        assert!(disks[1].removable);
        assert_eq!(disks[1].transport.as_deref(), Some("usb"));
    }

    #[test]
    fn tolerates_garbage_lsblk_output() {
        assert!(parse_lsblk("not json", &[]).is_empty());
        assert!(parse_lsblk("{}", &[]).is_empty());
    }
}
