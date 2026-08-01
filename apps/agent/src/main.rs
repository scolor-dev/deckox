use std::{
    env, fs,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::Path,
};

use axum::{
    Extension, Json, Router,
    extract::{Path as AxumPath, State},
    middleware,
    routing::{delete, get, post},
};
use deckox_protocol::{
    AddSshKeyRequest, AgentStatus, CommandResult, HealthResponse, ServiceAction, ServiceDetails,
    ServiceSummary, SshKeyList, SshKeySummary, StorageMount, SystemCapabilities, SystemInfo,
    SystemMetrics,
};
use tokio::net::UnixListener;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    config::AgentConfig, error::AgentError, power::PowerManager, services::ServiceManager,
    ssh_keys::SshKeyManager, storage::read_storage, system::read_system_info,
};

mod config;
mod error;
mod power;
mod request_context;
mod services;
mod ssh_keys;
mod storage;
mod system;

#[derive(Clone)]
struct AppState {
    power: PowerManager,
    services: ServiceManager,
    ssh_keys: SshKeyManager,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let config = AgentConfig::load().unwrap_or_else(|error| {
        eprintln!("failed to load Agent configuration: {error:?}");
        std::process::exit(2);
    });
    let socket_path = config.socket_path();
    let power = PowerManager::new(config.system.allow_reboot);
    let services = ServiceManager::new(config.services.allowed).unwrap_or_else(|error| {
        eprintln!("invalid service control configuration: {error:?}");
        std::process::exit(2);
    });
    let ssh_keys = SshKeyManager::new(config.ssh.managed_user).unwrap_or_else(|error| {
        eprintln!("invalid SSH key management configuration: {error:?}");
        std::process::exit(2);
    });

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
        .route("/v1/system", get(system_info))
        .route("/v1/system/capabilities", get(system_capabilities))
        .route("/v1/system/reboot", post(reboot_system))
        .route("/v1/system/metrics", get(system_metrics))
        .route("/v1/storage", get(storage))
        .route("/v1/services", get(list_services))
        .route("/v1/services/{service_id}", get(service_details))
        .route("/v1/services/{service_id}/start", post(start_service))
        .route("/v1/services/{service_id}/stop", post(stop_service))
        .route("/v1/services/{service_id}/restart", post(restart_service))
        .route("/v1/ssh/keys", get(list_ssh_keys).post(add_ssh_key))
        .route("/v1/ssh/keys/{key_id}", delete(remove_ssh_key))
        .with_state(AppState {
            power,
            services,
            ssh_keys,
        })
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

async fn system_info() -> Result<Json<SystemInfo>, AgentError> {
    read_system_info().await.map(Json)
}

async fn system_capabilities(State(state): State<AppState>) -> Json<SystemCapabilities> {
    Json(state.power.capabilities())
}

async fn reboot_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<CommandResult>, AgentError> {
    let result = state.power.reboot().await;
    match &result {
        Ok(command) => info!(
            event = "system_reboot",
            request_id = %request_id.0,
            command_id = %command.command_id,
            result = "accepted",
            "system reboot accepted"
        ),
        Err(error) => warn!(
            event = "system_reboot",
            request_id = %request_id.0,
            error = ?error,
            result = "rejected",
            "system reboot rejected"
        ),
    }
    result.map(Json)
}

async fn system_metrics() -> Result<Json<SystemMetrics>, AgentError> {
    system::read_system_metrics().await.map(Json)
}

async fn storage() -> Result<Json<Vec<StorageMount>>, AgentError> {
    read_storage().await.map(Json)
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
    };
    let result = state.services.control(&service_id, action).await;

    match &result {
        Ok(command_result) => info!(
            event = "service_action",
            request_id = %request_id.0,
            service = %service_id,
            action = action_name,
            status = ?command_result.status,
            "service action completed"
        ),
        Err(error) => warn!(
            event = "service_action",
            request_id = %request_id.0,
            service = %service_id,
            action = action_name,
            error = ?error,
            "service action rejected"
        ),
    }

    result.map(Json)
}

async fn list_ssh_keys(State(state): State<AppState>) -> Result<Json<SshKeyList>, AgentError> {
    state.ssh_keys.list().await.map(Json)
}

async fn add_ssh_key(
    State(state): State<AppState>,
    Extension(request_id): Extension<request_context::RequestId>,
    Json(payload): Json<AddSshKeyRequest>,
) -> Result<Json<SshKeySummary>, AgentError> {
    let result = state.ssh_keys.add(&payload.public_key).await;
    match &result {
        Ok(key) => info!(
            event = "ssh_key_add",
            request_id = %request_id.0,
            fingerprint = %key.fingerprint,
            result = "success",
            "SSH public key added"
        ),
        Err(error) => warn!(
            event = "ssh_key_add",
            request_id = %request_id.0,
            error = ?error,
            result = "failure",
            "SSH public key addition rejected"
        ),
    }
    result.map(Json)
}

async fn remove_ssh_key(
    State(state): State<AppState>,
    AxumPath(key_id): AxumPath<String>,
    Extension(request_id): Extension<request_context::RequestId>,
) -> Result<Json<SshKeySummary>, AgentError> {
    let result = state.ssh_keys.remove(&key_id).await;
    match &result {
        Ok(key) => info!(
            event = "ssh_key_remove",
            request_id = %request_id.0,
            fingerprint = %key.fingerprint,
            result = "success",
            "SSH public key removed"
        ),
        Err(error) => warn!(
            event = "ssh_key_remove",
            request_id = %request_id.0,
            key_id,
            error = ?error,
            result = "failure",
            "SSH public key removal rejected"
        ),
    }
    result.map(Json)
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
