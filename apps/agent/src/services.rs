use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{
    CommandResult, CommandStatus, ServiceAction, ServiceDetails, ServiceSummary,
};
use tokio::process::Command;

use crate::error::AgentError;

static COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const PROTECTED_SERVICES: [&str; 2] = ["deckox-agent.service", "deckox-server.service"];

#[derive(Clone)]
pub struct ServiceManager {
    allowed: HashSet<String>,
}

impl ServiceManager {
    pub fn new(allowed: Vec<String>) -> Result<Self, AgentError> {
        let mut validated = HashSet::new();
        for service in allowed {
            validate_service_id(&service)?;
            if PROTECTED_SERVICES.contains(&service.as_str()) {
                return Err(AgentError::bad_request(format!(
                    "{service} cannot be added to the control allowlist"
                )));
            }
            validated.insert(service);
        }
        Ok(Self { allowed: validated })
    }

    pub async fn list(&self) -> Result<Vec<ServiceSummary>, AgentError> {
        ensure_linux()?;
        let enabled_states = read_unit_file_states().await?;
        let output = systemctl(&[
            "list-units",
            "--type=service",
            "--all",
            "--no-legend",
            "--no-pager",
            "--plain",
        ])
        .await?;

        Ok(parse_service_list(&output, &enabled_states, &self.allowed))
    }

    pub async fn details(&self, service_id: &str) -> Result<ServiceDetails, AgentError> {
        ensure_linux()?;
        validate_service_id(service_id)?;

        let output = systemctl(&[
            "show",
            service_id,
            "--no-pager",
            "--property=Id,Description,LoadState,ActiveState,SubState,UnitFileState,MainPID",
        ])
        .await?;
        let values = parse_properties(&output);

        if values
            .get("LoadState")
            .is_none_or(|state| state == "not-found")
        {
            return Err(AgentError::not_found(format!(
                "service not found: {service_id}"
            )));
        }

        Ok(ServiceDetails {
            id: values
                .get("Id")
                .cloned()
                .unwrap_or_else(|| service_id.to_owned()),
            description: values.get("Description").cloned().unwrap_or_default(),
            load_state: values.get("LoadState").cloned().unwrap_or_default(),
            active_state: values.get("ActiveState").cloned().unwrap_or_default(),
            sub_state: values.get("SubState").cloned().unwrap_or_default(),
            unit_file_state: values
                .get("UnitFileState")
                .cloned()
                .filter(|v| !v.is_empty()),
            main_pid: values
                .get("MainPID")
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|pid| *pid != 0),
            control_allowed: self.allowed.contains(service_id),
        })
    }

    pub async fn control(
        &self,
        service_id: &str,
        action: ServiceAction,
    ) -> Result<CommandResult, AgentError> {
        ensure_linux()?;
        validate_service_id(service_id)?;

        if PROTECTED_SERVICES.contains(&service_id) {
            return Err(AgentError::forbidden(
                "Deckox services cannot be controlled through the Agent",
            ));
        }
        if !self.allowed.contains(service_id) {
            return Err(AgentError::forbidden(format!(
                "service is not in the control allowlist: {service_id}"
            )));
        }

        let action_name = match action {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
        };
        systemctl(&[action_name, service_id]).await?;

        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{action_name} completed for {service_id}")),
        })
    }
}

fn ensure_linux() -> Result<(), AgentError> {
    if cfg!(target_os = "linux") {
        Ok(())
    } else {
        Err(AgentError::unavailable(
            "systemd management requires a Linux host",
        ))
    }
}

fn validate_service_id(service_id: &str) -> Result<(), AgentError> {
    let valid = !service_id.is_empty()
        && service_id.len() <= 256
        && service_id.ends_with(".service")
        && service_id
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && service_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"@_.:-".contains(&byte));

    if valid {
        Ok(())
    } else {
        Err(AgentError::bad_request("invalid systemd service id"))
    }
}

async fn systemctl(args: &[&str]) -> Result<String, AgentError> {
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        Command::new("systemctl").args(args).output(),
    )
    .await
    .map_err(|_| AgentError::internal("systemctl command timed out"))?
    .map_err(|error| AgentError::internal(format!("failed to execute systemctl: {error}")))?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AgentError::internal(if message.is_empty() {
            "systemctl command failed".to_owned()
        } else {
            message
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn read_unit_file_states() -> Result<HashMap<String, String>, AgentError> {
    let output = systemctl(&[
        "list-unit-files",
        "--type=service",
        "--no-legend",
        "--no-pager",
    ])
    .await?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.to_owned(), fields.next()?.to_owned()))
        })
        .collect())
}

fn parse_service_list(
    input: &str,
    unit_file_states: &HashMap<String, String>,
    allowed: &HashSet<String>,
) -> Vec<ServiceSummary> {
    input
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let id = fields.next()?.to_owned();
            let load_state = fields.next()?.to_owned();
            let active_state = fields.next()?.to_owned();
            let sub_state = fields.next()?.to_owned();
            let description = fields.collect::<Vec<_>>().join(" ");
            Some(ServiceSummary {
                control_allowed: allowed.contains(&id),
                unit_file_state: unit_file_states.get(&id).cloned(),
                id,
                description,
                load_state,
                active_state,
                sub_state,
            })
        })
        .collect()
}

fn parse_properties(input: &str) -> HashMap<String, String> {
    input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn command_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = COMMAND_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("cmd-{timestamp}-{sequence}")
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::{PROTECTED_SERVICES, ServiceManager, parse_properties, parse_service_list};

    #[test]
    fn validates_allowlist() {
        assert!(ServiceManager::new(vec!["nginx.service".to_owned()]).is_ok());
        assert!(ServiceManager::new(vec!["nginx.service;reboot".to_owned()]).is_err());
        assert!(ServiceManager::new(vec!["-nginx.service".to_owned()]).is_err());
        assert!(ServiceManager::new(vec![PROTECTED_SERVICES[0].to_owned()]).is_err());
    }

    #[test]
    fn parses_service_rows() {
        let mut states = HashMap::new();
        states.insert("nginx.service".to_owned(), "enabled".to_owned());
        let allowed = HashSet::from(["nginx.service".to_owned()]);
        let services = parse_service_list(
            "nginx.service loaded active running A high performance web server\n",
            &states,
            &allowed,
        );

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].description, "A high performance web server");
        assert_eq!(services[0].unit_file_state.as_deref(), Some("enabled"));
        assert!(services[0].control_allowed);
    }

    #[test]
    fn parses_systemd_properties() {
        let properties = parse_properties(
            "Id=nginx.service\nDescription=Nginx\nActiveState=active\nMainPID=123\n",
        );
        assert_eq!(
            properties.get("ActiveState").map(String::as_str),
            Some("active")
        );
    }
}
