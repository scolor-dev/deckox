use std::{
    env,
    net::IpAddr,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use deckox_protocol::{AuditEvent, AuditPage};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::{fsutil::atomic_write_secure, request_context::RequestId};

const DEFAULT_AUDIT_LOG_FILE: &str = "/var/lib/deckox/audit.log";
const MAX_AUDIT_ENTRIES: usize = 5000;
const DEFAULT_PAGE_LIMIT: usize = 100;
const MAX_PAGE_LIMIT: usize = 500;
const REPORT_FILENAME: &str = "attachment; filename=\"deckox-audit.json\"";

/// Append-only JSON Lines log of administrative actions (login, password
/// change, service control, reboot, TOTP changes, CLI recovery commands).
/// Complements journald: `tracing` still emits the same events for ops
/// tooling, while this file backs the in-app audit viewer.
#[derive(Clone)]
pub struct AuditLog {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl AuditLog {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn from_env() -> Self {
        Self::new(PathBuf::from(
            env::var("DECKOX_AUDIT_LOG_FILE").unwrap_or_else(|_| DEFAULT_AUDIT_LOG_FILE.to_owned()),
        ))
    }

    /// Records one event. Failures are logged but never surfaced to the
    /// caller — a broken audit log must not block the administrative action
    /// it is trying to record.
    pub async fn record(
        &self,
        event: &str,
        actor: &str,
        source_ip: IpAddr,
        result: &str,
        detail: Option<String>,
    ) {
        self.record_raw(event, actor, source_ip.to_string(), result, detail)
            .await;
    }

    /// Same as [`record`](Self::record), but for actions taken from the
    /// `reset-password` / `disable-totp` CLI subcommands, which run outside
    /// the server process and have no network peer to attribute.
    pub async fn record_cli(&self, event: &str, result: &str, detail: Option<String>) {
        self.record_raw(event, "admin", "cli".to_owned(), result, detail)
            .await;
    }

    async fn record_raw(
        &self,
        event: &str,
        actor: &str,
        source_ip: String,
        result: &str,
        detail: Option<String>,
    ) {
        let entry = AuditEvent {
            timestamp_ms: now_ms(),
            event: event.to_owned(),
            actor: actor.to_owned(),
            source_ip,
            result: result.to_owned(),
            detail,
        };
        if let Err(error) = self.append(entry).await {
            warn!(%error, "failed to persist audit log entry");
        }
    }

    /// Convenience for the common case: the actor is always the single
    /// `"admin"` account today.
    pub async fn record_admin(
        &self,
        event: &str,
        source_ip: IpAddr,
        result: &str,
        detail: Option<String>,
    ) {
        self.record(event, "admin", source_ip, result, detail).await;
    }

    /// Pairs a journald line with an [`record_admin`](Self::record_admin)
    /// call so both destinations record the same `event`/`result`/`detail`
    /// instead of drifting apart. `result` selects the log level:
    /// `"success"`, `"accepted"`, and `"totp_required"` log at `info`,
    /// everything else (failures, rejections, rate limiting) at `warn`.
    /// `message` is the human-readable sentence shown in `journalctl`.
    pub async fn log_admin(
        &self,
        request_id: &RequestId,
        source_ip: IpAddr,
        event: &'static str,
        result: &'static str,
        detail: Option<String>,
        message: &'static str,
    ) {
        if matches!(result, "success" | "accepted" | "totp_required") {
            info!(
                event,
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %source_ip,
                result,
                detail = ?detail,
                "{message}"
            );
        } else {
            warn!(
                event,
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %source_ip,
                result,
                detail = ?detail,
                "{message}"
            );
        }
        self.record_admin(event, source_ip, result, detail).await;
    }

    async fn append(&self, entry: AuditEvent) -> Result<(), String> {
        let _guard = self.write_lock.lock().await;
        let mut entries = self.read_all().await?;
        entries.push(entry);
        if entries.len() > MAX_AUDIT_ENTRIES {
            let overflow = entries.len() - MAX_AUDIT_ENTRIES;
            entries.drain(..overflow);
        }
        self.write_all(&entries).await
    }

    /// Returns up to `limit` events, newest first. `before_ms` restricts the
    /// page to events strictly older than the given timestamp (pagination).
    pub async fn page(&self, limit: usize, before_ms: Option<u64>) -> AuditPage {
        let limit = limit.clamp(1, MAX_PAGE_LIMIT);
        let entries = self.read_all().await.unwrap_or_else(|error| {
            warn!(%error, "failed to read audit log");
            Vec::new()
        });

        let mut newest_first = entries
            .into_iter()
            .filter(|entry| before_ms.is_none_or(|before| entry.timestamp_ms < before))
            .collect::<Vec<_>>();
        newest_first.reverse();

        let has_more = newest_first.len() > limit;
        newest_first.truncate(limit);
        AuditPage {
            events: newest_first,
            has_more,
        }
    }

    /// Returns every stored entry (bounded by `MAX_AUDIT_ENTRIES`), newest
    /// first. Used for the full JSON export, which must not be clamped to
    /// the page size [`page`] uses for the in-app viewer.
    pub async fn export(&self) -> AuditPage {
        let mut entries = self.read_all().await.unwrap_or_else(|error| {
            warn!(%error, "failed to read audit log");
            Vec::new()
        });
        entries.reverse();
        AuditPage {
            events: entries,
            has_more: false,
        }
    }

    async fn read_all(&self) -> Result<Vec<AuditEvent>, String> {
        let content = match tokio::fs::read_to_string(&self.path).await {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(format!("failed to read {}: {error}", self.path.display()));
            }
        };
        Ok(content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<AuditEvent>(line).ok())
            .collect())
    }

    async fn write_all(&self, entries: &[AuditEvent]) -> Result<(), String> {
        let mut buffer = String::new();
        for entry in entries {
            let line = serde_json::to_string(entry)
                .map_err(|error| format!("failed to encode audit entry: {error}"))?;
            buffer.push_str(&line);
            buffer.push('\n');
        }
        atomic_write_secure(&self.path, buffer.as_bytes()).await
    }
}

pub const fn default_limit() -> usize {
    DEFAULT_PAGE_LIMIT
}

pub fn attachment(page: &AuditPage) -> Response {
    serde_json::to_vec_pretty(page).map_or_else(
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::AuditLog;

    fn temp_log() -> AuditLog {
        AuditLog::new(std::env::temp_dir().join(format!(
            "deckox-audit-test-{}.log",
            hex::encode(rand::random::<[u8; 8]>())
        )))
    }

    #[tokio::test]
    async fn records_events_newest_first() {
        let log = temp_log();
        let ip = IpAddr::from([192, 0, 2, 1]);
        log.record("auth_login", "admin", ip, "success", None).await;
        log.record("auth_logout", "admin", ip, "success", None)
            .await;

        let page = log.page(10, None).await;
        assert_eq!(page.events.len(), 2);
        assert_eq!(page.events[0].event, "auth_logout");
        assert_eq!(page.events[1].event, "auth_login");
        assert!(!page.has_more);

        let _ = tokio::fs::remove_file(&log.path).await;
    }

    #[tokio::test]
    async fn caps_stored_entries_and_reports_has_more() {
        let log = AuditLog {
            path: temp_log().path,
            write_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        };
        let ip = IpAddr::from([192, 0, 2, 2]);
        for _ in 0..5 {
            log.record("service_action", "admin", ip, "success", None)
                .await;
        }

        let page = log.page(2, None).await;
        assert_eq!(page.events.len(), 2);
        assert!(page.has_more);

        let _ = tokio::fs::remove_file(&log.path).await;
    }
}
