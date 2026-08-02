use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use deckox_protocol::{AgentDiagnostics, DiagnosticAgent, DiagnosticServer, DiagnosticsReport};

use crate::{agent_client::AgentClient, request_context::RequestId};

const REPORT_FILENAME: &str = "attachment; filename=\"deckox-diagnostics.json\"";

pub async fn collect(agent: &AgentClient, request_id: &RequestId) -> DiagnosticsReport {
    let generated_at_ms = current_timestamp_ms();
    match agent.request("GET", "/v1/diagnostics", request_id).await {
        Ok(response) if response.status.is_success() => {
            serde_json::from_value::<AgentDiagnostics>(response.body).map_or_else(
                |_| failure_report(generated_at_ms, true, "invalid_agent_response"),
                |agent_diagnostics| available_report(generated_at_ms, agent_diagnostics),
            )
        }
        Ok(_) => failure_report(generated_at_ms, true, "diagnostics_unavailable"),
        Err(_) => failure_report(generated_at_ms, false, "agent_unavailable"),
    }
}

pub fn attachment(report: &DiagnosticsReport) -> Response {
    serde_json::to_vec_pretty(report).map_or_else(
        |_| StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        |body| {
            (
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (header::CONTENT_DISPOSITION, REPORT_FILENAME),
                ],
                body,
            )
                .into_response()
        },
    )
}

fn available_report(
    generated_at_ms: u64,
    agent_diagnostics: AgentDiagnostics,
) -> DiagnosticsReport {
    DiagnosticsReport {
        generated_at_ms,
        server: server_diagnostics("running"),
        agent: DiagnosticAgent {
            connected: true,
            version: Some(agent_diagnostics.version),
            error_code: None,
        },
        host: Some(agent_diagnostics.host),
        deckox_services: Some(agent_diagnostics.deckox_services),
        runtime_config: Some(agent_diagnostics.runtime_config),
    }
}

fn failure_report(generated_at_ms: u64, connected: bool, error_code: &str) -> DiagnosticsReport {
    DiagnosticsReport {
        generated_at_ms,
        server: server_diagnostics("degraded"),
        agent: DiagnosticAgent {
            connected,
            version: None,
            error_code: Some(error_code.to_owned()),
        },
        host: None,
        deckox_services: None,
        runtime_config: None,
    }
}

fn server_diagnostics(status: &str) -> DiagnosticServer {
    DiagnosticServer {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        status: status.to_owned(),
    }
}

fn current_timestamp_ms() -> u64 {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use axum::{Json, http::StatusCode, response::IntoResponse};
    use deckox_protocol::{
        AgentDiagnostics, DeckoxServiceDiagnostics, DiagnosticHost, DiagnosticUnitState,
        RuntimeConfigSummary,
    };
    use serde_json::Value;

    use super::{REPORT_FILENAME, attachment, available_report, failure_report};

    fn agent_diagnostics() -> AgentDiagnostics {
        AgentDiagnostics {
            version: "0.3.7".to_owned(),
            host: DiagnosticHost {
                hostname: "deckox-host".to_owned(),
                operating_system: "Linux".to_owned(),
                os_version: Some("1".to_owned()),
                kernel_version: "6.1".to_owned(),
                architecture: "aarch64".to_owned(),
                uptime_seconds: 42,
                timezone: Some("Asia/Tokyo".to_owned()),
            },
            deckox_services: DeckoxServiceDiagnostics {
                agent: unit_state(),
                server: unit_state(),
            },
            runtime_config: RuntimeConfigSummary {
                reboot_allowed: false,
                allowed_services_count: 2,
                ssh_management_enabled: true,
            },
        }
    }

    fn unit_state() -> DiagnosticUnitState {
        DiagnosticUnitState {
            load_state: "loaded".to_owned(),
            active_state: "active".to_owned(),
            sub_state: "running".to_owned(),
            unit_file_state: Some("enabled".to_owned()),
        }
    }

    #[test]
    fn report_json_contains_no_secret_or_path_keys() {
        let report = available_report(1, agent_diagnostics());
        let value = serde_json::to_value(report).expect("diagnostics should serialize");
        let mut keys = Vec::new();
        collect_keys(&value, &mut keys);

        for forbidden in [
            "boot_id",
            "raw_config",
            "env",
            "password",
            "hash",
            "password_hash",
            "session",
            "sessions",
            "logs",
            "ssh_key",
            "ssh_keys",
            "managed_user",
            "managed_username",
            "socket",
            "socket_path",
            "socket_error",
            "config_path",
            "environment",
            "listen_address",
        ] {
            assert!(!keys.contains(&forbidden), "forbidden key: {forbidden}");
        }
    }

    #[test]
    fn unavailable_agent_keeps_safe_server_report() {
        let report = failure_report(123, false, "agent_unavailable");

        assert_eq!(report.generated_at_ms, 123);
        assert_eq!(report.server.status, "degraded");
        assert!(!report.agent.connected);
        assert_eq!(
            report.agent.error_code.as_deref(),
            Some("agent_unavailable")
        );
        assert!(report.host.is_none());
        assert_eq!(Json(report).into_response().status(), StatusCode::OK);
    }

    #[test]
    fn attachment_uses_a_constant_safe_filename() {
        let report = failure_report(1, false, "agent_unavailable");
        let response = attachment(&report);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        assert_eq!(
            response
                .headers()
                .get("content-disposition")
                .and_then(|value| value.to_str().ok()),
            Some(REPORT_FILENAME)
        );
    }

    fn collect_keys<'a>(value: &'a Value, keys: &mut Vec<&'a str>) {
        match value {
            Value::Object(object) => {
                for (key, nested) in object {
                    keys.push(key);
                    collect_keys(nested, keys);
                }
            }
            Value::Array(values) => {
                for nested in values {
                    collect_keys(nested, keys);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
}
