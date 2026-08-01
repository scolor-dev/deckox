use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_stream::stream;
use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use deckox_protocol::{RealtimeMetricsEvent, SystemMetrics};
use serde::Deserialize;
use tokio::{
    runtime::Handle,
    sync::{Mutex, Notify, broadcast},
};
use tracing::{debug, warn};

use crate::{agent_client::AgentClient, request_context::RequestId};

const DEFAULT_INTERVAL_SECONDS: u64 = 1;
const ALLOWED_INTERVAL_SECONDS: [u64; 3] = [1, 2, 5];
const CHANNEL_CAPACITY: usize = 32;

#[derive(Clone)]
pub struct MetricsHub {
    inner: Arc<MetricsHubInner>,
}

struct MetricsHubInner {
    agent: AgentClient,
    sender: broadcast::Sender<RealtimeMetricsEvent>,
    subscriptions: Mutex<HashMap<u64, Duration>>,
    subscriptions_changed: Notify,
    next_subscription_id: AtomicU64,
    next_sequence: AtomicU64,
}

struct MetricsSubscription {
    receiver: broadcast::Receiver<RealtimeMetricsEvent>,
    interval: Duration,
    _guard: SubscriptionGuard,
}

struct SubscriptionGuard {
    id: u64,
    inner: Arc<MetricsHubInner>,
}

#[derive(Debug, Default, Deserialize)]
pub struct MetricsQuery {
    interval: Option<u64>,
}

impl MetricsHub {
    pub fn new(agent: AgentClient) -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        let inner = Arc::new(MetricsHubInner {
            agent,
            sender,
            subscriptions: Mutex::new(HashMap::new()),
            subscriptions_changed: Notify::new(),
            next_subscription_id: AtomicU64::new(1),
            next_sequence: AtomicU64::new(1),
        });
        tokio::spawn(run_sampler(Arc::clone(&inner)));
        Self { inner }
    }

    async fn subscribe(&self, interval: Duration) -> MetricsSubscription {
        let id = self
            .inner
            .next_subscription_id
            .fetch_add(1, Ordering::Relaxed);
        let receiver = self.inner.sender.subscribe();
        self.inner.subscriptions.lock().await.insert(id, interval);
        self.inner.subscriptions_changed.notify_one();
        MetricsSubscription {
            receiver,
            interval,
            _guard: SubscriptionGuard {
                id,
                inner: Arc::clone(&self.inner),
            },
        }
    }
}

impl Drop for SubscriptionGuard {
    fn drop(&mut self) {
        let id = self.id;
        let inner = Arc::clone(&self.inner);
        if let Ok(handle) = Handle::try_current() {
            handle.spawn(async move {
                inner.subscriptions.lock().await.remove(&id);
                inner.subscriptions_changed.notify_one();
                debug!(subscription_id = id, "metrics SSE subscriber removed");
            });
        }
    }
}

pub async fn metrics_events(
    State(hub): State<MetricsHub>,
    Query(query): Query<MetricsQuery>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let subscription = hub.subscribe(query.interval()).await;
    let event_stream = stream! {
        let MetricsSubscription {
            mut receiver,
            interval,
            _guard,
        } = subscription;
        let mut last_sent = None::<Instant>;
        loop {
            match receiver.recv().await {
                Ok(payload) => {
                    let now = Instant::now();
                    if last_sent.is_some_and(|last| now.duration_since(last) < interval) {
                        continue;
                    }
                    last_sent = Some(now);
                    if let Ok(data) = serde_json::to_string(&payload) {
                        yield Ok(Event::default()
                            .event("metrics")
                            .id(payload.sequence.to_string())
                            .data(data));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "metrics SSE subscriber lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

impl MetricsQuery {
    fn interval(&self) -> Duration {
        let seconds = self
            .interval
            .filter(|value| ALLOWED_INTERVAL_SECONDS.contains(value))
            .unwrap_or(DEFAULT_INTERVAL_SECONDS);
        Duration::from_secs(seconds)
    }
}

async fn run_sampler(inner: Arc<MetricsHubInner>) {
    loop {
        let changed = inner.subscriptions_changed.notified();
        let interval = {
            let subscriptions = inner.subscriptions.lock().await;
            minimum_interval(&subscriptions)
        };
        let Some(interval) = interval else {
            changed.await;
            continue;
        };

        let sequence = inner.next_sequence.fetch_add(1, Ordering::Relaxed);
        let request_id = RequestId(format!(
            "metrics-{}",
            hex::encode(rand::random::<[u8; 12]>())
        ));
        let payload = match inner
            .agent
            .get_json::<SystemMetrics>("/v1/system/metrics", &request_id)
            .await
        {
            Ok(metrics) => RealtimeMetricsEvent {
                sequence,
                timestamp_ms: unix_timestamp_ms(),
                agent_online: true,
                metrics: Some(metrics),
                error_code: None,
            },
            Err(error) => {
                warn!(%error, "failed to sample real-time metrics");
                RealtimeMetricsEvent {
                    sequence,
                    timestamp_ms: unix_timestamp_ms(),
                    agent_online: false,
                    metrics: None,
                    error_code: Some("agent_unavailable".to_owned()),
                }
            }
        };
        let _ = inner.sender.send(payload);

        tokio::select! {
            () = tokio::time::sleep(interval) => {}
            () = changed => {}
        }
    }
}

fn minimum_interval(subscriptions: &HashMap<u64, Duration>) -> Option<Duration> {
    subscriptions.values().copied().min()
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use super::{MetricsQuery, minimum_interval};

    #[test]
    fn accepts_only_supported_intervals() {
        assert_eq!(
            MetricsQuery { interval: Some(2) }.interval(),
            Duration::from_secs(2)
        );
        assert_eq!(
            MetricsQuery { interval: Some(3) }.interval(),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn sampler_is_idle_without_subscribers() {
        assert_eq!(minimum_interval(&HashMap::new()), None);
    }

    #[test]
    fn sampler_uses_fastest_subscriber_interval() {
        let subscriptions = HashMap::from([
            (1, Duration::from_secs(5)),
            (2, Duration::from_secs(1)),
            (3, Duration::from_secs(2)),
        ]);
        assert_eq!(
            minimum_interval(&subscriptions),
            Some(Duration::from_secs(1))
        );
    }
}
