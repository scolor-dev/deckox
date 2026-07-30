use std::{collections::HashMap, path::Path, time::Duration};

use deckox_protocol::{CpuMetrics, LoadAverage, MemoryMetrics, SystemInfo, SystemMetrics};
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

    let first_cpu = read_cpu_sample().await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let second_cpu = read_cpu_sample().await?;
    let usage_percent = calculate_cpu_usage(first_cpu, second_cpu)?;

    let memory_text = tokio::fs::read_to_string("/proc/meminfo")
        .await
        .map_err(|error| AgentError::internal(format!("failed to read /proc/meminfo: {error}")))?;
    let memory = parse_memory(&memory_text)?;
    let load_average = parse_load_average(&read_trimmed("/proc/loadavg").await?)?;

    Ok(SystemMetrics {
        cpu: CpuMetrics {
            logical_cores: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
            usage_percent,
        },
        memory,
        load_average,
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

fn parse_uptime(input: &str) -> Result<u64, AgentError> {
    input
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value as u64)
        .ok_or_else(|| AgentError::internal("invalid /proc/uptime format"))
}

#[derive(Clone, Copy)]
struct CpuSample {
    idle: u64,
    total: u64,
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
    Ok((((total.saturating_sub(idle)) as f64 / total as f64) * 100.0).clamp(0.0, 100.0))
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
    use super::{
        CpuSample, calculate_cpu_usage, parse_cpu_sample, parse_key_values, parse_load_average,
        parse_memory, parse_uptime,
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
        assert_eq!(load.five_minutes, 0.20);
    }
}
