use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::{Duration, Instant},
};

use deckox_protocol::{
    CpuMetrics, DiskIoMetrics, LoadAverage, MemoryMetrics, NetworkMetrics, SystemInfo,
    SystemMetrics,
};
use tokio::process::Command;

use crate::error::AgentError;

pub async fn read_system_info() -> Result<SystemInfo, AgentError> {
    ensure_linux()?;

    let os_release = tokio::fs::read_to_string("/etc/os-release")
        .await
        .map_err(|error| {
            AgentError::internal(format!("failed to read /etc/os-release: {error}"))
        })?;
    let os_values = parse_key_values(&os_release);
    let hostname = read_trimmed("/etc/hostname").await?;
    let kernel_version = command_text("uname", &["-r"]).await?;
    let uptime_seconds = parse_uptime(&read_trimmed("/proc/uptime").await?)?;
    let boot_id = read_optional_trimmed("/proc/sys/kernel/random/boot_id").await;
    let timezone = read_timezone().await;

    Ok(SystemInfo {
        hostname,
        operating_system: os_values
            .get("NAME")
            .cloned()
            .unwrap_or_else(|| "Linux".to_owned()),
        os_version: os_values.get("VERSION_ID").cloned(),
        kernel_version,
        architecture: std::env::consts::ARCH.to_owned(),
        uptime_seconds,
        boot_id,
        timezone,
    })
}

pub async fn read_system_metrics() -> Result<SystemMetrics, AgentError> {
    ensure_linux()?;

    let network_interfaces = eligible_network_interfaces().await;
    let block_devices = eligible_block_devices().await;
    let first_cpu = read_cpu_sample().await?;
    let first_network = read_network_sample(network_interfaces.as_ref()).await;
    let first_disk = read_disk_sample(block_devices.as_ref()).await;
    let sample_started = Instant::now();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let second_cpu = read_cpu_sample().await?;
    let second_network = read_network_sample(network_interfaces.as_ref()).await;
    let second_disk = read_disk_sample(block_devices.as_ref()).await;
    let elapsed = sample_started.elapsed();
    let usage_percent = calculate_cpu_usage(first_cpu, second_cpu)?;
    let network = calculate_network_metrics(first_network, second_network, elapsed);
    let disk_io = calculate_disk_metrics(first_disk, second_disk, elapsed);
    let temperature_celsius = read_cpu_temperature().await;

    let memory_text = tokio::fs::read_to_string("/proc/meminfo")
        .await
        .map_err(|error| AgentError::internal(format!("failed to read /proc/meminfo: {error}")))?;
    let memory = parse_memory(&memory_text)?;
    let load_average = parse_load_average(&read_trimmed("/proc/loadavg").await?)?;

    Ok(SystemMetrics {
        cpu: CpuMetrics {
            logical_cores: std::thread::available_parallelism().map_or(1, usize::from),
            usage_percent,
            temperature_celsius,
        },
        memory,
        load_average,
        network,
        disk_io,
    })
}

fn ensure_linux() -> Result<(), AgentError> {
    if cfg!(target_os = "linux") {
        Ok(())
    } else {
        Err(AgentError::unavailable(
            "this endpoint requires a Linux host",
        ))
    }
}

async fn read_trimmed(path: impl AsRef<Path>) -> Result<String, AgentError> {
    let path = path.as_ref();
    tokio::fs::read_to_string(path)
        .await
        .map(|value| value.trim().to_owned())
        .map_err(|error| {
            AgentError::internal(format!("failed to read {}: {error}", path.display()))
        })
}

async fn read_optional_trimmed(path: impl AsRef<Path>) -> Option<String> {
    tokio::fs::read_to_string(path)
        .await
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn command_text(program: &str, args: &[&str]) -> Result<String, AgentError> {
    let output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new(program).args(args).output(),
    )
    .await
    .map_err(|_| AgentError::internal(format!("{program} timed out")))?
    .map_err(|error| AgentError::internal(format!("failed to execute {program}: {error}")))?;

    if !output.status.success() {
        return Err(AgentError::internal(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

async fn read_timezone() -> Option<String> {
    if let Some(timezone) = read_optional_trimmed("/etc/timezone").await {
        return Some(timezone);
    }

    let target = tokio::fs::read_link("/etc/localtime").await.ok()?;
    target
        .strip_prefix("/usr/share/zoneinfo")
        .ok()
        .map(|value| value.to_string_lossy().trim_start_matches('/').to_owned())
}

fn parse_key_values(input: &str) -> HashMap<String, String> {
    input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

pub fn parse_uptime(input: &str) -> Result<u64, AgentError> {
    let value = input
        .split_whitespace()
        .next()
        .ok_or_else(|| AgentError::internal("invalid /proc/uptime format"))?;
    let whole_seconds = value.split_once('.').map_or(value, |(whole, _)| whole);
    whole_seconds
        .parse()
        .map_err(|_| AgentError::internal("invalid /proc/uptime format"))
}

#[derive(Clone, Copy)]
struct CpuSample {
    idle: u64,
    total: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NetworkSample {
    received_bytes: u64,
    transmitted_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DiskSample {
    read_bytes: u64,
    written_bytes: u64,
}

async fn read_cpu_sample() -> Result<CpuSample, AgentError> {
    parse_cpu_sample(&read_trimmed("/proc/stat").await?)
}

fn parse_cpu_sample(input: &str) -> Result<CpuSample, AgentError> {
    let values = input
        .lines()
        .next()
        .ok_or_else(|| AgentError::internal("missing aggregate CPU line"))?
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AgentError::internal("invalid /proc/stat CPU values"))?;

    if values.len() < 5 {
        return Err(AgentError::internal("incomplete /proc/stat CPU values"));
    }

    Ok(CpuSample {
        idle: values[3].saturating_add(values[4]),
        total: values.iter().sum(),
    })
}

fn calculate_cpu_usage(first: CpuSample, second: CpuSample) -> Result<f64, AgentError> {
    let total = second.total.saturating_sub(first.total);
    let idle = second.idle.saturating_sub(first.idle);
    if total == 0 {
        return Err(AgentError::internal("CPU sample interval was empty"));
    }
    let active = u32::try_from(total.saturating_sub(idle))
        .map_err(|_| AgentError::internal("CPU active sample was too large"))?;
    let total =
        u32::try_from(total).map_err(|_| AgentError::internal("CPU sample was too large"))?;
    Ok(((f64::from(active) / f64::from(total)) * 100.0).clamp(0.0, 100.0))
}

async fn eligible_network_interfaces() -> Option<HashSet<String>> {
    let mut entries = tokio::fs::read_dir("/sys/class/net").await.ok()?;
    let mut eligible = HashSet::new();
    while let Some(entry) = entries.next_entry().await.ok()? {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name != "lo"
            && tokio::fs::metadata(entry.path().join("device"))
                .await
                .is_ok()
        {
            eligible.insert(name);
        }
    }
    (!eligible.is_empty()).then_some(eligible)
}

async fn eligible_block_devices() -> Option<HashSet<String>> {
    let mut entries = tokio::fs::read_dir("/sys/class/block").await.ok()?;
    let mut eligible = HashSet::new();
    while let Some(entry) = entries.next_entry().await.ok()? {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram") {
            continue;
        }
        if tokio::fs::metadata(entry.path().join("partition"))
            .await
            .is_ok()
        {
            continue;
        }

        let mut slaves = tokio::fs::read_dir(entry.path().join("slaves"))
            .await
            .ok()?;
        if slaves.next_entry().await.ok()?.is_none() {
            eligible.insert(name);
        }
    }
    (!eligible.is_empty()).then_some(eligible)
}

async fn read_network_sample(eligible: Option<&HashSet<String>>) -> Option<NetworkSample> {
    let eligible = eligible?;
    let input = tokio::fs::read_to_string("/proc/net/dev").await.ok()?;
    parse_network_sample(&input, eligible)
}

fn parse_network_sample(input: &str, eligible: &HashSet<String>) -> Option<NetworkSample> {
    let mut received_bytes = 0_u64;
    let mut transmitted_bytes = 0_u64;
    let mut matched = false;
    for line in input.lines() {
        let Some((name, counters)) = line.split_once(':') else {
            continue;
        };
        if !eligible.contains(name.trim()) {
            continue;
        }
        let values = counters.split_whitespace().collect::<Vec<_>>();
        let (Some(received), Some(transmitted)) = (values.first(), values.get(8)) else {
            continue;
        };
        let (Ok(received), Ok(transmitted)) = (received.parse::<u64>(), transmitted.parse::<u64>())
        else {
            continue;
        };
        received_bytes = received_bytes.saturating_add(received);
        transmitted_bytes = transmitted_bytes.saturating_add(transmitted);
        matched = true;
    }
    matched.then_some(NetworkSample {
        received_bytes,
        transmitted_bytes,
    })
}

async fn read_disk_sample(eligible: Option<&HashSet<String>>) -> Option<DiskSample> {
    let eligible = eligible?;
    let input = tokio::fs::read_to_string("/proc/diskstats").await.ok()?;
    parse_disk_sample(&input, eligible)
}

fn parse_disk_sample(input: &str, eligible: &HashSet<String>) -> Option<DiskSample> {
    const SECTOR_BYTES: u64 = 512;
    let mut read_bytes = 0_u64;
    let mut written_bytes = 0_u64;
    let mut matched = false;
    for line in input.lines() {
        let values = line.split_whitespace().collect::<Vec<_>>();
        let (Some(name), Some(read_sectors), Some(written_sectors)) =
            (values.get(2), values.get(5), values.get(9))
        else {
            continue;
        };
        if !eligible.contains(*name) {
            continue;
        }
        let (Ok(read_sectors), Ok(written_sectors)) =
            (read_sectors.parse::<u64>(), written_sectors.parse::<u64>())
        else {
            continue;
        };
        read_bytes = read_bytes.saturating_add(read_sectors.saturating_mul(SECTOR_BYTES));
        written_bytes = written_bytes.saturating_add(written_sectors.saturating_mul(SECTOR_BYTES));
        matched = true;
    }
    matched.then_some(DiskSample {
        read_bytes,
        written_bytes,
    })
}

fn bytes_per_second(first: u64, second: u64, elapsed: Duration) -> Option<u64> {
    let elapsed_nanos = elapsed.as_nanos();
    if elapsed_nanos == 0 || second < first {
        return None;
    }
    let rate = u128::from(second - first)
        .saturating_mul(1_000_000_000)
        .checked_div(elapsed_nanos)?;
    Some(u64::try_from(rate).unwrap_or(u64::MAX))
}

fn calculate_network_metrics(
    first: Option<NetworkSample>,
    second: Option<NetworkSample>,
    elapsed: Duration,
) -> Option<NetworkMetrics> {
    let (first, second) = (first?, second?);
    Some(NetworkMetrics {
        received_bytes_per_second: bytes_per_second(
            first.received_bytes,
            second.received_bytes,
            elapsed,
        )?,
        transmitted_bytes_per_second: bytes_per_second(
            first.transmitted_bytes,
            second.transmitted_bytes,
            elapsed,
        )?,
    })
}

fn calculate_disk_metrics(
    first: Option<DiskSample>,
    second: Option<DiskSample>,
    elapsed: Duration,
) -> Option<DiskIoMetrics> {
    let (first, second) = (first?, second?);
    Some(DiskIoMetrics {
        read_bytes_per_second: bytes_per_second(first.read_bytes, second.read_bytes, elapsed)?,
        written_bytes_per_second: bytes_per_second(
            first.written_bytes,
            second.written_bytes,
            elapsed,
        )?,
    })
}

async fn read_cpu_temperature() -> Option<f64> {
    let mut entries = tokio::fs::read_dir("/sys/class/thermal").await.ok()?;
    let mut temperatures = Vec::new();
    while let Some(entry) = entries.next_entry().await.ok()? {
        let zone_type = tokio::fs::read_to_string(entry.path().join("type"))
            .await
            .ok();
        let value = tokio::fs::read_to_string(entry.path().join("temp"))
            .await
            .ok();
        if let (Some(zone_type), Some(value)) = (zone_type, value) {
            if let Some(temperature) = parse_cpu_temperature(&zone_type, &value) {
                temperatures.push(temperature);
            }
        }
    }
    temperatures.into_iter().max_by(f64::total_cmp)
}

fn parse_cpu_temperature(zone_type: &str, value: &str) -> Option<f64> {
    let zone_type = zone_type.trim().to_ascii_lowercase();
    let is_cpu_zone =
        zone_type.contains("cpu") || zone_type.contains("soc") || zone_type.contains("x86_pkg");
    if !is_cpu_zone {
        return None;
    }
    let millidegrees = value.trim().parse::<i32>().ok()?;
    if !(-40_000..=150_000).contains(&millidegrees) {
        return None;
    }
    Some(f64::from(millidegrees) / 1_000.0)
}

fn parse_memory(input: &str) -> Result<MemoryMetrics, AgentError> {
    let values: HashMap<&str, u64> = input
        .lines()
        .filter_map(|line| {
            let (key, rest) = line.split_once(':')?;
            let kib = rest.split_whitespace().next()?.parse::<u64>().ok()?;
            Some((key, kib.saturating_mul(1024)))
        })
        .collect();

    let total = *values
        .get("MemTotal")
        .ok_or_else(|| AgentError::internal("MemTotal missing from /proc/meminfo"))?;
    let available = *values
        .get("MemAvailable")
        .ok_or_else(|| AgentError::internal("MemAvailable missing from /proc/meminfo"))?;
    let swap_total = values.get("SwapTotal").copied().unwrap_or(0);
    let swap_free = values.get("SwapFree").copied().unwrap_or(0);

    Ok(MemoryMetrics {
        total_bytes: total,
        used_bytes: total.saturating_sub(available),
        available_bytes: available,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_total.saturating_sub(swap_free),
    })
}

fn parse_load_average(input: &str) -> Result<LoadAverage, AgentError> {
    let values = input
        .split_whitespace()
        .take(3)
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AgentError::internal("invalid /proc/loadavg values"))?;

    if values.len() != 3 {
        return Err(AgentError::internal("incomplete /proc/loadavg values"));
    }

    Ok(LoadAverage {
        one_minute: values[0],
        five_minutes: values[1],
        fifteen_minutes: values[2],
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, time::Duration};

    use super::{
        CpuSample, DiskSample, NetworkSample, bytes_per_second, calculate_cpu_usage,
        calculate_disk_metrics, calculate_network_metrics, parse_cpu_sample, parse_cpu_temperature,
        parse_disk_sample, parse_key_values, parse_load_average, parse_memory,
        parse_network_sample, parse_uptime,
    };

    #[test]
    fn parses_os_release() {
        let values = parse_key_values("NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\n");
        assert_eq!(values.get("NAME").map(String::as_str), Some("Ubuntu"));
        assert_eq!(values.get("VERSION_ID").map(String::as_str), Some("24.04"));
    }

    #[test]
    fn parses_uptime_seconds() {
        assert_eq!(parse_uptime("1234.56 789.00").expect("valid uptime"), 1234);
    }

    #[test]
    fn parses_cpu_values_and_usage() {
        let sample = parse_cpu_sample("cpu  100 2 30 400 10 5 6 0\n").expect("valid CPU");
        assert_eq!(sample.total, 553);
        assert_eq!(sample.idle, 410);

        let usage = calculate_cpu_usage(
            CpuSample {
                idle: 100,
                total: 200,
            },
            CpuSample {
                idle: 140,
                total: 300,
            },
        )
        .expect("valid usage");
        assert!((usage - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_memory_values() {
        let memory = parse_memory(
            "MemTotal: 1000 kB\nMemAvailable: 400 kB\nSwapTotal: 200 kB\nSwapFree: 150 kB\n",
        )
        .expect("valid memory");
        assert_eq!(memory.used_bytes, 600 * 1024);
        assert_eq!(memory.swap_used_bytes, 50 * 1024);
    }

    #[test]
    fn parses_load_average_values() {
        let load = parse_load_average("0.10 0.20 0.30 1/100 123").expect("valid load");
        assert!((load.five_minutes - 0.20).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_only_eligible_network_interfaces() {
        let eligible = HashSet::from(["eth0".to_owned()]);
        let sample = parse_network_sample(
            "Inter-| Receive | Transmit\n\
             lo: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0\n\
             eth0: 1024 1 2 3 4 5 6 7 2048 9 10 11 12 13 14 15\n\
             veth1: 9000 0 0 0 0 0 0 0 8000 0 0 0 0 0 0 0\n",
            &eligible,
        )
        .expect("eligible interface");
        assert_eq!(
            sample,
            NetworkSample {
                received_bytes: 1024,
                transmitted_bytes: 2048,
            }
        );
    }

    #[test]
    fn ignores_malformed_network_counters() {
        let eligible = HashSet::from(["eth0".to_owned()]);
        assert!(parse_network_sample("eth0: invalid\n", &eligible).is_none());
    }

    #[test]
    fn parses_disk_sectors_as_bytes_for_eligible_devices() {
        let eligible = HashSet::from(["sda".to_owned()]);
        let sample = parse_disk_sample(
            "8 0 sda 10 0 20 0 30 0 40 0 0 0 0 0 0 0\n\
             8 1 sda1 10 0 200 0 30 0 400 0 0 0 0 0 0 0\n\
             7 0 loop0 10 0 500 0 30 0 600 0 0 0 0 0 0 0\n",
            &eligible,
        )
        .expect("eligible disk");
        assert_eq!(
            sample,
            DiskSample {
                read_bytes: 20 * 512,
                written_bytes: 40 * 512,
            }
        );
    }

    #[test]
    fn calculates_rates_and_rejects_invalid_counter_windows() {
        assert_eq!(
            bytes_per_second(1_000, 2_000, Duration::from_millis(250)),
            Some(4_000)
        );
        assert_eq!(bytes_per_second(2_000, 1_000, Duration::from_secs(1)), None);
        assert_eq!(bytes_per_second(0, 1, Duration::ZERO), None);

        let network = calculate_network_metrics(
            Some(NetworkSample {
                received_bytes: 100,
                transmitted_bytes: 300,
            }),
            Some(NetworkSample {
                received_bytes: 300,
                transmitted_bytes: 700,
            }),
            Duration::from_secs(2),
        )
        .expect("network rate");
        assert_eq!(network.received_bytes_per_second, 100);
        assert_eq!(network.transmitted_bytes_per_second, 200);

        let disk = calculate_disk_metrics(
            Some(DiskSample {
                read_bytes: 10,
                written_bytes: 20,
            }),
            Some(DiskSample {
                read_bytes: 30,
                written_bytes: 60,
            }),
            Duration::from_secs(2),
        )
        .expect("disk rate");
        assert_eq!(disk.read_bytes_per_second, 10);
        assert_eq!(disk.written_bytes_per_second, 20);
    }

    #[test]
    fn accepts_only_plausible_cpu_temperatures() {
        assert_eq!(
            parse_cpu_temperature("cpu-thermal\n", "52750\n"),
            Some(52.75)
        );
        assert_eq!(parse_cpu_temperature("x86_pkg_temp", "80000"), Some(80.0));
        assert_eq!(parse_cpu_temperature("battery", "35000"), None);
        assert_eq!(parse_cpu_temperature("cpu-thermal", "not-a-number"), None);
        assert_eq!(parse_cpu_temperature("cpu-thermal", "200000"), None);
    }
}
