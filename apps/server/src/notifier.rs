//! Background threshold monitor that posts a webhook when something needs
//! attention even while nobody has the admin panel open: an allow-listed
//! service enters the `failed` state, swap or the busiest mount crosses a
//! warning threshold (the same ones the overview page already highlights),
//! or the Agent itself becomes unreachable. Each condition is edge-triggered
//! (fired once on entry, once on recovery) so a sustained breach does not
//! spam the webhook on every poll.

use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{
    ServiceSummary, StorageMount, SystemMetrics, UpdateCheckStatus, UpdateStatus,
};
use reqwest::Client;
use tracing::warn;

use crate::{
    agent_client::AgentClient, audit::AuditLog, request_context::RequestId, update::UpdateChecker,
};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(5);
const SWAP_HIGH_PERCENT: f64 = 80.0;
const DISK_HIGH_PERCENT: f64 = 90.0;

#[derive(Default)]
struct NotifierState {
    agent_unreachable: bool,
    swap_high: bool,
    disk_high: bool,
    failed_services: HashSet<String>,
    /// The latest version we have already sent a webhook for, so a sustained
    /// `update_available` state is not renotified every tick — only a
    /// version that is new relative to the last one we announced.
    notified_update_version: Option<String>,
}

/// Spawns the polling loop. A no-op (returns immediately without spawning)
/// when no webhook URL is configured, so the feature costs nothing when
/// unused.
pub fn spawn(
    agent: AgentClient,
    audit: AuditLog,
    updates: UpdateChecker,
    webhook_url: Option<String>,
) {
    let Some(webhook_url) = webhook_url else {
        return;
    };
    tokio::spawn(async move {
        let client = Client::new();
        let mut state = NotifierState::default();
        loop {
            tick(&agent, &audit, &updates, &client, &webhook_url, &mut state).await;
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn tick(
    agent: &AgentClient,
    audit: &AuditLog,
    updates: &UpdateChecker,
    client: &Client,
    webhook_url: &str,
    state: &mut NotifierState,
) {
    check_update(updates, client, webhook_url, audit, state).await;

    let request_id = RequestId(format!(
        "monitor-{}",
        hex::encode(rand::random::<[u8; 16]>())
    ));

    let services = agent
        .get_json::<Vec<ServiceSummary>>("/v1/services", &request_id)
        .await;
    let metrics = agent
        .get_json::<SystemMetrics>("/v1/system/metrics", &request_id)
        .await;
    let storage = agent
        .get_json::<Vec<StorageMount>>("/v1/storage", &request_id)
        .await;

    let (Ok(services), Ok(metrics), Ok(storage)) = (services, metrics, storage) else {
        if !state.agent_unreachable {
            state.agent_unreachable = true;
            send(
                client,
                webhook_url,
                audit,
                "agent_unreachable",
                "Deckox AgentへAPI接続できません。".to_owned(),
            )
            .await;
        }
        return;
    };
    if state.agent_unreachable {
        state.agent_unreachable = false;
        send(
            client,
            webhook_url,
            audit,
            "agent_recovered",
            "Deckox AgentへのAPI接続が復旧しました。".to_owned(),
        )
        .await;
    }

    check_swap(&metrics, client, webhook_url, audit, state).await;
    check_disk(&storage, client, webhook_url, audit, state).await;
    check_services(&services, client, webhook_url, audit, state).await;
}

/// `None` when `percent` has not crossed `threshold` since `was_high`;
/// otherwise the new state to store. Kept free of I/O so the edge-triggering
/// logic is unit-testable without a live webhook endpoint.
fn threshold_transition(was_high: bool, percent: f64, threshold: f64) -> Option<bool> {
    if percent >= threshold && !was_high {
        Some(true)
    } else if percent < threshold && was_high {
        Some(false)
    } else {
        None
    }
}

/// Services that just entered `failed` (first) and services that just left
/// it while still present in `services` (second) — a service that
/// disappeared from the list entirely (e.g. removed from the allowlist) is
/// dropped silently rather than reported as "recovered". Also returns the
/// updated failed-set to store.
fn service_transitions(
    previously_failed: &HashSet<String>,
    services: &[ServiceSummary],
) -> (Vec<String>, Vec<String>, HashSet<String>) {
    let by_id: HashMap<&str, &ServiceSummary> = services
        .iter()
        .map(|service| (service.id.as_str(), service))
        .collect();
    let currently_failed: HashSet<String> = services
        .iter()
        .filter(|service| service.control_allowed && service.active_state == "failed")
        .map(|service| service.id.clone())
        .collect();

    let newly_failed = currently_failed
        .difference(previously_failed)
        .cloned()
        .collect();
    let recovered = previously_failed
        .difference(&currently_failed)
        .filter(|id| by_id.contains_key(id.as_str()))
        .cloned()
        .collect();

    (newly_failed, recovered, currently_failed)
}

/// `UpdateChecker::check` caches its GitHub lookup for 15 minutes, so
/// calling it on every 30-second tick is cheap — it only actually reaches
/// GitHub as often as the cache expires.
async fn check_update(
    updates: &UpdateChecker,
    client: &Client,
    webhook_url: &str,
    audit: &AuditLog,
    state: &mut NotifierState,
) {
    let status = updates.check().await;
    match status.status {
        // Genuinely caught up — clear so a future release notifies again.
        UpdateCheckStatus::UpToDate => {
            state.notified_update_version = None;
            return;
        }
        // The check itself failed (e.g. GitHub hiccup); this says nothing
        // about whether an already-announced update is still pending, so
        // leave `notified_update_version` untouched rather than resetting
        // it — resetting here would re-fire the same notification once the
        // next successful check sees the same version again.
        UpdateCheckStatus::Unavailable => return,
        UpdateCheckStatus::Available => {}
    }
    let Some(latest_version) =
        update_notification(&status, state.notified_update_version.as_deref())
    else {
        return;
    };
    state.notified_update_version = Some(latest_version.clone());

    let mut message = format!(
        "新しいバージョン {latest_version} が利用可能です(現在: {})。",
        status.current_version
    );
    if let Some(release_url) = &status.release_url {
        let _ = write!(message, " {release_url}");
    }
    send(client, webhook_url, audit, "update_available", message).await;
}

/// `Some(version)` when `status` names a newer version than the one we last
/// notified about; `None` when there is nothing new to say. Assumes the
/// caller has already checked `status.update_available` (and cleared
/// `previously_notified` when it is false) — kept as a separate, pure
/// function so the "is this actually new" decision is unit-testable without
/// a live `UpdateChecker`.
fn update_notification(status: &UpdateStatus, previously_notified: Option<&str>) -> Option<String> {
    let latest = status.latest_version.as_ref()?;
    if previously_notified == Some(latest.as_str()) {
        return None;
    }
    Some(latest.clone())
}

async fn check_swap(
    metrics: &SystemMetrics,
    client: &Client,
    webhook_url: &str,
    audit: &AuditLog,
    state: &mut NotifierState,
) {
    if metrics.memory.swap_total_bytes == 0 {
        return;
    }
    // Real memory/swap sizes stay far below 2^52, so this conversion never
    // actually loses precision — the byte counts involved fit exactly in an
    // f64 mantissa.
    #[allow(clippy::cast_precision_loss)]
    let swap_percent =
        metrics.memory.swap_used_bytes as f64 / metrics.memory.swap_total_bytes as f64 * 100.0;
    let Some(now_high) = threshold_transition(state.swap_high, swap_percent, SWAP_HIGH_PERCENT)
    else {
        return;
    };
    state.swap_high = now_high;
    let (event, message) = if now_high {
        (
            "swap_high",
            format!("Swap使用率が{swap_percent:.0}%になりました(閾値{SWAP_HIGH_PERCENT:.0}%)。"),
        )
    } else {
        (
            "swap_normal",
            format!("Swap使用率が{swap_percent:.0}%まで下がりました。"),
        )
    };
    send(client, webhook_url, audit, event, message).await;
}

async fn check_disk(
    storage: &[StorageMount],
    client: &Client,
    webhook_url: &str,
    audit: &AuditLog,
    state: &mut NotifierState,
) {
    let Some(busiest) = storage
        .iter()
        .max_by(|a, b| a.usage_percent.total_cmp(&b.usage_percent))
    else {
        return;
    };
    let Some(now_high) =
        threshold_transition(state.disk_high, busiest.usage_percent, DISK_HIGH_PERCENT)
    else {
        return;
    };
    state.disk_high = now_high;
    let (event, message) = if now_high {
        (
            "disk_high",
            format!(
                "{}の使用率が{:.0}%になりました(閾値{DISK_HIGH_PERCENT:.0}%)。",
                busiest.mount_point, busiest.usage_percent
            ),
        )
    } else {
        (
            "disk_normal",
            format!(
                "{}の使用率が{:.0}%まで下がりました。",
                busiest.mount_point, busiest.usage_percent
            ),
        )
    };
    send(client, webhook_url, audit, event, message).await;
}

async fn check_services(
    services: &[ServiceSummary],
    client: &Client,
    webhook_url: &str,
    audit: &AuditLog,
    state: &mut NotifierState,
) {
    let (newly_failed, recovered, currently_failed) =
        service_transitions(&state.failed_services, services);

    for id in newly_failed {
        send(
            client,
            webhook_url,
            audit,
            "service_failed",
            format!("{id} がfailed状態になりました。"),
        )
        .await;
    }
    for id in recovered {
        send(
            client,
            webhook_url,
            audit,
            "service_recovered",
            format!("{id} が復旧しました。"),
        )
        .await;
    }
    state.failed_services = currently_failed;
}

/// Posts one notification and records the outcome to the audit log under the
/// `system` actor — used by the background ticker, which has nobody to
/// attribute the send to. [`send_once`] is the version other callers (the
/// admin-triggered "send test notification" button) use when they want to
/// report the outcome themselves instead.
async fn send(client: &Client, webhook_url: &str, audit: &AuditLog, event: &str, message: String) {
    match send_once(client, webhook_url, event, message).await {
        Ok(()) => {
            audit
                .record_system("webhook_notification", "success", Some(event.to_owned()))
                .await;
        }
        Err(error) => {
            warn!(%error, event, "failed to send webhook notification");
            audit
                .record_system(
                    "webhook_notification",
                    "failure",
                    Some(format!("event={event} error={error}")),
                )
                .await;
        }
    }
}

/// Posts one webhook notification and returns a human-readable failure
/// reason on anything short of a successful HTTP response. Does no logging
/// of its own so it can be reused by both the unattended ticker (via
/// [`send`], which audit-logs as `system`) and the settings page's test
/// button (which reports the outcome straight back to the admin who clicked
/// it).
pub async fn send_once(
    client: &Client,
    webhook_url: &str,
    event: &str,
    message: String,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "source": "deckox",
        "event": event,
        "message": message,
        "text": message,
        "timestamp_ms": now_ms(),
    });
    let body = serde_json::to_vec(&payload)
        .map_err(|error| format!("failed to encode payload: {error}"))?;

    let response = client
        .post(webhook_url)
        .timeout(WEBHOOK_TIMEOUT)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "webhook endpoint returned HTTP {}",
            response.status()
        ))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_transition_fires_once_on_the_way_up_and_down() {
        assert_eq!(threshold_transition(false, 79.0, 80.0), None);
        assert_eq!(threshold_transition(false, 80.0, 80.0), Some(true));
        assert_eq!(threshold_transition(true, 80.0, 80.0), None);
        assert_eq!(threshold_transition(true, 79.9, 80.0), Some(false));
        assert_eq!(threshold_transition(false, 79.9, 80.0), None);
    }

    fn update_status(latest_version: Option<&str>) -> UpdateStatus {
        UpdateStatus {
            // Deliberately not a real release version, so `bump-version.sh`'s
            // leftover-reference check never flags this fixture as a stale
            // version string to update.
            current_version: "0.0.0-test".to_owned(),
            latest_version: latest_version.map(str::to_owned),
            update_available: latest_version.is_some(),
            release_url: None,
            checked_at_ms: Some(0),
            status: if latest_version.is_some() {
                UpdateCheckStatus::Available
            } else {
                UpdateCheckStatus::UpToDate
            },
        }
    }

    #[test]
    fn update_notification_fires_once_per_new_version() {
        // Deliberately out of range for any real release (which stay in the
        // 0.x series for the foreseeable future), so `bump-version.sh`'s
        // leftover-reference check never flags these as stale version
        // strings to update.
        let status = update_status(Some("9.9.9"));

        assert_eq!(update_notification(&status, None), Some("9.9.9".to_owned()));
        assert_eq!(
            update_notification(&status, Some("9.9.9")),
            None,
            "already notified about this exact version"
        );
        assert_eq!(
            update_notification(&status, Some("9.9.8")),
            Some("9.9.9".to_owned()),
            "a newer version than the one last notified should notify again"
        );
    }

    fn service(id: &str, active_state: &str, control_allowed: bool) -> ServiceSummary {
        ServiceSummary {
            id: id.to_owned(),
            description: String::new(),
            load_state: "loaded".to_owned(),
            active_state: active_state.to_owned(),
            sub_state: String::new(),
            unit_file_state: None,
            control_allowed,
            standard_system: false,
            deckox_managed: false,
            product: None,
        }
    }

    #[test]
    fn reports_a_newly_failed_allowlisted_service() {
        let previously_failed = HashSet::new();
        let services = vec![service("nginx.service", "failed", true)];

        let (newly_failed, recovered, currently_failed) =
            service_transitions(&previously_failed, &services);

        assert_eq!(newly_failed, vec!["nginx.service".to_owned()]);
        assert!(recovered.is_empty());
        assert!(currently_failed.contains("nginx.service"));
    }

    #[test]
    fn ignores_failures_outside_the_control_allowlist() {
        let previously_failed = HashSet::new();
        let services = vec![service("random.service", "failed", false)];

        let (newly_failed, _recovered, currently_failed) =
            service_transitions(&previously_failed, &services);

        assert!(newly_failed.is_empty());
        assert!(currently_failed.is_empty());
    }

    #[test]
    fn reports_recovery_only_when_the_service_is_still_present() {
        let mut previously_failed = HashSet::new();
        previously_failed.insert("nginx.service".to_owned());
        previously_failed.insert("removed.service".to_owned());
        let services = vec![service("nginx.service", "active", true)];

        let (newly_failed, recovered, currently_failed) =
            service_transitions(&previously_failed, &services);

        assert!(newly_failed.is_empty());
        assert_eq!(recovered, vec!["nginx.service".to_owned()]);
        assert!(currently_failed.is_empty());
    }
}
