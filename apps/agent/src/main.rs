use std::{
    env, fs,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
};

use axum::{
    Extension, Json, Router,
    extract::{Path as AxumPath, Query, State},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use deckox_protocol::{
    AgentDiagnostics, AgentInfo, AgentStatus, AgentUpdateRequest, BackupSummary, CommandResult,
    CreateScheduleRequest, EventBatch, EventResult, HealthResponse, PROTOCOL_VERSION,
    RuntimeConfigSummary, ServiceAction, ServiceDetails, ServiceLogPriority, ServiceLogs,
    ServiceSchedule, ServiceSummary, SoftwarePackage, StepUp, StorageMount, SystemCapabilities,
    SystemInfo, SystemMetrics,
};
use serde::Deserialize;
use tokio::net::UnixListener;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    config::AgentConfig,
    error::AgentError,
    power::PowerManager,
    schedules::ScheduleStore,
    services::ServiceManager,
    software::SoftwareManager,
    storage::{read_disks, read_storage},
    system::read_system_info,
    update::UpdateManager,
};

mod backups;
mod config;
mod diagnostics;
mod error;
mod events;
mod modules;
mod operations;
mod power;
mod request_context;
mod schedules;
mod services;
mod software;
mod storage;
mod system;
mod update;
mod verifier;

#[derive(Clone)]
struct AppState {
    power: PowerManager,
    services: ServiceManager,
    software: SoftwareManager,
    update: UpdateManager,
    runtime_config: RuntimeConfigSummary,
    schedules: ScheduleStore,
    events: events::EventBus,
    operations: operations::Operations,
    modules: modules::ModuleRegistry,
    verifier: verifier::Verifier,
    confirm: std::sync::Arc<std::collections::HashSet<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct OperationQuery {
    /// `?async=true` returns a job at once instead of waiting for the result.
    #[serde(default, rename = "async")]
    run_async: bool,
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    #[serde(default)]
    after: u64,
    epoch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServiceLogsQuery {
    #[serde(default = "default_log_lines")]
    lines: u16,
    #[serde(default = "default_log_priority")]
    priority: ServiceLogPriority,
}

const fn default_log_lines() -> u16 {
    100
}

const fn default_log_priority() -> ServiceLogPriority {
    ServiceLogPriority::All
}

/// Loads configuration and assembles every long-lived manager `main` wires
/// into the router's shared state. Split out so `main` itself stays under
/// Clippy's line-count limit.
async fn build_state() -> (AppState, PathBuf) {
    let config = AgentConfig::load().unwrap_or_else(|error| {
        eprintln!("failed to load Agent configuration: {error:?}");
        std::process::exit(2);
    });
    let socket_path = config.socket_path();
    let runtime_config = RuntimeConfigSummary {
        reboot_allowed: config.system.allow_reboot,
        update_allowed: config.system.allow_update,
        allowed_services_count: config.services.allowed.len(),
    };
    let power = PowerManager::new(config.system.allow_reboot);
    let runtime_dir = socket_path
        .parent()
        .map_or_else(|| PathBuf::from("/run/deckox"), Path::to_path_buf);
    let update = UpdateManager::new(config.system.allow_update, &runtime_dir);
    let services = ServiceManager::new(config.services.allowed, AgentConfig::resolve_path())
        .unwrap_or_else(|error| {
            eprintln!("invalid service control configuration: {error:?}");
            std::process::exit(2);
        });
    let package_manager = software::detect_package_manager().await;
    let software = SoftwareManager::new(
        config.software,
        AgentConfig::resolve_path(),
        package_manager,
    )
    .unwrap_or_else(|error| {
        eprintln!("invalid software management configuration: {error:?}");
        std::process::exit(2);
    });
    let schedule_store = ScheduleStore::load(ScheduleStore::resolve_path())
        .await
        .unwrap_or_else(|error| {
            eprintln!("failed to load schedules: {error:?}");
            std::process::exit(2);
        });
    let module_registry =
        modules::ModuleRegistry::new(&config.modules.disabled).unwrap_or_else(|error| {
            eprintln!("invalid [modules] configuration: {error}");
            std::process::exit(2);
        });
    let confirm = validated_confirm(&config.security.confirm).unwrap_or_else(|error| {
        eprintln!("invalid [security] configuration: {error}");
        std::process::exit(2);
    });
    let events = events::EventBus::new();
    let operations = operations::Operations::new(events.clone());
    if module_registry.is_enabled("schedules") {
        schedules::spawn(
            schedule_store.clone(),
            services.clone(),
            events.clone(),
            operations.clone(),
        );
    }

    (
        AppState {
            power,
            services,
            software,
            update,
            runtime_config,
            schedules: schedule_store,
            events,
            operations,
            modules: module_registry,
            verifier: verifier::Verifier::from_environment(),
            confirm: std::sync::Arc::new(confirm),
        },
        socket_path,
    )
}

#[tokio::main]
async fn main() {
    init_tracing();

    let (state, socket_path) = build_state().await;

    if let Err(error) = prepare_socket(&socket_path) {
        error!(%error, path = %socket_path.display(), "failed to prepare agent socket");
        std::process::exit(1);
    }

    let listener = UnixListener::bind(&socket_path).unwrap_or_else(|error| {
        eprintln!("failed to bind {}: {error}", socket_path.display());
        std::process::exit(1);
    });

    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o660)).unwrap_or_else(|error| {
        eprintln!(
            "failed to set permissions on {}: {error}",
            socket_path.display()
        );
        std::process::exit(1);
    });

    let app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/info", get(agent_info))
        .route("/v1/events", get(agent_events))
        .route("/v1/security", get(security_status))
        .route("/v1/auth/verifier", post(replace_verifier))
        .route("/v1/modules", get(list_modules))
        .route("/v1/jobs", get(list_jobs))
        .route("/v1/jobs/{job_id}", get(get_job))
        .route("/v1/status", get(agent_status))
        .route("/v1/diagnostics", get(agent_diagnostics))
        .route("/v1/system", get(system_info))
        .route("/v1/system/capabilities", get(system_capabilities))
        .route("/v1/system/reboot", post(reboot_system))
        .route("/v1/system/update", post(update_system))
        .route("/v1/system/metrics", get(system_metrics))
        .route("/v1/storage", get(storage))
        .route("/v1/storage/disks", get(storage_disks))
        .route("/v1/backups", get(list_backups))
        .route("/v1/services", get(list_services))
        .route("/v1/services/{service_id}", get(service_details))
        .route("/v1/services/{service_id}/start", post(start_service))
        .route("/v1/services/{service_id}/stop", post(stop_service))
        .route("/v1/services/{service_id}/restart", post(restart_service))
        .route("/v1/services/{service_id}/enable", post(enable_service))
        .route("/v1/services/{service_id}/disable", post(disable_service))
        .route("/v1/services/{service_id}/allow", post(allow_service))
        .route("/v1/services/{service_id}/disallow", post(disallow_service))
        .route("/v1/services/{service_id}/logs", get(service_logs))
        .route("/v1/software", get(list_software))
        .route("/v1/software/installed", get(list_installed_software))
        .route("/v1/software/{software_id}/install", post(install_software))
        .route("/v1/software/{software_id}/remove", post(remove_software))
        .route("/v1/software/{software_id}/upgrade", post(upgrade_software))
        .route("/v1/software/{software_id}/allow", post(allow_software))
        .route(
            "/v1/software/{software_id}/disallow",
            post(disallow_software),
        )
        .route("/v1/schedules", get(list_schedules).post(create_schedule))
        .route(
            "/v1/schedules/{schedule_id}",
            axum::routing::delete(delete_schedule),
        )
        .route("/v1/schedules/{schedule_id}/enable", post(enable_schedule))
        .route(
            "/v1/schedules/{schedule_id}/disable",
            post(disable_schedule),
        )
        .layer(middleware::from_fn_with_state(
            state.modules.clone(),
            modules::gate,
        ))
        .with_state(state)
        .layer(middleware::from_fn(request_context::assign_request_id));

    info!(path = %socket_path.display(), "deckox agent started");

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    if let Err(error) = result {
        error!(%error, "agent stopped unexpectedly");
    }

    if let Err(error) = fs::remove_file(&socket_path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        error!(%error, path = %socket_path.display(), "failed to remove agent socket");
    }
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("deckox_agent=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

fn prepare_socket(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => fs::remove_file(path),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("refusing to replace non-socket path {}", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
    })
}

async fn agent_status() -> Json<AgentStatus> {
    let hostname = tokio::fs::read_to_string("/etc/hostname")
        .await
        .map_or_else(|_| "unknown".to_owned(), |value| value.trim().to_owned());

    let uptime_seconds = tokio::fs::read_to_string("/proc/uptime")
        .await
        .ok()
        .and_then(|value| system::parse_uptime(&value).ok());

    Json(AgentStatus {
        status: "running".to_owned(),
        hostname,
        operating_system: env::consts::OS.to_owned(),
        architecture: env::consts::ARCH.to_owned(),
        uptime_seconds,
    })
}

async fn agent_diagnostics(
    State(state): State<AppState>,
) -> Result<Json<AgentDiagnostics>, AgentError> {
    diagnostics::read_diagnostics(state.runtime_config)
        .await
        .map(Json)
}

async fn system_info() -> Result<Json<SystemInfo>, AgentError> {
    read_system_info().await.map(Json)
}

async fn system_capabilities(State(state): State<AppState>) -> Json<SystemCapabilities> {
    Json(SystemCapabilities {
        reboot_allowed: state.power.reboot_allowed(),
        update_allowed: state.update.allowed(),
    })
}

/// The operations whose requests can be required to carry the admin password.
const CONFIRMABLE: &[&str] = &[
    "software_action",
    "system_reboot",
    "system_update",
    "service_allowlist",
    "software_allowlist",
    "service_stop",
    "schedule_change",
];

fn validated_confirm(names: &[String]) -> Result<std::collections::HashSet<String>, String> {
    for name in names {
        if !CONFIRMABLE.contains(&name.as_str()) {
            return Err(format!(
                "unknown operation \"{name}\" in [security] confirm (known: {})",
                CONFIRMABLE.join(", ")
            ));
        }
    }
    Ok(names.iter().cloned().collect())
}

/// Refuses `operation` unless it carries the admin password, when the Agent is
/// set to require one for it. Refusals are recorded as events.
async fn authorize(
    state: &AppState,
    operation: &'static str,
    password: Option<&str>,
) -> Result<(), AgentError> {
    if !state.confirm.contains(operation) {
        return Ok(());
    }
    let outcome = match password {
        Some(password) => state.verifier.verify(password).await,
        None => Err(AgentError::unauthorized(
            "password_required",
            "this operation needs the admin password",
        )),
    };
    if let Err(error) = &outcome {
        state.events.publish(events::EventDraft {
            kind: "password_check",
            result: EventResult::Rejected,
            subject: format!("operation={operation}"),
            request_id: None,
            command_id: None,
            message: Some(error.message().to_owned()),
        });
    }
    outcome
}

fn password_of(body: Option<&Json<StepUp>>) -> Option<&str> {
    body.and_then(|Json(step_up)| step_up.current_password.as_deref())
}

async fn security_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut confirm: Vec<&String> = state.confirm.iter().collect();
    confirm.sort();
    Json(serde_json::json!({
        "verifier_provisioned": state.verifier.is_provisioned(),
        "locked_for_seconds": state.verifier.locked_for(),
        "confirm": confirm,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct ReplaceVerifierRequest {
    current_password: String,
    new_password: String,
}

/// Called by the Server when the admin changes the password in the Web UI,
/// so the Agent's copy follows. The old password must check out first.
async fn replace_verifier(
    State(state): State<AppState>,
    Json(request): Json<ReplaceVerifierRequest>,
) -> Result<Json<CommandResult>, AgentError> {
    state
        .verifier
        .replace(&request.current_password, &request.new_password)
        .await?;
    state.events.publish(events::EventDraft {
        kind: "password_change",
        result: EventResult::Completed,
        subject: "admin password".to_owned(),
        request_id: None,
        command_id: None,
        message: None,
    });
    Ok(Json(CommandResult {
        command_id: "verifier".to_owned(),
        status: deckox_protocol::CommandStatus::Completed,
        message: None,
    }))
}

/// Runs a mutating operation under its resource lock. With `?async=true` it
/// starts a background job and answers `202` with the [`Job`](deckox_protocol::Job);
/// otherwise it waits and answers with the command result as before.
async fn run_operation(
    state: &AppState,
    spec: operations::OperationSpec,
    run_async: bool,
    work: impl std::future::Future<Output = Result<CommandResult, AgentError>> + Send + 'static,
) -> Result<axum::response::Response, AgentError> {
    if run_async {
        let job = state.operations.submit(spec, work);
        return Ok((axum::http::StatusCode::ACCEPTED, Json(job)).into_response());
    }
    let result = state.operations.exclusive(&spec.key, work).await;
    let request_id = request_context::RequestId(spec.request_id.unwrap_or_default());
    log_command_result(&state.events, spec.kind, &request_id, &spec.subject, result)
        .map(IntoResponse::into_response)
}

async fn list_modules(State(state): State<AppState>) -> Json<deckox_protocol::ModuleManifest> {
    Json(state.modules.manifest())
}

async fn list_jobs(State(state): State<AppState>) -> Json<Vec<deckox_protocol::Job>> {
    Json(state.operations.jobs())
}

async fn get_job(
    State(state): State<AppState>,
    AxumPath(job_id): AxumPath<String>,
) -> Result<Json<deckox_protocol::Job>, AgentError> {
    state
        .operations
        .job(&job_id)
        .map(Json)
        .ok_or_else(|| AgentError::not_found(format!("job not found: {job_id}")))
}

/// Logs the outcome of an Agent-side command and passes the result through
/// unchanged. `event` names the operation for log filtering; `detail` carries
/// whatever per-call context matters (empty when there is none) as a single
/// field, since tracing's macros need a fixed field set at each call site
/// and the three operations that use this (reboot, self-update, service
/// actions) each carry different extra context.
fn log_command_result(
    events: &events::EventBus,
    event: &'static str,
    request_id: &request_context::RequestId,
    detail: &str,
    result: Result<CommandResult, AgentError>,
) -> Result<Json<CommandResult>, AgentError> {
    events.publish(events::EventDraft {
        kind: event,
        result: if result.is_ok() {
            EventResult::Accepted
        } else {
            EventResult::Rejected
        },
        subject: detail.to_owned(),
        request_id: Some(request_id.0.clone()),
        command_id: result
            .as_ref()
            .ok()
            .map(|command| command.command_id.clone()),
        message: result
            .as_ref()
            .err()
            .map(|error| error.message().to_owned()),
    });
    match &result {
        Ok(command) => info!(
            event,
            request_id = %request_id.0,
            detail,
            command_id = %command.command_id,
            result = "accepted",
            "command accepted"
        ),
        Err(error) => warn!(
            event,
            request_id = %request_id.0,
            detail,
            error = ?error,
            result = "rejected",
            "command rejected"
        ),
    }
    result.map(Json)
}

async fn reboot_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<Json<CommandResult>, AgentError> {
    authorize(&state, "system_reboot", password_of(body.as_ref())).await?;
    let result = state
        .operations
        .exclusive("host", state.power.reboot())
        .await;
    log_command_result(&state.events, "system_reboot", &request_id, "", result)
}

async fn update_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<request_context::RequestId>,
    Json(payload): Json<AgentUpdateRequest>,
) -> Result<Json<CommandResult>, AgentError> {
    authorize(&state, "system_update", payload.current_password.as_deref()).await?;
    let result = state
        .operations
        .exclusive(
            "host",
            state
                .update
                .trigger(&payload.target_version, &payload.install_script),
        )
        .await;
    log_command_result(
        &state.events,
        "system_update",
        &request_id,
        &format!("target_version={}", payload.target_version),
        result,
    )
}

async fn system_metrics() -> Result<Json<SystemMetrics>, AgentError> {
    system::read_system_metrics().await.map(Json)
}

async fn storage() -> Result<Json<Vec<StorageMount>>, AgentError> {
    read_storage().await.map(Json)
}

async fn storage_disks() -> Result<Json<Vec<deckox_protocol::StorageDisk>>, AgentError> {
    read_disks().await.map(Json)
}

async fn list_backups() -> Result<Json<Vec<BackupSummary>>, AgentError> {
    backups::list_backups().await.map(Json)
}

async fn list_services(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServiceSummary>>, AgentError> {
    state.services.list().await.map(Json)
}

async fn service_details(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
) -> Result<Json<ServiceDetails>, AgentError> {
    state.services.details(&service_id).await.map(Json)
}

async fn start_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    control_service(
        state,
        service_id,
        ServiceAction::Start,
        operation.run_async,
        request_id,
        password_of(body.as_ref()),
    )
    .await
}

async fn stop_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    control_service(
        state,
        service_id,
        ServiceAction::Stop,
        operation.run_async,
        request_id,
        password_of(body.as_ref()),
    )
    .await
}

async fn restart_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    control_service(
        state,
        service_id,
        ServiceAction::Restart,
        operation.run_async,
        request_id,
        password_of(body.as_ref()),
    )
    .await
}

async fn enable_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    control_service(
        state,
        service_id,
        ServiceAction::Enable,
        operation.run_async,
        request_id,
        password_of(body.as_ref()),
    )
    .await
}

async fn disable_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    control_service(
        state,
        service_id,
        ServiceAction::Disable,
        operation.run_async,
        request_id,
        password_of(body.as_ref()),
    )
    .await
}

async fn allow_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<Json<CommandResult>, AgentError> {
    authorize(&state, "service_allowlist", password_of(body.as_ref())).await?;
    let result = state.services.allow(&service_id).await;
    log_command_result(
        &state.events,
        "service_allowlist",
        &request_id,
        &format!("service={service_id} action=allow"),
        result,
    )
}

async fn disallow_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<Json<CommandResult>, AgentError> {
    authorize(&state, "service_allowlist", password_of(body.as_ref())).await?;
    let result = state.services.disallow(&service_id).await;
    log_command_result(
        &state.events,
        "service_allowlist",
        &request_id,
        &format!("service={service_id} action=disallow"),
        result,
    )
}

async fn list_software(
    State(state): State<AppState>,
) -> Result<Json<Vec<SoftwarePackage>>, AgentError> {
    state.software.list().await.map(Json)
}

async fn list_installed_software(
    State(state): State<AppState>,
) -> Result<Json<Vec<deckox_protocol::InstalledSoftware>>, AgentError> {
    state.software.list_installed().await.map(Json)
}

async fn agent_info(State(state): State<AppState>) -> Json<AgentInfo> {
    Json(AgentInfo {
        protocol_version: PROTOCOL_VERSION,
        agent_version: env!("CARGO_PKG_VERSION").to_owned(),
        epoch: state.events.epoch().to_owned(),
    })
}

async fn agent_events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Json<EventBatch> {
    Json(state.events.since(query.epoch.as_deref(), query.after))
}

async fn install_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    authorize(&state, "software_action", password_of(body.as_ref())).await?;
    let software = state.software.clone();
    let name = software_id.clone();
    run_operation(
        &state,
        operations::OperationSpec {
            kind: "software_action",
            subject: format!("software={software_id} action=install"),
            key: "packages".to_owned(),
            request_id: Some(request_id.0),
        },
        operation.run_async,
        async move { software.install(&name).await },
    )
    .await
}

async fn remove_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    authorize(&state, "software_action", password_of(body.as_ref())).await?;
    let software = state.software.clone();
    let name = software_id.clone();
    run_operation(
        &state,
        operations::OperationSpec {
            kind: "software_action",
            subject: format!("software={software_id} action=remove"),
            key: "packages".to_owned(),
            request_id: Some(request_id.0),
        },
        operation.run_async,
        async move { software.remove(&name).await },
    )
    .await
}

async fn upgrade_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Query(operation): Query<OperationQuery>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<axum::response::Response, AgentError> {
    authorize(&state, "software_action", password_of(body.as_ref())).await?;
    let software = state.software.clone();
    let name = software_id.clone();
    run_operation(
        &state,
        operations::OperationSpec {
            kind: "software_action",
            subject: format!("software={software_id} action=upgrade"),
            key: "packages".to_owned(),
            request_id: Some(request_id.0),
        },
        operation.run_async,
        async move { software.upgrade(&name).await },
    )
    .await
}

async fn allow_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<Json<CommandResult>, AgentError> {
    authorize(&state, "software_allowlist", password_of(body.as_ref())).await?;
    let result = state.software.allow(&software_id).await;
    log_command_result(
        &state.events,
        "software_allowlist",
        &request_id,
        &format!("software={software_id} action=allow"),
        result,
    )
}

async fn disallow_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
    body: Option<Json<StepUp>>,
) -> Result<Json<CommandResult>, AgentError> {
    authorize(&state, "software_allowlist", password_of(body.as_ref())).await?;
    let result = state.software.disallow(&software_id).await;
    log_command_result(
        &state.events,
        "software_allowlist",
        &request_id,
        &format!("software={software_id} action=disallow"),
        result,
    )
}

async fn list_schedules(State(state): State<AppState>) -> Json<Vec<ServiceSchedule>> {
    Json(state.schedules.list().await)
}

async fn create_schedule(
    State(state): State<AppState>,
    Extension(request_id): Extension<request_context::RequestId>,
    Json(payload): Json<CreateScheduleRequest>,
) -> Result<Json<ServiceSchedule>, AgentError> {
    authorize(
        &state,
        "schedule_change",
        payload.current_password.as_deref(),
    )
    .await?;
    let detail = format!(
        "service={} action={:?} hour={} minute={}",
        payload.service_id, payload.action, payload.hour, payload.minute
    );
    let result = state.schedules.create(payload, &state.services).await;
    match &result {
        Ok(schedule) => info!(
            event = "schedule_create",
            request_id = %request_id.0,
            detail,
            schedule_id = %schedule.id,
            result = "accepted",
            "schedule created"
        ),
        Err(error) => warn!(
            event = "schedule_create",
            request_id = %request_id.0,
            detail,
            error = ?error,
            result = "rejected",
            "schedule rejected"
        ),
    }
    result.map(Json)
}

async fn delete_schedule(
    State(state): State<AppState>,
    AxumPath(schedule_id): AxumPath<String>,
    body: Option<Json<StepUp>>,
) -> Result<(), AgentError> {
    authorize(&state, "schedule_change", password_of(body.as_ref())).await?;
    state.schedules.delete(&schedule_id).await
}

async fn enable_schedule(
    State(state): State<AppState>,
    AxumPath(schedule_id): AxumPath<String>,
    body: Option<Json<StepUp>>,
) -> Result<Json<ServiceSchedule>, AgentError> {
    authorize(&state, "schedule_change", password_of(body.as_ref())).await?;
    state
        .schedules
        .set_enabled(&schedule_id, true)
        .await
        .map(Json)
}

async fn disable_schedule(
    State(state): State<AppState>,
    AxumPath(schedule_id): AxumPath<String>,
    body: Option<Json<StepUp>>,
) -> Result<Json<ServiceSchedule>, AgentError> {
    authorize(&state, "schedule_change", password_of(body.as_ref())).await?;
    state
        .schedules
        .set_enabled(&schedule_id, false)
        .await
        .map(Json)
}

async fn service_logs(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Query(query): Query<ServiceLogsQuery>,
) -> Result<Json<ServiceLogs>, AgentError> {
    state
        .services
        .logs(&service_id, query.lines, query.priority)
        .await
        .map(Json)
}

async fn control_service(
    state: AppState,
    service_id: String,
    action: ServiceAction,
    run_async: bool,
    request_id: request_context::RequestId,
    password: Option<&str>,
) -> Result<axum::response::Response, AgentError> {
    if matches!(action, ServiceAction::Stop | ServiceAction::Disable) {
        authorize(&state, "service_stop", password).await?;
    }
    let action_name = match &action {
        ServiceAction::Start => "start",
        ServiceAction::Stop => "stop",
        ServiceAction::Restart => "restart",
        ServiceAction::Enable => "enable",
        ServiceAction::Disable => "disable",
    };
    let services = state.services.clone();
    let id = service_id.clone();
    run_operation(
        &state,
        operations::OperationSpec {
            kind: "service_action",
            subject: format!("service={service_id} action={action_name}"),
            key: format!("service:{service_id}"),
            request_id: Some(request_id.0),
        },
        run_async,
        async move { services.control(&id, action).await },
    )
    .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }

    info!("shutdown signal received");
}
