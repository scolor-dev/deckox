//! The Agent's event stream: a bounded, numbered log of what happened on the
//! host. Modules publish into it; the Server reads it with
//! `GET /v1/events?after=`. Keeping the direction Server → Agent means the
//! Agent never opens a connection out.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{AgentEvent, EventBatch, EventResult};

const CAPACITY: usize = 500;
const MAX_BATCH: usize = 200;

/// What a module supplies when it publishes; the bus adds number and time.
pub struct EventDraft {
    pub kind: &'static str,
    pub result: EventResult,
    pub subject: String,
    pub request_id: Option<String>,
    pub command_id: Option<String>,
    pub message: Option<String>,
}

struct Inner {
    next_seq: u64,
    events: VecDeque<AgentEvent>,
}

#[derive(Clone)]
pub struct EventBus {
    inner: Arc<Mutex<Inner>>,
    epoch: Arc<str>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                next_seq: 1,
                events: VecDeque::with_capacity(CAPACITY),
            })),
            epoch: format!("{}-{}", now_ms(), hex::encode(rand::random::<[u8; 4]>())).into(),
        }
    }

    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    pub fn publish(&self, draft: EventDraft) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let seq = inner.next_seq;
        inner.next_seq += 1;
        if inner.events.len() == CAPACITY {
            inner.events.pop_front();
        }
        inner.events.push_back(AgentEvent {
            seq,
            timestamp_ms: now_ms(),
            kind: draft.kind.to_owned(),
            result: draft.result,
            subject: draft.subject,
            request_id: draft.request_id,
            command_id: draft.command_id,
            message: draft.message,
        });
    }

    /// Events after position `after`. A consumer that last saw a different
    /// `epoch` (the Agent restarted) gets everything still held, since its
    /// old position no longer means anything.
    pub fn since(&self, epoch: Option<&str>, after: u64) -> EventBatch {
        let after = if epoch.is_some_and(|seen| seen != &*self.epoch) {
            0
        } else {
            after
        };
        let Ok(inner) = self.inner.lock() else {
            return EventBatch {
                epoch: self.epoch.to_string(),
                events: Vec::new(),
                next_after: after,
            };
        };
        let events: Vec<AgentEvent> = inner
            .events
            .iter()
            .filter(|event| event.seq > after)
            .take(MAX_BATCH)
            .cloned()
            .collect();
        let next_after = events
            .last()
            .map_or_else(|| after.max(inner.next_seq - 1), |event| event.seq);
        EventBatch {
            epoch: self.epoch.to_string(),
            events,
            next_after,
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use deckox_protocol::EventResult;

    use super::{CAPACITY, EventBus, EventDraft};

    fn draft(subject: &str) -> EventDraft {
        EventDraft {
            kind: "test",
            result: EventResult::Completed,
            subject: subject.to_owned(),
            request_id: None,
            command_id: None,
            message: None,
        }
    }

    #[test]
    fn numbers_events_and_returns_only_newer_ones() {
        let bus = EventBus::new();
        bus.publish(draft("a"));
        bus.publish(draft("b"));

        let all = bus.since(None, 0);
        assert_eq!(
            all.events.iter().map(|event| event.seq).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(all.next_after, 2);

        bus.publish(draft("c"));
        let newer = bus.since(Some(bus.epoch()), 2);
        assert_eq!(newer.events.len(), 1);
        assert_eq!(newer.events[0].subject, "c");
        assert_eq!(newer.next_after, 3);

        let none = bus.since(Some(bus.epoch()), 3);
        assert!(none.events.is_empty());
        assert_eq!(none.next_after, 3, "an idle consumer keeps its position");
    }

    #[test]
    fn a_different_epoch_restarts_from_the_beginning() {
        let bus = EventBus::new();
        bus.publish(draft("a"));
        let batch = bus.since(Some("some-older-agent-process"), 99);
        assert_eq!(batch.events.len(), 1, "stale position is ignored");
        assert_eq!(batch.epoch, bus.epoch());
    }

    #[test]
    fn keeps_only_the_most_recent_events() {
        let bus = EventBus::new();
        for index in 0..CAPACITY + 10 {
            bus.publish(draft(&index.to_string()));
        }
        let batch = bus.since(None, 0);
        assert_eq!(batch.events.first().map(|event| event.seq), Some(11));
        let mut position = 0;
        let mut seen = 0;
        loop {
            let page = bus.since(None, position);
            if page.events.is_empty() {
                break;
            }
            seen += page.events.len();
            position = page.next_after;
        }
        assert_eq!(seen, CAPACITY, "paging reaches every retained event");
    }
}
