use std::{env, net::SocketAddr, path::PathBuf};

use axum::{
    Extension, Json, Router,
    extract::{FromRef, Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use deckox_protocol::{
    AgentStatus, AuditPage, DiagnosticsReport, ServiceLogPriority, UpdateStatus,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::agent_client::AgentClient;
use crate::{
    audit::AuditLog,
    auth::{AuthManager, AuthenticatedUser, PasswordConfirmationResult},
    metrics_stream::MetricsHub,
    request_context::RequestId,
};

mod agent_client;
mod audit;
mod auth;
mod cli;
mod diagnostics;
mod fsutil;
mod metrics_stream;
mod request_context;
mod update;

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_AGENT_SOCKET: &str = "/run/deckox/agent.sock";
const DEFAULT_WEB_DIR: &str = "/usr/local/share/deckox/web";

#[derive(Clone)]
struct AppState {
    agent: AgentClient,
    auth: AuthManager,
    audit: AuditLog,
    metrics: MetricsHub,
    updates: update::UpdateChecker,
    instance_id: String,
}

impl FromRef<AppState> for AuthManager {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<AppState> for MetricsHub {
    fn from_ref(state: &AppState) -> Self {
        state.metrics.clone()
    }
}

#[derive(Serialize)]
struct ServerStatus {
    name: &'static str,
    version: &'static str,
    status: &'static str,
    agent: Option<AgentStatus>,
    agent_error: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}

/// Shared by every handler in this crate (including `auth.rs`, via
/// `crate::error_response`) that needs to return a JSON error body.
fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (
        status,
        Json(ErrorResponse {
            code,
            message: message.to_owned(),
        }),
    )
        .into_response()
}

#[derive(Serialize)]
struct ServerHealth {
    status: &'static str,
    instance_id: String,
}

#[derive(Deserialize)]
struct RebootRequest {
    current_password: String,
}

#[derive(Debug, Deserialize)]
struct ServiceLogsQuery {
    #[serde(default = "default_log_lines")]
    lines: u16,
    #[serde(default = "default_log_priority")]
    priority: ServiceLogPriority,
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    #[serde(default = "audit::default_limit")]
    limit: usize,
    before_ms: Option<u64>,
}

const fn default_log_lines() -> u16 {
    100
}

const fn default_log_priority() -> ServiceLogPriority {
    ServiceLogPriority::All
}

#[tokio::main]
async fn main() {
    if cli::dispatch(env::args().nth(1).as_deref()).await {
        return;
    }

    init_tracing();

    let listen_addr = env::var("DECKOX_LISTEN_ADDR")
        .unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_owned())
        .parse::<SocketAddr>()
        .unwrap_or_else(|error| {
            eprintln!("invalid DECKOX_LISTEN_ADDR: {error}");
            std::process::exit(2);
        });
    let web_dir =
        PathBuf::from(env::var("DECKOX_WEB_DIR").unwrap_or_else(|_| DEFAULT_WEB_DIR.to_owned()));
    let audit = AuditLog::from_env();
    let auth = AuthManager::load(audit.clone()).unwrap_or_else(|error| {
        eprintln!("failed to load authentication configuration: {error}");
        std::process::exit(2);
    });
    let agent = AgentClient::new(PathBuf::from(
        env::var("DECKOX_AGENT_SOCKET").unwrap_or_else(|_| DEFAULT_AGENT_SOCKET.to_owned()),
    ));
    let updates = load_update_checker();
    let state = AppState {
        agent: agent.clone(),
        auth: auth.clone(),
        audit,
        metrics: MetricsHub::new(agent),
        updates,
        instance_id: format!("{:016x}", rand::random::<u64>()),
    };

    let protected_api = Router::new()
        .route("/status", get(status))
        .route("/diagnostics", get(diagnostics))
        .route("/diagnostics/report", get(diagnostics_report))
        .route("/audit", get(audit_events))
        .route("/audit/report", get(audit_report))
        .route("/update", get(update_status))
        .route("/system", get(proxy_system))
        .route("/system/capabilities", get(proxy_system_capabilities))
        .route("/system/reboot", post(reboot_system))
        .route("/system/metrics", get(proxy_metrics))
        .route("/events/metrics", get(metrics_stream::metrics_events))
        .route("/storage", get(proxy_storage))
        .route("/services", get(proxy_services))
        .route("/services/{service_id}", get(proxy_service_details))
        .route("/services/{service_id}/start", post(proxy_start_service))
        .route("/services/{service_id}/stop", post(proxy_stop_service))
        .route(
            "/services/{service_id}/restart",
            post(proxy_restart_service),
        )
        .route("/services/{service_id}/enable", post(proxy_enable_service))
        .route(
            "/services/{service_id}/disable",
            post(proxy_disable_service),
        )
        .route("/services/{service_id}/logs", get(proxy_service_logs))
        .route("/auth/logout", post(auth::logout))
        .route("/settings/password", post(auth::change_password))
        .route("/settings/totp/status", get(auth::totp_status))
        .route("/settings/totp/setup", post(auth::totp_setup))
        .route("/settings/totp/confirm", post(auth::totp_confirm))
        .route("/settings/totp/disable", post(auth::totp_disable))
        .route_layer(middleware::from_fn_with_state(
            auth.clone(),
            auth::require_auth,
        ))
        .fallback(api_not_found);
    let public_api = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/login/totp", post(auth::login_totp))
        .route("/auth/session", get(auth::status))
        .merge(protected_api);
    let static_files =
        ServeDir::new(&web_dir).not_found_service(ServeFile::new(web_dir.join("index.html")));
    let app = Router::new()
        .route("/healthz", get(health))
        .nest("/api/v1", public_api)
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(request_context::assign_request_id))
        .with_state(state);

    let listener = TcpListener::bind(listen_addr)
        .await
        .unwrap_or_else(|error| {
            eprintln!("failed to bind {listen_addr}: {error}");
            std::process::exit(1);
        });

    info!(address = %listen_addr, web_dir = %web_dir.display(), "deckox server started");

    if let Err(error) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        error!(%error, "server stopped unexpectedly");
        std::process::exit(1);
    }
}

fn load_update_checker() -> update::UpdateChecker {
    update::UpdateChecker::new().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    })
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("deckox_server=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

async fn health(State(state): State<AppState>) -> Json<ServerHealth> {
    Json(ServerHealth {
        status: "ok",
        instance_id: state.instance_id,
    })
}

async fn status(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Json<ServerStatus> {
    match state
        .agent
        .get_json::<AgentStatus>("/v1/status", &request_id)
        .await
    {
        Ok(agent) => Json(ServerStatus {
            name: "deckox",
            version: env!("CARGO_PKG_VERSION"),
            status: "running",
            agent: Some(agent),
            agent_error: None,
        }),
        Err(error) => Json(ServerStatus {
            name: "deckox",
            version: env!("CARGO_PKG_VERSION"),
            status: "degraded",
            agent: None,
            agent_error: Some(error),
        }),
    }
}

async fn diagnostics(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Json<DiagnosticsReport> {
    Json(diagnostics::collect(&state.agent, &request_id).await)
}

async fn diagnostics_report(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    let report = diagnostics::collect(&state.agent, &request_id).await;
    diagnostics::attachment(&report)
}

async fn audit_events(
    State(state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> Json<AuditPage> {
    Json(state.audit.page(query.limit, query.before_ms).await)
}

async fn audit_report(State(state): State<AppState>) -> Response {
    let page = state.audit.export().await;
    audit::attachment(&page)
}

async fn update_status(State(state): State<AppState>) -> Json<UpdateStatus> {
    Json(state.updates.check().await)
}

async fn proxy_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/system", &request_id).await
}

async fn proxy_metrics(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/system/metrics", &request_id).await
}

async fn proxy_system_capabilities(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/system/capabilities", &request_id).await
}

async fn reboot_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<RebootRequest>,
) -> Response {
    reboot_host(&state, &request_id, &user, payload.current_password).await
}

/// Confirms the caller's password, proxies the reboot request to the Agent,
/// and records the outcome. Kept separate from the thin [`reboot_system`]
/// handler above, mirroring how [`proxy_service_request`] backs the service
/// action handlers.
async fn reboot_host(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    current_password: String,
) -> Response {
    match state
        .auth
        .confirm_current_password(user.source_ip, current_password)
        .await
    {
        PasswordConfirmationResult::Invalid => {
            state
                .audit
                .log_admin(
                    request_id,
                    user.source_ip,
                    "system_reboot",
                    "failure",
                    Some("invalid_password".to_owned()),
                    "system reboot confirmation failed",
                )
                .await;
            return error_response(
                StatusCode::UNAUTHORIZED,
                "invalid_current_password",
                "current password is incorrect",
            );
        }
        PasswordConfirmationResult::RateLimited => {
            state
                .audit
                .log_admin(
                    request_id,
                    user.source_ip,
                    "system_reboot",
                    "rate_limited",
                    None,
                    "system reboot confirmation rate limited",
                )
                .await;
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many password confirmation attempts",
            );
        }
        PasswordConfirmationResult::Confirmed => {}
    }

    let response = proxy_agent(&state.agent, "POST", "/v1/system/reboot", request_id).await;
    if response.status().is_success() {
        state
            .audit
            .log_admin(
                request_id,
                user.source_ip,
                "system_reboot",
                "accepted",
                None,
                "system reboot accepted",
            )
            .await;
    } else {
        state
            .audit
            .log_admin(
                request_id,
                user.source_ip,
                "system_reboot",
                "failure",
                Some(format!("status={}", response.status().as_u16())),
                "system reboot failed",
            )
            .await;
    }
    response
}

async fn proxy_storage(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/storage", &request_id).await
}

async fn proxy_services(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/services", &request_id).await
}

async fn proxy_service_details(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_service_request(
        &state.agent,
        &state.audit,
        "GET",
        &service_id,
        None,
        &request_id,
        None,
    )
    .await
}

async fn proxy_start_service(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_service_request(
        &state.agent,
        &state.audit,
        "POST",
        &service_id,
        Some("start"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_stop_service(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_service_request(
        &state.agent,
        &state.audit,
        "POST",
        &service_id,
        Some("stop"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_restart_service(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_service_request(
        &state.agent,
        &state.audit,
        "POST",
        &service_id,
        Some("restart"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_enable_service(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_service_request(
        &state.agent,
        &state.audit,
        "POST",
        &service_id,
        Some("enable"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_disable_service(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_service_request(
        &state.agent,
        &state.audit,
        "POST",
        &service_id,
        Some("disable"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_service_logs(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Query(query): Query<ServiceLogsQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    if !valid_service_id(&service_id) {
        return invalid_service_id();
    }
    if !valid_log_lines(query.lines) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "log lines must be one of 50, 100, 200, or 500",
        );
    }

    let priority = log_priority_name(query.priority);
    let path = format!(
        "/v1/services/{service_id}/logs?lines={}&priority={priority}",
        query.lines
    );
    proxy_agent(&state.agent, "GET", &path, &request_id).await
}

async fn proxy_service_request(
    client: &AgentClient,
    audit: &AuditLog,
    method: &str,
    service_id: &str,
    action: Option<&str>,
    request_id: &RequestId,
    user: Option<&AuthenticatedUser>,
) -> Response {
    if !valid_service_id(service_id) {
        return invalid_service_id();
    }

    let path = action.map_or_else(
        || format!("/v1/services/{service_id}"),
        |action| format!("/v1/services/{service_id}/{action}"),
    );
    let response = proxy_agent(client, method, &path, request_id).await;
    if let (Some(action), Some(user)) = (action, user) {
        if response.status().is_success() {
            audit
                .log_admin(
                    request_id,
                    user.source_ip,
                    "service_action",
                    "success",
                    Some(format!("service={service_id} action={action}")),
                    "service action completed",
                )
                .await;
        } else {
            audit
                .log_admin(
                    request_id,
                    user.source_ip,
                    "service_action",
                    "failure",
                    Some(format!(
                        "service={service_id} action={action} status={}",
                        response.status().as_u16()
                    )),
                    "service action failed",
                )
                .await;
        }
    }
    response
}

fn invalid_service_id() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "bad_request",
        "invalid systemd service id",
    )
}

const fn valid_log_lines(lines: u16) -> bool {
    matches!(lines, 50 | 100 | 200 | 500)
}

const fn log_priority_name(priority: ServiceLogPriority) -> &'static str {
    match priority {
        ServiceLogPriority::All => "all",
        ServiceLogPriority::Error => "error",
        ServiceLogPriority::Warning => "warning",
        ServiceLogPriority::Info => "info",
    }
}

async fn proxy_agent(
    client: &AgentClient,
    method: &str,
    path: &str,
    request_id: &RequestId,
) -> Response {
    match client.request(method, path, request_id).await {
        Ok(response) => (response.status, Json(response.body)).into_response(),
        Err(message) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                code: "agent_unavailable",
                message,
            }),
        )
            .into_response(),
    }
}

fn valid_service_id(service_id: &str) -> bool {
    !service_id.is_empty()
        && service_id.len() <= 256
        && service_id.ends_with(".service")
        && service_id
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && service_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"@_.:-".contains(&byte))
}

async fn api_not_found() -> impl IntoResponse {
    error_response(StatusCode::NOT_FOUND, "not_found", "resource not found")
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

#[cfg(test)]
mod tests {
    use deckox_protocol::ServiceLogPriority;

    use super::{log_priority_name, valid_log_lines, valid_service_id};

    #[test]
    fn validates_service_ids_before_proxying() {
        assert!(valid_service_id("nginx.service"));
        assert!(valid_service_id("postgresql@main.service"));
        assert!(!valid_service_id("-nginx.service"));
        assert!(!valid_service_id("nginx.service/restart"));
        assert!(!valid_service_id("nginx.service\r\nHost: bad"));
    }

    #[test]
    fn validates_log_queries_before_proxying() {
        assert!(valid_log_lines(50));
        assert!(valid_log_lines(500));
        assert!(!valid_log_lines(0));
        assert!(!valid_log_lines(501));
        assert_eq!(log_priority_name(ServiceLogPriority::All), "all");
        assert_eq!(log_priority_name(ServiceLogPriority::Error), "error");
        assert_eq!(log_priority_name(ServiceLogPriority::Warning), "warning");
        assert_eq!(log_priority_name(ServiceLogPriority::Info), "info");
    }
}
