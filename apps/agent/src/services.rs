use std::{
    collections::{HashMap, HashSet},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{
    CommandResult, CommandStatus, ServiceAction, ServiceDetails, ServiceLogEntry,
    ServiceLogPriority, ServiceLogs, ServiceSummary,
};
use serde_json::Value;
use tokio::{io::AsyncReadExt, process::Command};

use crate::error::AgentError;

static COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const PROTECTED_SERVICES: [&str; 2] = ["deckox-agent.service", "deckox-server.service"];
const ALLOWED_LOG_LINES: [u16; 4] = [50, 100, 200, 500];
const MAX_JOURNAL_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_LOG_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_LOG_MESSAGE_BYTES: usize = 8 * 1024;

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

        self.ensure_allowed(service_id)?;

        let action_name = match action {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
            ServiceAction::Enable => "enable",
            ServiceAction::Disable => "disable",
        };
        systemctl(&[action_name, service_id]).await?;

        Ok(CommandResult {
            command_id: command_id(),
            status: CommandStatus::Completed,
            message: Some(format!("{action_name} completed for {service_id}")),
        })
    }

    pub async fn logs(
        &self,
        service_id: &str,
        lines: u16,
        priority: ServiceLogPriority,
    ) -> Result<ServiceLogs, AgentError> {
        ensure_linux()?;
        validate_service_id(service_id)?;
        self.ensure_allowed(service_id)?;
        validate_log_lines(lines)?;

        let lines = lines.to_string();
        let mut args = vec![
            "--unit",
            service_id,
            "--no-pager",
            "--output=json",
            "--output-fields=__REALTIME_TIMESTAMP,PRIORITY,MESSAGE,SYSLOG_IDENTIFIER,_COMM,_PID",
            "--lines",
            &lines,
        ];
        if let Some(priority) = journal_priority(priority) {
            args.extend(["--priority", priority]);
        }

        let output = journalctl(&args).await?;
        Ok(ServiceLogs {
            service_id: service_id.to_owned(),
            entries: parse_journal_entries(&output),
        })
    }

    fn ensure_allowed(&self, service_id: &str) -> Result<(), AgentError> {
        if PROTECTED_SERVICES.contains(&service_id) {
            return Err(AgentError::forbidden(
                "Deckox services cannot be managed through the Agent",
            ));
        }
        if !self.allowed.contains(service_id) {
            return Err(AgentError::forbidden(format!(
                "service is not in the control allowlist: {service_id}"
            )));
        }
        Ok(())
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
    command_text("systemctl", args, 30, None).await
}

async fn journalctl(args: &[&str]) -> Result<String, AgentError> {
    let mut child = Command::new("journalctl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| AgentError::internal(format!("failed to execute journalctl: {error}")))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| AgentError::internal("failed to capture journalctl output"))?
        .take(u64::try_from(MAX_JOURNAL_OUTPUT_BYTES).unwrap_or(u64::MAX) + 1);
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| AgentError::internal("failed to capture journalctl errors"))?
        .take(64 * 1024);

    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let ((), (), status) = tokio::try_join!(
            async { stdout.read_to_end(&mut output).await.map(|_| ()) },
            async { stderr.read_to_end(&mut errors).await.map(|_| ()) },
            child.wait(),
        )?;
        Ok::<_, std::io::Error>((output, errors, status))
    })
    .await
    .map_err(|_| AgentError::internal("journalctl command timed out"))?
    .map_err(|error| AgentError::internal(format!("failed to read journalctl output: {error}")))?;
    let (output, errors, status) = result;

    if output.len() > MAX_JOURNAL_OUTPUT_BYTES {
        return Err(AgentError::internal(
            "journalctl output exceeded the safety limit",
        ));
    }
    if !status.success() {
        let message = String::from_utf8_lossy(&errors).trim().to_owned();
        return Err(AgentError::internal(if message.is_empty() {
            "journalctl command failed".to_owned()
        } else {
            message
        }));
    }
    Ok(String::from_utf8_lossy(&output).into_owned())
}

async fn command_text(
    program: &str,
    args: &[&str],
    timeout_seconds: u64,
    maximum_output_bytes: Option<usize>,
) -> Result<String, AgentError> {
    let output = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        Command::new(program).args(args).output(),
    )
    .await
    .map_err(|_| AgentError::internal(format!("{program} command timed out")))?
    .map_err(|error| AgentError::internal(format!("failed to execute {program}: {error}")))?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AgentError::internal(if message.is_empty() {
            format!("{program} command failed")
        } else {
            message
        }));
    }

    if maximum_output_bytes.is_some_and(|maximum| output.stdout.len() > maximum) {
        return Err(AgentError::internal(format!(
            "{program} output exceeded the safety limit"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn validate_log_lines(lines: u16) -> Result<(), AgentError> {
    if ALLOWED_LOG_LINES.contains(&lines) {
        Ok(())
    } else {
        Err(AgentError::bad_request(
            "log lines must be one of 50, 100, 200, or 500",
        ))
    }
}

const fn journal_priority(priority: ServiceLogPriority) -> Option<&'static str> {
    match priority {
        ServiceLogPriority::All => None,
        ServiceLogPriority::Error => Some("err"),
        ServiceLogPriority::Warning => Some("warning"),
        ServiceLogPriority::Info => Some("info"),
    }
}

fn parse_journal_entries(input: &str) -> Vec<ServiceLogEntry> {
    let mut response_bytes: usize = 0;
    input
        .lines()
        .filter_map(|line| parse_journal_entry(line).ok())
        .take_while(|entry| {
            response_bytes = response_bytes.saturating_add(entry.message.len());
            response_bytes <= MAX_LOG_RESPONSE_BYTES
        })
        .collect()
}

fn parse_journal_entry(line: &str) -> Result<ServiceLogEntry, serde_json::Error> {
    let value: Value = serde_json::from_str(line)?;
    let timestamp_ms = string_field(&value, "__REALTIME_TIMESTAMP")
        .and_then(|timestamp| timestamp.parse::<u64>().ok())
        .map_or(0, |timestamp| timestamp / 1_000);
    let priority = string_field(&value, "PRIORITY")
        .and_then(|priority| priority.parse::<u8>().ok())
        .filter(|priority| *priority <= 7)
        .unwrap_or(6);
    let message = value
        .get("MESSAGE")
        .and_then(Value::as_str)
        .map_or_else(|| "[binary journal message]".to_owned(), truncate_message);
    let process = string_field(&value, "SYSLOG_IDENTIFIER")
        .or_else(|| string_field(&value, "_COMM"))
        .map(str::to_owned);
    let pid = string_field(&value, "_PID").and_then(|pid| pid.parse::<u32>().ok());

    Ok(ServiceLogEntry {
        timestamp_ms,
        priority,
        message,
        process,
        pid,
    })
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn truncate_message(message: &str) -> String {
    if message.len() <= MAX_LOG_MESSAGE_BYTES {
        return message.to_owned();
    }
    let mut end = MAX_LOG_MESSAGE_BYTES.saturating_sub('…'.len_utf8());
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &message[..end])
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

    use deckox_protocol::ServiceLogPriority;

    use super::{
        MAX_LOG_MESSAGE_BYTES, PROTECTED_SERVICES, ServiceManager, journal_priority,
        parse_journal_entries, parse_properties, parse_service_list, truncate_message,
        validate_log_lines,
    };

    #[test]
    fn validates_allowlist() {
        assert!(ServiceManager::new(vec!["nginx.service".to_owned()]).is_ok());
        assert!(ServiceManager::new(vec!["nginx.service;reboot".to_owned()]).is_err());
        assert!(ServiceManager::new(vec!["-nginx.service".to_owned()]).is_err());
        for service in PROTECTED_SERVICES {
            assert!(ServiceManager::new(vec![service.to_owned()]).is_err());
        }
    }

    #[test]
    fn enforces_allowlist_by_exact_service_id() {
        let manager =
            ServiceManager::new(vec!["nginx.service".to_owned()]).expect("allowlist is valid");
        assert!(manager.ensure_allowed("nginx.service").is_ok());
        assert!(manager.ensure_allowed("nginx-extra.service").is_err());
        assert!(manager.ensure_allowed("deckox-agent.service").is_err());
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

    #[test]
    fn accepts_only_bounded_log_line_counts() {
        for lines in [50, 100, 200, 500] {
            assert!(validate_log_lines(lines).is_ok());
        }
        for lines in [0, 49, 101, 501] {
            assert!(validate_log_lines(lines).is_err());
        }
    }

    #[test]
    fn maps_log_priorities_to_fixed_journal_values() {
        assert_eq!(journal_priority(ServiceLogPriority::All), None);
        assert_eq!(journal_priority(ServiceLogPriority::Error), Some("err"));
        assert_eq!(
            journal_priority(ServiceLogPriority::Warning),
            Some("warning")
        );
        assert_eq!(journal_priority(ServiceLogPriority::Info), Some("info"));
    }

    #[test]
    fn parses_journal_json_lines_and_ignores_invalid_rows() {
        let entries = parse_journal_entries(
            "{\"__REALTIME_TIMESTAMP\":\"1700000000123456\",\"PRIORITY\":\"3\",\"MESSAGE\":\"failed\",\"SYSLOG_IDENTIFIER\":\"nginx\",\"_PID\":\"42\"}\nnot-json\n{\"MESSAGE\":[1,2]}\n",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp_ms, 1_700_000_000_123);
        assert_eq!(entries[0].priority, 3);
        assert_eq!(entries[0].message, "failed");
        assert_eq!(entries[0].process.as_deref(), Some("nginx"));
        assert_eq!(entries[0].pid, Some(42));
        assert_eq!(entries[1].message, "[binary journal message]");
    }

    #[test]
    fn truncates_log_messages_on_a_utf8_boundary() {
        let message = "あ".repeat(MAX_LOG_MESSAGE_BYTES);
        let truncated = truncate_message(&message);
        assert!(truncated.len() <= MAX_LOG_MESSAGE_BYTES);
        assert!(truncated.ends_with('…'));
    }
}
