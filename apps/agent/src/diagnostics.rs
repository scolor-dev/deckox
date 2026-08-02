use std::{collections::HashMap, time::Duration};

use deckox_protocol::{
    AgentDiagnostics, DeckoxServiceDiagnostics, DiagnosticHost, DiagnosticUnitState,
    RuntimeConfigSummary,
};
use tokio::process::Command;
use tracing::warn;

use crate::{error::AgentError, system::read_system_info};

const SYSTEMCTL_PROPERTIES: &str = "LoadState,ActiveState,SubState,UnitFileState";

#[derive(Clone, Copy)]
enum DeckoxUnit {
    Agent,
    Server,
}

impl DeckoxUnit {
    const fn id(self) -> &'static str {
        match self {
            Self::Agent => "deckox-agent.service",
            Self::Server => "deckox-server.service",
        }
    }
}

pub async fn read_diagnostics(
    runtime_config: RuntimeConfigSummary,
) -> Result<AgentDiagnostics, AgentError> {
    let system = read_system_info().await?;
    let (agent, server) = tokio::join!(
        read_unit_state(DeckoxUnit::Agent),
        read_unit_state(DeckoxUnit::Server)
    );

    Ok(AgentDiagnostics {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        host: DiagnosticHost {
            hostname: system.hostname,
            operating_system: system.operating_system,
            os_version: system.os_version,
            kernel_version: system.kernel_version,
            architecture: system.architecture,
            uptime_seconds: system.uptime_seconds,
            timezone: system.timezone,
        },
        deckox_services: DeckoxServiceDiagnostics { agent, server },
        runtime_config,
    })
}

async fn read_unit_state(unit: DeckoxUnit) -> DiagnosticUnitState {
    let unit_id = unit.id();
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("systemctl")
            .args([
                "show",
                unit_id,
                "--no-pager",
                "--property",
                SYSTEMCTL_PROPERTIES,
            ])
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) if output.status.success() => {
            parse_unit_state(&String::from_utf8_lossy(&output.stdout))
        }
        Ok(Ok(_)) => {
            warn!(unit = unit_id, "diagnostic systemctl command failed");
            unavailable_unit_state()
        }
        Ok(Err(error)) => {
            warn!(unit = unit_id, %error, "failed to execute diagnostic systemctl command");
            unavailable_unit_state()
        }
        Err(_) => {
            warn!(unit = unit_id, "diagnostic systemctl command timed out");
            unavailable_unit_state()
        }
    }
}

fn parse_unit_state(input: &str) -> DiagnosticUnitState {
    let values: HashMap<&str, &str> = input
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();

    DiagnosticUnitState {
        load_state: value_or_unknown(&values, "LoadState"),
        active_state: value_or_unknown(&values, "ActiveState"),
        sub_state: value_or_unknown(&values, "SubState"),
        unit_file_state: values
            .get("UnitFileState")
            .copied()
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
    }
}

fn value_or_unknown(values: &HashMap<&str, &str>, key: &str) -> String {
    values
        .get(key)
        .copied()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}

fn unavailable_unit_state() -> DiagnosticUnitState {
    DiagnosticUnitState {
        load_state: "unavailable".to_owned(),
        active_state: "unavailable".to_owned(),
        sub_state: "unavailable".to_owned(),
        unit_file_state: None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_unit_state;

    #[test]
    fn parses_only_fixed_unit_state_properties() {
        let state = parse_unit_state(
            "LoadState=loaded\nActiveState=active\nSubState=running\nUnitFileState=enabled\nDescription=secret\n",
        );

        assert_eq!(state.load_state, "loaded");
        assert_eq!(state.active_state, "active");
        assert_eq!(state.sub_state, "running");
        assert_eq!(state.unit_file_state.as_deref(), Some("enabled"));
    }

    #[test]
    fn missing_properties_do_not_expose_command_output() {
        let state = parse_unit_state("PasswordHash=not-for-diagnostics\n");

        assert_eq!(state.load_state, "unknown");
        assert_eq!(state.active_state, "unknown");
        assert_eq!(state.sub_state, "unknown");
        assert!(state.unit_file_state.is_none());
    }
}
