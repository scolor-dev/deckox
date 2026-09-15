//! Recurring, unattended execution of allow-listed service actions
//! (start/stop/restart), so an admin does not have to be present — or set up
//! their own systemd timer over SSH — to, say, restart a service nightly.
//! Schedules are persisted as JSON (an array of tables in TOML would need a
//! hand-rolled editor like `config::write_allowed_services`, which is not
//! worth it for a resource this dynamic) and evaluated by a background tick
//! loop that reuses [`ServiceManager::control`] — the exact same validated
//! path a manual button click takes, just triggered by a clock instead of an
//! HTTP request.

use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{Datelike, Local, Timelike};
use deckox_protocol::{CreateScheduleRequest, ScheduleAction, ServiceAction, ServiceSchedule};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{error::AgentError, services::ServiceManager};

const DEFAULT_SCHEDULES_PATH: &str = "/var/lib/deckox/schedules.json";
const MAX_SCHEDULES: usize = 100;
const TICK_INTERVAL: Duration = Duration::from_secs(20);
/// How many past minute-buckets a fired schedule is remembered for, so a
/// schedule seen due on several consecutive ticks within the same minute
/// fires only once. Comfortably larger than one minute of ticks.
const FIRED_MEMORY_MINUTES: i64 = 2;

#[derive(Clone)]
pub struct ScheduleStore {
    path: PathBuf,
    schedules: Arc<RwLock<Vec<ServiceSchedule>>>,
}

impl ScheduleStore {
    pub fn resolve_path() -> PathBuf {
        PathBuf::from(
            env::var("DECKOX_SCHEDULES_FILE").unwrap_or_else(|_| DEFAULT_SCHEDULES_PATH.to_owned()),
        )
    }

    pub async fn load(path: PathBuf) -> Result<Self, AgentError> {
        let schedules = match tokio::fs::read_to_string(&path).await {
            Ok(content) => serde_json::from_str(&content).map_err(|error| {
                AgentError::internal(format!(
                    "invalid schedules file {}: {error}",
                    path.display()
                ))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                return Err(AgentError::internal(format!(
                    "failed to read schedules file {}: {error}",
                    path.display()
                )));
            }
        };
        Ok(Self {
            path,
            schedules: Arc::new(RwLock::new(schedules)),
        })
    }

    pub async fn list(&self) -> Vec<ServiceSchedule> {
        self.schedules.read().await.clone()
    }

    pub async fn create(
        &self,
        request: CreateScheduleRequest,
        services: &ServiceManager,
    ) -> Result<ServiceSchedule, AgentError> {
        validate_time(request.hour, request.minute)?;
        let weekdays = normalize_weekdays(&request.weekdays)?;
        if !services.is_allowed(&request.service_id).await {
            return Err(AgentError::forbidden(format!(
                "service is not in the control allowlist: {}",
                request.service_id
            )));
        }

        let schedule = ServiceSchedule {
            id: new_id(),
            service_id: request.service_id,
            action: request.action,
            hour: request.hour,
            minute: request.minute,
            weekdays,
            enabled: true,
            created_at_ms: now_ms(),
            last_run_at_ms: None,
            last_result: None,
        };
        let snapshot = {
            let mut schedules = self.schedules.write().await;
            if schedules.len() >= MAX_SCHEDULES {
                return Err(AgentError::bad_request(format!(
                    "at most {MAX_SCHEDULES} schedules are supported"
                )));
            }
            schedules.push(schedule.clone());
            schedules.clone()
        };
        self.persist(&snapshot).await?;
        Ok(schedule)
    }

    pub async fn delete(&self, id: &str) -> Result<(), AgentError> {
        let snapshot = {
            let mut schedules = self.schedules.write().await;
            let before = schedules.len();
            schedules.retain(|schedule| schedule.id != id);
            if schedules.len() == before {
                return Err(AgentError::not_found("schedule not found"));
            }
            schedules.clone()
        };
        self.persist(&snapshot).await
    }

    pub async fn set_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<ServiceSchedule, AgentError> {
        let (updated, snapshot) = {
            let mut schedules = self.schedules.write().await;
            let schedule = schedules
                .iter_mut()
                .find(|schedule| schedule.id == id)
                .ok_or_else(|| AgentError::not_found("schedule not found"))?;
            schedule.enabled = enabled;
            (schedule.clone(), schedules.clone())
        };
        self.persist(&snapshot).await?;
        Ok(updated)
    }

    async fn record_run(&self, id: &str, result: String) {
        let snapshot = {
            let mut schedules = self.schedules.write().await;
            let Some(schedule) = schedules.iter_mut().find(|schedule| schedule.id == id) else {
                return;
            };
            schedule.last_run_at_ms = Some(now_ms());
            schedule.last_result = Some(result);
            schedules.clone()
        };
        if let Err(error) = self.persist(&snapshot).await {
            warn!(error = ?error, schedule_id = id, "failed to persist schedule run result");
        }
    }

    async fn persist(&self, schedules: &[ServiceSchedule]) -> Result<(), AgentError> {
        let encoded = serde_json::to_vec_pretty(schedules).map_err(|error| {
            AgentError::internal(format!("failed to encode schedules: {error}"))
        })?;
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|error| {
                AgentError::internal(format!("failed to create {}: {error}", parent.display()))
            })?;
        }
        let temp_path = self.path.with_extension("json.tmp");
        tokio::fs::write(&temp_path, encoded)
            .await
            .map_err(|error| {
                AgentError::internal(format!("failed to write {}: {error}", temp_path.display()))
            })?;
        tokio::fs::rename(&temp_path, &self.path)
            .await
            .map_err(|error| {
                AgentError::internal(format!(
                    "failed to replace {} with the updated schedules: {error}",
                    self.path.display()
                ))
            })
    }
}

/// Spawns the tick loop that fires due schedules. Cheap to run even with no
/// schedules configured, so it is always started rather than gated behind a
/// config flag like reboot/update.
pub fn spawn(store: ScheduleStore, services: ServiceManager) {
    tokio::spawn(async move {
        let mut fired: HashMap<String, i64> = HashMap::new();
        loop {
            tick(&store, &services, &mut fired).await;
            tokio::time::sleep(TICK_INTERVAL).await;
        }
    });
}

async fn tick(store: &ScheduleStore, services: &ServiceManager, fired: &mut HashMap<String, i64>) {
    let now = Local::now();
    let minute_bucket = now.timestamp() / 60;
    let weekday = iso_weekday(now.weekday());
    let hour = u8::try_from(now.hour()).unwrap_or(0);
    let minute = u8::try_from(now.minute()).unwrap_or(0);

    let due: Vec<ServiceSchedule> = store
        .list()
        .await
        .into_iter()
        .filter(|schedule| {
            schedule.enabled
                && schedule.hour == hour
                && schedule.minute == minute
                && schedule.weekdays.contains(&weekday)
                && fired.get(&schedule.id) != Some(&minute_bucket)
        })
        .collect();

    for schedule in due {
        fired.insert(schedule.id.clone(), minute_bucket);
        run_schedule(store, services, &schedule).await;
    }

    fired.retain(|_, bucket| minute_bucket - *bucket <= FIRED_MEMORY_MINUTES);
}

async fn run_schedule(
    store: &ScheduleStore,
    services: &ServiceManager,
    schedule: &ServiceSchedule,
) {
    let action = match schedule.action {
        ScheduleAction::Start => ServiceAction::Start,
        ScheduleAction::Stop => ServiceAction::Stop,
        ScheduleAction::Restart => ServiceAction::Restart,
    };
    let result = services.control(&schedule.service_id, action).await;
    match result {
        Ok(_) => {
            info!(
                schedule_id = schedule.id,
                service = schedule.service_id,
                "scheduled service action completed"
            );
            store.record_run(&schedule.id, "success".to_owned()).await;
        }
        Err(error) => {
            warn!(
                schedule_id = schedule.id,
                service = schedule.service_id,
                error = ?error,
                "scheduled service action failed"
            );
            store
                .record_run(&schedule.id, format!("failed: {}", error.message()))
                .await;
        }
    }
}

fn iso_weekday(weekday: chrono::Weekday) -> u8 {
    u8::try_from(weekday.number_from_monday()).unwrap_or(1)
}

fn validate_time(hour: u8, minute: u8) -> Result<(), AgentError> {
    if hour > 23 {
        return Err(AgentError::bad_request("hour must be between 0 and 23"));
    }
    if minute > 59 {
        return Err(AgentError::bad_request("minute must be between 0 and 59"));
    }
    Ok(())
}

fn normalize_weekdays(weekdays: &[u8]) -> Result<Vec<u8>, AgentError> {
    if weekdays.is_empty() {
        return Err(AgentError::bad_request("weekdays must not be empty"));
    }
    if weekdays.iter().any(|day| !(1..=7).contains(day)) {
        return Err(AgentError::bad_request(
            "weekdays must be between 1 (Monday) and 7 (Sunday)",
        ));
    }
    let mut normalized: Vec<u8> = weekdays.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    Ok(normalized)
}

fn new_id() -> String {
    format!("sched-{}", hex::encode(rand::random::<[u8; 8]>()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_time() {
        assert!(validate_time(23, 59).is_ok());
        assert!(validate_time(24, 0).is_err());
        assert!(validate_time(0, 60).is_err());
    }

    #[test]
    fn normalizes_and_deduplicates_weekdays() {
        assert_eq!(normalize_weekdays(&[3, 1, 1, 7]).unwrap(), vec![1, 3, 7]);
        assert!(normalize_weekdays(&[]).is_err());
        assert!(normalize_weekdays(&[0]).is_err());
        assert!(normalize_weekdays(&[8]).is_err());
    }

    #[tokio::test]
    async fn create_rejects_services_outside_the_allowlist() {
        let store = ScheduleStore::load(std::env::temp_dir().join(format!(
            "deckox-agent-schedules-test-{}.json",
            hex::encode(rand::random::<[u8; 8]>())
        )))
        .await
        .expect("load");
        let services =
            ServiceManager::new(Vec::new(), PathBuf::from("/tmp/deckox-agent-test.toml"))
                .expect("service manager");

        let result = store
            .create(
                CreateScheduleRequest {
                    service_id: "nginx.service".to_owned(),
                    action: ScheduleAction::Restart,
                    hour: 3,
                    minute: 0,
                    weekdays: vec![1],
                },
                &services,
            )
            .await;

        assert!(result.is_err());
        assert!(store.list().await.is_empty());
    }

    #[tokio::test]
    async fn create_list_and_delete_round_trip() {
        let path = std::env::temp_dir().join(format!(
            "deckox-agent-schedules-test-{}.json",
            hex::encode(rand::random::<[u8; 8]>())
        ));
        let store = ScheduleStore::load(path.clone()).await.expect("load");
        let services = ServiceManager::new(
            vec!["nginx.service".to_owned()],
            PathBuf::from("/tmp/deckox-agent-test.toml"),
        )
        .expect("service manager");

        let created = store
            .create(
                CreateScheduleRequest {
                    service_id: "nginx.service".to_owned(),
                    action: ScheduleAction::Restart,
                    hour: 3,
                    minute: 30,
                    weekdays: vec![1, 3, 5],
                },
                &services,
            )
            .await
            .expect("create");

        assert_eq!(store.list().await.len(), 1);
        assert!(created.enabled);

        let disabled = store
            .set_enabled(&created.id, false)
            .await
            .expect("disable");
        assert!(!disabled.enabled);

        store.delete(&created.id).await.expect("delete");
        assert!(store.list().await.is_empty());
        assert!(store.delete(&created.id).await.is_err());

        let _ = tokio::fs::remove_file(&path).await;
    }
}
