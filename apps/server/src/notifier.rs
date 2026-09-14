//! Background threshold monitor that posts a webhook when something needs
//! attention even while nobody has the admin panel open: an allow-listed
//! service enters the `failed` state, swap or the busiest mount crosses a
//! warning threshold (the same ones the overview page already highlights),
//! or the Agent itself becomes unreachable. Each condition is edge-triggered
//! (fired once on entry, once on recovery) so a sustained breach does not
//! spam the webhook on every poll.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{ServiceSummary, StorageMount, SystemMetrics};
use reqwest::Client;
use tracing::warn;

use crate::{agent_client::AgentClient, audit::AuditLog, request_context::RequestId};

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
}

/// Spawns the polling loop. A no-op (returns immediately without spawning)
/// when no webhook URL is configured, so the feature costs nothing when
/// unused.
pub fn spawn(agent: AgentClient, audit: AuditLog, webhook_url: Option<String>) {
    let Some(webhook_url) = webhook_url else {
        return;
    };
    tokio::spawn(async move {
        let client = Client::new();
        let mut state = NotifierState::default();
        loop {
            tick(&agent, &audit, &client, &webhook_url, &mut state).await;
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn tick(
    agent: &AgentClient,
    audit: &AuditLog,
    client: &Client,
    webhook_url: &str,
    state: &mut NotifierState,
) {
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

    let (services, metrics, storage) = match (services, metrics, storage) {
        (Ok(services), Ok(metrics), Ok(storage)) => {
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
            (services, metrics, storage)
        }
        _ => {
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
        }
    };

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

async fn send(client: &Client, webhook_url: &str, audit: &AuditLog, event: &str, message: String) {
    let payload = serde_json::json!({
        "source": "deckox",
        "event": event,
        "message": message,
        "text": message,
        "timestamp_ms": now_ms(),
    });
    let body = match serde_json::to_vec(&payload) {
        Ok(body) => body,
        Err(error) => {
            warn!(%error, event, "failed to encode webhook payload");
            return;
        }
    };

    let result = client
        .post(webhook_url)
        .timeout(WEBHOOK_TIMEOUT)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await;

    match result {
        Ok(response) if response.status().is_success() => {
            audit
                .record_system("webhook_notification", "success", Some(event.to_owned()))
                .await;
        }
        Ok(response) => {
            let status = response.status();
            warn!(%status, event, "webhook endpoint rejected notification");
            audit
                .record_system(
                    "webhook_notification",
                    "failure",
                    Some(format!("event={event} status={status}")),
                )
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
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
