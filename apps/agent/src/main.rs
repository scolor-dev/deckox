use std::{
    env, fs,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
};

use axum::{
    Extension, Json, Router,
    extract::{Path as AxumPath, Query, State},
    middleware,
    routing::{get, post},
};
use deckox_protocol::{
    AgentDiagnostics, AgentStatus, AgentUpdateRequest, BackupSummary, CommandResult,
    CreateScheduleRequest, HealthResponse, RuntimeConfigSummary, ServiceAction, ServiceDetails,
    ServiceLogPriority, ServiceLogs, ServiceSchedule, ServiceSummary, SoftwarePackage,
    StorageMount, SystemCapabilities, SystemInfo, SystemMetrics,
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
mod config_cli;
mod diagnostics;
mod error;
mod power;
mod request_context;
mod schedules;
mod services;
mod software;
mod storage;
mod system;
mod update;

#[derive(Clone)]
struct AppState {
    power: PowerManager,
    services: ServiceManager,
    software: SoftwareManager,
    update: UpdateManager,
    runtime_config: RuntimeConfigSummary,
    schedules: ScheduleStore,
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
    schedules::spawn(schedule_store.clone(), services.clone());

    (
        AppState {
            power,
            services,
            software,
            update,
            runtime_config,
            schedules: schedule_store,
        },
        socket_path,
    )
}

#[tokio::main]
async fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if config_cli::dispatch(&arguments) {
        return;
    }

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

/// Logs the outcome of an Agent-side command and passes the result through
/// unchanged. `event` names the operation for log filtering; `detail` carries
/// whatever per-call context matters (empty when there is none) as a single
/// field, since tracing's macros need a fixed field set at each call site
/// and the three operations that use this (reboot, self-update, service
/// actions) each carry different extra context.
fn log_command_result(
    event: &'static str,
    request_id: &request_context::RequestId,
    detail: &str,
    result: Result<CommandResult, AgentError>,
) -> Result<Json<CommandResult>, AgentError> {
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
) -> Result<Json<CommandResult>, AgentError> {
    log_command_result("system_reboot", &request_id, "", state.power.reboot().await)
}

async fn update_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<request_context::RequestId>,
    Json(payload): Json<AgentUpdateRequest>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state
        .update
        .trigger(&payload.target_version, &payload.install_script)
        .await;
    log_command_result(
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
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    control_service(state, service_id, ServiceAction::Start, request_id).await
}

async fn stop_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    control_service(state, service_id, ServiceAction::Stop, request_id).await
}

async fn restart_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    control_service(state, service_id, ServiceAction::Restart, request_id).await
}

async fn enable_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    control_service(state, service_id, ServiceAction::Enable, request_id).await
}

async fn disable_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    control_service(state, service_id, ServiceAction::Disable, request_id).await
}

async fn allow_service(
    State(state): State<AppState>,
    AxumPath(service_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.services.allow(&service_id).await;
    log_command_result(
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
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.services.disallow(&service_id).await;
    log_command_result(
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

async fn install_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.software.install(&software_id).await;
    log_command_result(
        "software_action",
        &request_id,
        &format!("software={software_id} action=install"),
        result,
    )
}

async fn remove_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.software.remove(&software_id).await;
    log_command_result(
        "software_action",
        &request_id,
        &format!("software={software_id} action=remove"),
        result,
    )
}

async fn upgrade_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.software.upgrade(&software_id).await;
    log_command_result(
        "software_action",
        &request_id,
        &format!("software={software_id} action=upgrade"),
        result,
    )
}

async fn allow_software(
    State(state): State<AppState>,
    AxumPath(software_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.software.allow(&software_id).await;
    log_command_result(
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
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.software.disallow(&software_id).await;
    log_command_result(
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
) -> Result<(), AgentError> {
    state.schedules.delete(&schedule_id).await
}

async fn enable_schedule(
    State(state): State<AppState>,
    AxumPath(schedule_id): AxumPath<String>,
) -> Result<Json<ServiceSchedule>, AgentError> {
    state
        .schedules
        .set_enabled(&schedule_id, true)
        .await
        .map(Json)
}

async fn disable_schedule(
    State(state): State<AppState>,
    AxumPath(schedule_id): AxumPath<String>,
) -> Result<Json<ServiceSchedule>, AgentError> {
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
    request_id: request_context::RequestId,
) -> Result<Json<CommandResult>, AgentError> {
    let action_name = match &action {
        ServiceAction::Start => "start",
        ServiceAction::Stop => "stop",
        ServiceAction::Restart => "restart",
        ServiceAction::Enable => "enable",
        ServiceAction::Disable => "disable",
    };
    let result = state.services.control(&service_id, action).await;
    log_command_result(
        "service_action",
        &request_id,
        &format!("service={service_id} action={action_name}"),
        result,
    )
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
