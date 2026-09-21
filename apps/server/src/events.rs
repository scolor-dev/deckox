//! The Server's event feed: everything worth telling the admin about, in one
//! numbered stream. It follows the Agent's own event stream (Server → Agent,
//! never the other way round) and adds what the Server itself notices — the
//! Agent restarting, disappearing, or speaking a different protocol version.
//! The Web reads it with `GET /api/v1/events?after=`.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{AgentEvent, AgentInfo, EventBatch, EventResult, PROTOCOL_VERSION};
use serde::Serialize;
use tracing::{info, warn};

use crate::{agent_client::AgentClient, request_context::RequestId};

const CAPACITY: usize = 500;
const MAX_BATCH: usize = 200;
const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize)]
pub struct FeedEvent {
    /// Position in this Server process's feed, starting at 1.
    pub seq: u64,
    /// `agent` for events the Agent reported, `server` for the Server's own.
    pub source: &'static str,
    pub timestamp_ms: u64,
    pub kind: String,
    pub result: EventResult,
    pub subject: String,
    pub request_id: Option<String>,
    pub command_id: Option<String>,
    pub message: Option<String>,
}

/// What the Server currently knows about the Agent it talks to.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AgentLink {
    pub connected: bool,
    pub agent_version: Option<String>,
    pub protocol_version: Option<u32>,
    /// `false` when the Agent speaks a different protocol version, in which
    /// case its events are not read.
    pub compatible: bool,
}

#[derive(Debug, Serialize)]
pub struct FeedBatch {
    /// Identifies this Server process; numbers start over when it changes.
    pub instance_id: String,
    pub events: Vec<FeedEvent>,
    pub next_after: u64,
    pub agent: AgentLink,
}

#[derive(Default)]
struct Inner {
    next_seq: u64,
    events: VecDeque<FeedEvent>,
    link: AgentLink,
    epoch: Option<String>,
    agent_position: u64,
    reported_mismatch: bool,
}

#[derive(Clone)]
pub struct EventFeed {
    inner: Arc<Mutex<Inner>>,
    instance_id: Arc<str>,
}

impl EventFeed {
    pub fn new(instance_id: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                next_seq: 1,
                ..Inner::default()
            })),
            instance_id: instance_id.into(),
        }
    }

    pub fn link(&self) -> AgentLink {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .link
            .clone()
    }

    pub fn since(&self, after: u64) -> FeedBatch {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let events: Vec<FeedEvent> = inner
            .events
            .iter()
            .filter(|event| event.seq > after)
            .take(MAX_BATCH)
            .cloned()
            .collect();
        let next_after = events
            .last()
            .map_or_else(|| after.max(inner.next_seq - 1), |event| event.seq);
        FeedBatch {
            instance_id: self.instance_id.to_string(),
            events,
            next_after,
            agent: inner.link.clone(),
        }
    }

    /// Records the answer to `GET /v1/info`. Returns whether the Agent's
    /// events should be read.
    fn apply_info(&self, info: &AgentInfo) -> bool {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let compatible = info.protocol_version == PROTOCOL_VERSION;
        let was_connected = inner.link.connected;
        inner.link = AgentLink {
            connected: true,
            agent_version: Some(info.agent_version.clone()),
            protocol_version: Some(info.protocol_version),
            compatible,
        };
        if !was_connected {
            push(
                &mut inner,
                "agent_connected",
                EventResult::Completed,
                "agent",
                None,
            );
        }
        if inner
            .epoch
            .as_deref()
            .is_some_and(|seen| seen != info.epoch)
        {
            inner.agent_position = 0;
            push(
                &mut inner,
                "agent_restarted",
                EventResult::Completed,
                "agent",
                None,
            );
        }
        inner.epoch = Some(info.epoch.clone());
        if !compatible && !inner.reported_mismatch {
            inner.reported_mismatch = true;
            let message = format!(
                "Agent speaks protocol {} but this Server speaks {PROTOCOL_VERSION}; restart both after updating",
                info.protocol_version
            );
            push(
                &mut inner,
                "protocol_mismatch",
                EventResult::Failed,
                "agent",
                Some(message),
            );
        }
        drop(inner);
        compatible
    }

    fn apply_unreachable(&self, reason: &str) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if inner.link.connected {
            push(
                &mut inner,
                "agent_disconnected",
                EventResult::Failed,
                "agent",
                Some(reason.to_owned()),
            );
        }
        inner.link = AgentLink::default();
    }

    fn position(&self) -> (u64, Option<String>) {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (inner.agent_position, inner.epoch.clone())
    }

    fn apply_batch(&self, batch: EventBatch) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for event in batch.events {
            let AgentEvent {
                timestamp_ms,
                kind,
                result,
                subject,
                request_id,
                command_id,
                message,
                ..
            } = event;
            let seq = inner.next_seq;
            inner.next_seq += 1;
            trim(&mut inner);
            inner.events.push_back(FeedEvent {
                seq,
                source: "agent",
                timestamp_ms,
                kind,
                result,
                subject,
                request_id,
                command_id,
                message,
            });
        }
        inner.agent_position = batch.next_after;
    }
}

fn trim(inner: &mut Inner) {
    if inner.events.len() >= CAPACITY {
        inner.events.pop_front();
    }
}

fn push(
    inner: &mut Inner,
    kind: &str,
    result: EventResult,
    subject: &str,
    message: Option<String>,
) {
    let seq = inner.next_seq;
    inner.next_seq += 1;
    trim(inner);
    inner.events.push_back(FeedEvent {
        seq,
        source: "server",
        timestamp_ms: now_ms(),
        kind: kind.to_owned(),
        result,
        subject: subject.to_owned(),
        request_id: None,
        command_id: None,
        message,
    });
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Keeps the feed following the Agent for the life of the process.
pub fn spawn(agent: AgentClient, feed: EventFeed) {
    tokio::spawn(async move {
        loop {
            follow_once(&agent, &feed).await;
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn follow_once(agent: &AgentClient, feed: &EventFeed) {
    let request_id = RequestId(format!("feed-{}", hex::encode(rand::random::<[u8; 6]>())));
    let info = match agent.get_json::<AgentInfo>("/v1/info", &request_id).await {
        Ok(info) => info,
        Err(error) => {
            // An Agent from before /v1/info existed answers 404: it is
            // reachable but speaks an older protocol.
            if error.contains("HTTP 404") {
                let legacy = AgentInfo {
                    protocol_version: 0,
                    agent_version: "unknown".to_owned(),
                    epoch: "legacy".to_owned(),
                };
                feed.apply_info(&legacy);
            } else {
                feed.apply_unreachable(&error);
            }
            return;
        }
    };
    if !feed.apply_info(&info) {
        return;
    }
    let (after, epoch) = feed.position();
    let path = epoch.map_or_else(
        || format!("/v1/events?after={after}"),
        |epoch| format!("/v1/events?after={after}&epoch={epoch}"),
    );
    match agent.get_json::<EventBatch>(&path, &request_id).await {
        Ok(batch) => feed.apply_batch(batch),
        Err(error) => {
            warn!(%error, "failed to read the Agent's events");
            info!("will retry");
        }
    }
}

#[cfg(test)]
mod tests {
    use deckox_protocol::{AgentEvent, AgentInfo, EventBatch, EventResult, PROTOCOL_VERSION};

    use super::EventFeed;

    fn info(epoch: &str, protocol_version: u32) -> AgentInfo {
        AgentInfo {
            protocol_version,
            agent_version: "0.0.0".to_owned(),
            epoch: epoch.to_owned(),
        }
    }

    fn agent_event(seq: u64, subject: &str) -> AgentEvent {
        AgentEvent {
            seq,
            timestamp_ms: 1,
            kind: "service_action".to_owned(),
            result: EventResult::Accepted,
            subject: subject.to_owned(),
            request_id: Some("req".to_owned()),
            command_id: None,
            message: None,
        }
    }

    fn kinds(feed: &EventFeed) -> Vec<String> {
        feed.since(0)
            .events
            .into_iter()
            .map(|event| event.kind)
            .collect()
    }

    #[test]
    fn follows_the_agent_and_numbers_its_own_stream() {
        let feed = EventFeed::new("instance");
        assert!(feed.apply_info(&info("e1", PROTOCOL_VERSION)));
        feed.apply_batch(EventBatch {
            epoch: "e1".to_owned(),
            events: vec![agent_event(1, "a"), agent_event(2, "b")],
            next_after: 2,
        });

        let batch = feed.since(0);
        assert_eq!(
            kinds(&feed),
            ["agent_connected", "service_action", "service_action"]
        );
        assert_eq!(batch.next_after, 3);
        assert_eq!(feed.position(), (2, Some("e1".to_owned())));
        assert!(batch.agent.connected && batch.agent.compatible);
        assert_eq!(feed.since(3).events.len(), 0);
    }

    #[test]
    fn a_new_epoch_means_the_agent_restarted() {
        let feed = EventFeed::new("instance");
        feed.apply_info(&info("e1", PROTOCOL_VERSION));
        feed.apply_batch(EventBatch {
            epoch: "e1".to_owned(),
            events: vec![agent_event(1, "a")],
            next_after: 1,
        });
        feed.apply_info(&info("e2", PROTOCOL_VERSION));

        assert_eq!(
            feed.position(),
            (0, Some("e2".to_owned())),
            "read from the start again"
        );
        assert!(kinds(&feed).contains(&"agent_restarted".to_owned()));
    }

    #[test]
    fn losing_and_regaining_the_agent_is_reported_once_each() {
        let feed = EventFeed::new("instance");
        feed.apply_info(&info("e1", PROTOCOL_VERSION));
        feed.apply_unreachable("gone");
        feed.apply_unreachable("still gone");
        assert!(!feed.since(0).agent.connected);
        feed.apply_info(&info("e1", PROTOCOL_VERSION));

        assert_eq!(
            kinds(&feed),
            ["agent_connected", "agent_disconnected", "agent_connected"]
        );
    }

    #[test]
    fn a_different_protocol_version_is_reported_and_events_are_skipped() {
        let feed = EventFeed::new("instance");
        assert!(!feed.apply_info(&info("e1", PROTOCOL_VERSION + 1)));
        assert!(!feed.apply_info(&info("e1", PROTOCOL_VERSION + 1)));

        let mismatches = kinds(&feed)
            .into_iter()
            .filter(|kind| kind == "protocol_mismatch")
            .count();
        assert_eq!(mismatches, 1, "reported once, not on every poll");
        assert!(!feed.since(0).agent.compatible);
    }

    #[tokio::test]
    async fn follows_a_real_socket_and_reads_only_new_events() {
        use serde_json::json;

        use crate::{agent_client::AgentClient, test_support::FakeAgent};

        let info =
            json!({"protocol_version": PROTOCOL_VERSION, "agent_version": "9.9.9", "epoch": "e1"});
        let batch = json!({
            "epoch": "e1",
            "events": [{
                "seq": 1, "timestamp_ms": 5, "kind": "service_action", "result": "accepted",
                "subject": "service=a.service action=stop", "request_id": "r1",
                "command_id": null, "message": null
            }],
            "next_after": 1
        });
        let agent = FakeAgent::start(&[
            ("GET", "/v1/info", 200, info),
            ("GET", "/v1/events", 200, batch),
        ]);
        let client = AgentClient::new(agent.socket.clone());
        let feed = EventFeed::new("instance");

        super::follow_once(&client, &feed).await;
        assert_eq!(kinds(&feed), ["agent_connected", "service_action"]);
        assert_eq!(feed.since(0).agent.agent_version.as_deref(), Some("9.9.9"));

        super::follow_once(&client, &feed).await;
        let paths: Vec<String> = agent
            .requests()
            .into_iter()
            .map(|request| request.path)
            .collect();
        assert_eq!(paths.iter().filter(|path| *path == "/v1/events").count(), 2);
        assert_eq!(
            feed.position(),
            (1, Some("e1".to_owned())),
            "the position moved on"
        );
    }

    #[tokio::test]
    async fn an_agent_without_v1_info_is_treated_as_an_older_protocol() {
        use crate::{agent_client::AgentClient, test_support::FakeAgent};

        let agent = FakeAgent::start(&[]);
        let feed = EventFeed::new("instance");
        super::follow_once(&AgentClient::new(agent.socket.clone()), &feed).await;

        let link = feed.since(0).agent;
        assert!(link.connected && !link.compatible);
        assert_eq!(link.protocol_version, Some(0));
        assert!(kinds(&feed).contains(&"protocol_mismatch".to_owned()));
    }

    #[tokio::test]
    async fn an_unreachable_socket_is_reported_as_disconnected() {
        use crate::agent_client::AgentClient;

        let feed = EventFeed::new("instance");
        feed.apply_info(&info("e1", PROTOCOL_VERSION));
        super::follow_once(&AgentClient::new("/nonexistent/agent.sock".into()), &feed).await;
        assert!(!feed.since(0).agent.connected);
        assert!(kinds(&feed).contains(&"agent_disconnected".to_owned()));
    }
}
