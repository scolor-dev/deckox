//! Keeps operations on the same resource from running at the same time, and
//! runs long ones in the background as jobs.
//!
//! Every mutating operation takes a lock named after what it changes
//! (`packages`, `host`, `service:<id>`), so two `apt` runs, or a scheduled
//! restart and a manual one, queue instead of colliding. An operation started
//! with `?async=true` returns a [`Job`] straight away and finishes in the
//! background.

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{CommandResult, EventResult, Job, JobState};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    error::AgentError,
    events::{EventBus, EventDraft},
};

const KEPT_JOBS: usize = 100;

/// What an operation is, for its job record and its event.
pub struct OperationSpec {
    pub kind: &'static str,
    pub subject: String,
    /// The resource lock it takes, for example `packages`.
    pub key: String,
    pub request_id: Option<String>,
}

#[derive(Clone)]
pub struct Operations {
    locks: Arc<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
    jobs: Arc<Mutex<VecDeque<Job>>>,
    events: EventBus,
    sequence: Arc<AtomicU64>,
}

impl Operations {
    pub fn new(events: EventBus) -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            events,
            sequence: Arc::new(AtomicU64::new(1)),
        }
    }

    fn lock_for(&self, key: &str) -> Arc<AsyncMutex<()>> {
        let mut locks = self
            .locks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(locks.entry(key.to_owned()).or_default())
    }

    /// Runs `work` once nothing else holds the `key` lock.
    pub async fn exclusive<T>(&self, key: &str, work: impl Future<Output = T>) -> T {
        let lock = self.lock_for(key);
        let _guard = lock.lock().await;
        work.await
    }

    /// Starts `work` in the background and returns its job at once.
    pub fn submit(
        &self,
        spec: OperationSpec,
        work: impl Future<Output = Result<CommandResult, AgentError>> + Send + 'static,
    ) -> Job {
        let id = format!(
            "job-{}-{}",
            now_ms(),
            self.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let job = Job {
            id: id.clone(),
            kind: spec.kind.to_owned(),
            subject: spec.subject.clone(),
            state: JobState::Queued,
            created_ms: now_ms(),
            started_ms: None,
            finished_ms: None,
            message: None,
            request_id: spec.request_id.clone(),
        };
        self.store(job.clone());

        let operations = self.clone();
        tokio::spawn(async move {
            let lock = operations.lock_for(&spec.key);
            let _guard = lock.lock().await;
            operations.update(&id, |job| {
                job.state = JobState::Running;
                job.started_ms = Some(now_ms());
            });
            let outcome = work.await;
            let (state, result, message) = match &outcome {
                Ok(command) => (
                    JobState::Succeeded,
                    EventResult::Completed,
                    command.message.clone(),
                ),
                Err(error) => (
                    JobState::Failed,
                    EventResult::Failed,
                    Some(error.message().to_owned()),
                ),
            };
            operations.update(&id, |job| {
                job.state = state;
                job.finished_ms = Some(now_ms());
                job.message.clone_from(&message);
            });
            operations.events.publish(EventDraft {
                kind: spec.kind,
                result,
                subject: spec.subject,
                request_id: spec.request_id,
                command_id: Some(id),
                message,
            });
        });
        job
    }

    pub fn jobs(&self) -> Vec<Job> {
        self.jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .rev()
            .cloned()
            .collect()
    }

    pub fn job(&self, id: &str) -> Option<Job> {
        self.jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find(|job| job.id == id)
            .cloned()
    }

    fn store(&self, job: Job) {
        let mut jobs = self
            .jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if jobs.len() == KEPT_JOBS {
            jobs.pop_front();
        }
        jobs.push_back(job);
    }

    fn update(&self, id: &str, change: impl FnOnce(&mut Job)) {
        let mut jobs = self
            .jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(job) = jobs.iter_mut().find(|job| job.id == id) {
            change(job);
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
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use deckox_protocol::{CommandResult, CommandStatus, JobState};

    use super::{OperationSpec, Operations};
    use crate::{error::AgentError, events::EventBus};

    fn spec(key: &str) -> OperationSpec {
        OperationSpec {
            kind: "test_action",
            subject: "test".to_owned(),
            key: key.to_owned(),
            request_id: Some("req-1".to_owned()),
        }
    }

    fn done() -> CommandResult {
        CommandResult {
            command_id: "c".to_owned(),
            status: CommandStatus::Completed,
            message: Some("finished".to_owned()),
        }
    }

    async fn wait_for(operations: &Operations, id: &str, state: JobState) {
        for _ in 0..200 {
            if operations.job(id).is_some_and(|job| job.state == state) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job {id} never reached {state:?}");
    }

    #[tokio::test]
    async fn operations_on_one_resource_never_overlap() {
        let operations = Operations::new(EventBus::new());
        let running = std::sync::Arc::new(AtomicUsize::new(0));
        let overlaps = std::sync::Arc::new(AtomicUsize::new(0));

        let mut tasks = Vec::new();
        for _ in 0..8 {
            let (operations, running, overlaps) =
                (operations.clone(), running.clone(), overlaps.clone());
            tasks.push(tokio::spawn(async move {
                operations
                    .exclusive("packages", async {
                        if running.fetch_add(1, Ordering::SeqCst) > 0 {
                            overlaps.fetch_add(1, Ordering::SeqCst);
                        }
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        running.fetch_sub(1, Ordering::SeqCst);
                    })
                    .await;
            }));
        }
        for task in tasks {
            task.await.expect("task");
        }
        assert_eq!(overlaps.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn different_resources_run_side_by_side() {
        let operations = Operations::new(EventBus::new());
        let held = operations.clone();
        let blocker = tokio::spawn(async move {
            held.exclusive("packages", tokio::time::sleep(Duration::from_millis(200)))
                .await;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;

        let started = std::time::Instant::now();
        operations
            .exclusive("service:nginx.service", async {})
            .await;
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "an unrelated lock is not blocked"
        );
        blocker.await.expect("blocker");
    }

    #[tokio::test]
    async fn a_job_moves_from_queued_to_finished_and_announces_itself() {
        let events = EventBus::new();
        let operations = Operations::new(events.clone());
        let job = operations.submit(spec("packages"), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(done())
        });
        assert_eq!(job.state, JobState::Queued);

        wait_for(&operations, &job.id, JobState::Succeeded).await;
        let finished = operations.job(&job.id).expect("job kept");
        assert_eq!(finished.message.as_deref(), Some("finished"));
        assert!(finished.started_ms.is_some() && finished.finished_ms.is_some());

        let announced = events.since(None, 0).events;
        assert_eq!(announced.len(), 1);
        assert_eq!(announced[0].kind, "test_action");
        assert_eq!(announced[0].command_id.as_deref(), Some(job.id.as_str()));
    }

    #[tokio::test]
    async fn a_failing_job_records_the_error() {
        let operations = Operations::new(EventBus::new());
        let job = operations.submit(spec("packages"), async {
            Err(AgentError::conflict("package is locked"))
        });
        wait_for(&operations, &job.id, JobState::Failed).await;
        assert_eq!(
            operations
                .job(&job.id)
                .and_then(|job| job.message)
                .as_deref(),
            Some("package is locked")
        );
    }

    #[tokio::test]
    async fn a_second_job_waits_for_the_first_on_the_same_resource() {
        let operations = Operations::new(EventBus::new());
        let first = operations.submit(spec("packages"), async {
            tokio::time::sleep(Duration::from_millis(60)).await;
            Ok(done())
        });
        let second = operations.submit(spec("packages"), async { Ok(done()) });
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert_ne!(
            operations.job(&second.id).map(|job| job.state),
            Some(JobState::Succeeded),
            "the second job has not run yet"
        );
        wait_for(&operations, &first.id, JobState::Succeeded).await;
        wait_for(&operations, &second.id, JobState::Succeeded).await;
    }
}
