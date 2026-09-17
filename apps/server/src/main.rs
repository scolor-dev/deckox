use std::{env, net::SocketAddr, path::PathBuf};

use axum::{
    Extension, Json, Router,
    extract::{FromRef, Path, Query, State},
    http::{StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use deckox_protocol::{
    AgentStatus, AgentUpdateRequest, AuditPage, CreateScheduleRequest, DiagnosticsReport,
    ServiceLogPriority, ServiceLogs, UpdateStatus,
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
mod notifier;
mod request_context;
mod security_headers;
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
    webhook_url: Option<String>,
    instance_id: String,
    listen_port: u16,
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
    port: u16,
    agent: Option<AgentStatus>,
    agent_error: Option<String>,
    webhook_configured: bool,
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

#[derive(Deserialize)]
struct UpdateTriggerRequest {
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
    let webhook_url = env::var("DECKOX_WEBHOOK_URL").ok();
    notifier::spawn(
        agent.clone(),
        audit.clone(),
        updates.clone(),
        webhook_url.clone(),
    );
    let state = AppState {
        agent: agent.clone(),
        auth: auth.clone(),
        audit,
        metrics: MetricsHub::new(agent),
        updates,
        webhook_url,
        instance_id: format!("{:016x}", rand::random::<u64>()),
        listen_port: listen_addr.port(),
    };

    let app = build_router(state, &auth, &web_dir);

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

/// Assembles the full route tree: public auth endpoints, the
/// authentication-gated API, the SPA fallback, and the layers (tracing,
/// request IDs, security headers) applied to all of them uniformly.
fn build_router(state: AppState, auth: &AuthManager, web_dir: &std::path::Path) -> Router {
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
        .route("/system/update", post(update_system))
        .route("/system/metrics", get(proxy_metrics))
        .route("/events/metrics", get(metrics_stream::metrics_events))
        .route("/storage", get(proxy_storage))
        .route("/backups", get(proxy_backups))
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
        .route("/services/{service_id}/allow", post(proxy_allow_service))
        .route(
            "/services/{service_id}/disallow",
            post(proxy_disallow_service),
        )
        .route("/services/{service_id}/logs", get(proxy_service_logs))
        .route(
            "/services/{service_id}/logs/report",
            get(service_logs_report),
        )
        .route("/software", get(proxy_software))
        .route("/software/{software_id}/install", post(install_software))
        .route("/software/{software_id}/remove", post(remove_software))
        .route("/software/{software_id}/upgrade", post(upgrade_software))
        .route("/software/{software_id}/allow", post(proxy_allow_software))
        .route(
            "/software/{software_id}/disallow",
            post(proxy_disallow_software),
        )
        .route("/schedules", get(proxy_schedules).post(create_schedule))
        .route(
            "/schedules/{schedule_id}",
            axum::routing::delete(delete_schedule),
        )
        .route("/schedules/{schedule_id}/enable", post(enable_schedule))
        .route("/schedules/{schedule_id}/disable", post(disable_schedule))
        .route("/auth/logout", post(auth::logout))
        .route("/settings/password", post(auth::change_password))
        .route("/settings/totp/status", get(auth::totp_status))
        .route("/settings/totp/setup", post(auth::totp_setup))
        .route("/settings/totp/confirm", post(auth::totp_confirm))
        .route("/settings/totp/disable", post(auth::totp_disable))
        .route("/settings/webhook/test", post(test_webhook))
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
        ServeDir::new(web_dir).not_found_service(ServeFile::new(web_dir.join("index.html")));
    Router::new()
        .route("/healthz", get(health))
        .nest("/api/v1", public_api)
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(request_context::assign_request_id))
        .layer(middleware::from_fn_with_state(
            auth.secure_cookie(),
            security_headers::apply,
        ))
        .with_state(state)
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
            port: state.listen_port,
            agent: Some(agent),
            agent_error: None,
            webhook_configured: state.webhook_url.is_some(),
        }),
        Err(error) => Json(ServerStatus {
            name: "deckox",
            version: env!("CARGO_PKG_VERSION"),
            status: "degraded",
            port: state.listen_port,
            agent: None,
            agent_error: Some(error),
            webhook_configured: state.webhook_url.is_some(),
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

/// Confirms the caller's current password for a dangerous, password-gated
/// operation (reboot, self-update, ...), auditing the outcome under `event`.
/// Returns `Ok(())` when confirmed; on failure it has already logged and
/// audited, and returns the finished error [`Response`] to return directly.
/// Boxed because clippy flags a bare `Response` as too large for a `Result`
/// error variant.
async fn confirm_password_or_respond(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    current_password: String,
    event: &'static str,
) -> Result<(), Box<Response>> {
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
                    event,
                    "failure",
                    Some("invalid_password".to_owned()),
                    "password confirmation failed",
                )
                .await;
            Err(Box::new(error_response(
                StatusCode::UNAUTHORIZED,
                "invalid_current_password",
                "current password is incorrect",
            )))
        }
        PasswordConfirmationResult::RateLimited => {
            state
                .audit
                .log_admin(
                    request_id,
                    user.source_ip,
                    event,
                    "rate_limited",
                    None,
                    "password confirmation rate limited",
                )
                .await;
            Err(Box::new(error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many password confirmation attempts",
            )))
        }
        PasswordConfirmationResult::Confirmed => Ok(()),
    }
}

/// Proxies the reboot request to the Agent and records the outcome. Kept
/// separate from the thin [`reboot_system`] handler above, mirroring how
/// [`proxy_service_request`] backs the service action handlers.
async fn reboot_host(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    current_password: String,
) -> Response {
    if let Err(response) =
        confirm_password_or_respond(state, request_id, user, current_password, "system_reboot")
            .await
    {
        return *response;
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

async fn update_system(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<UpdateTriggerRequest>,
) -> Response {
    trigger_update(&state, &request_id, &user, payload.current_password).await
}

/// Confirms the caller's password, resolves and fetches the pinned installer
/// for the latest known release, proxies the self-update request to the
/// Agent, and records the outcome. Mirrors [`reboot_host`]; the version and
/// script are resolved here (not accepted from the client) because only the
/// Server is allowed to reach GitHub, and re-resolving keeps a stale browser
/// tab from requesting an update to a version different from what was
/// actually checked.
async fn log_update_event(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    result: &'static str,
    detail: Option<String>,
    message: &'static str,
) {
    state
        .audit
        .log_admin(
            request_id,
            user.source_ip,
            "system_update",
            result,
            detail,
            message,
        )
        .await;
}

/// Resolves the already-checked target version and fetches its installer,
/// returning the finished error [`Response`] (already audited) when either
/// step fails. Split out of [`trigger_update`] to keep it under the
/// project's line-count lint.
async fn resolve_update_request(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
) -> Result<AgentUpdateRequest, Box<Response>> {
    let status = state.updates.check().await;
    let Some(target_version) = status
        .update_available
        .then_some(status.latest_version)
        .flatten()
    else {
        log_update_event(
            state,
            request_id,
            user,
            "failure",
            Some("no_update_available".to_owned()),
            "system update requested with no update available",
        )
        .await;
        return Err(Box::new(error_response(
            StatusCode::CONFLICT,
            "no_update_available",
            "no newer Deckox version is available",
        )));
    };

    let Ok(install_script) = state.updates.fetch_install_script(&target_version).await else {
        log_update_event(
            state,
            request_id,
            user,
            "failure",
            Some("installer_fetch_failed".to_owned()),
            "failed to fetch the installer for the target version",
        )
        .await;
        return Err(Box::new(error_response(
            StatusCode::BAD_GATEWAY,
            "installer_unavailable",
            "could not fetch the installer for the target version",
        )));
    };

    Ok(AgentUpdateRequest {
        target_version,
        install_script,
    })
}

/// Confirms the caller's password, resolves and fetches the pinned installer
/// for the latest known release, proxies the self-update request to the
/// Agent, and records the outcome. Mirrors [`reboot_host`]; the version and
/// script are resolved here (not accepted from the client) because only the
/// Server is allowed to reach GitHub, and re-resolving keeps a stale browser
/// tab from requesting an update to a version different from what was
/// actually checked.
async fn trigger_update(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    current_password: String,
) -> Response {
    if let Err(response) =
        confirm_password_or_respond(state, request_id, user, current_password, "system_update")
            .await
    {
        return *response;
    }

    let request = match resolve_update_request(state, request_id, user).await {
        Ok(request) => request,
        Err(response) => return *response,
    };
    let target_version = request.target_version.clone();

    let response = agent_result_to_response(
        state
            .agent
            .request_with_json_body("POST", "/v1/system/update", request_id, &request)
            .await,
    );

    if response.status().is_success() {
        log_update_event(
            state,
            request_id,
            user,
            "accepted",
            Some(format!("target_version={target_version}")),
            "system update accepted",
        )
        .await;
    } else {
        log_update_event(
            state,
            request_id,
            user,
            "failure",
            Some(format!("status={}", response.status().as_u16())),
            "system update failed",
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

async fn proxy_backups(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/backups", &request_id).await
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

async fn proxy_allow_service(
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
        Some("allow"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_disallow_service(
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
        Some("disallow"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_software(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/software", &request_id).await
}

async fn proxy_allow_software(
    State(state): State<AppState>,
    Path(software_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_software_allowlist_request(
        &state.agent,
        &state.audit,
        &software_id,
        "allow",
        &request_id,
        &user,
    )
    .await
}

async fn proxy_disallow_software(
    State(state): State<AppState>,
    Path(software_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_software_allowlist_request(
        &state.agent,
        &state.audit,
        &software_id,
        "disallow",
        &request_id,
        &user,
    )
    .await
}

async fn proxy_software_allowlist_request(
    client: &AgentClient,
    audit: &AuditLog,
    software_id: &str,
    action: &str,
    request_id: &RequestId,
    user: &AuthenticatedUser,
) -> Response {
    if !valid_software_id(software_id) {
        return invalid_software_id();
    }

    let path = format!("/v1/software/{software_id}/{action}");
    let response = proxy_agent(client, "POST", &path, request_id).await;
    if response.status().is_success() {
        audit
            .log_admin(
                request_id,
                user.source_ip,
                "software_allowlist",
                "success",
                Some(format!("software={software_id} action={action}")),
                "software allowlist changed",
            )
            .await;
    } else {
        audit
            .log_admin(
                request_id,
                user.source_ip,
                "software_allowlist",
                "failure",
                Some(format!(
                    "software={software_id} action={action} status={}",
                    response.status().as_u16()
                )),
                "software allowlist change failed",
            )
            .await;
    }
    response
}

#[derive(Deserialize)]
struct SoftwareActionRequest {
    current_password: String,
}

async fn install_software(
    State(state): State<AppState>,
    Path(software_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<SoftwareActionRequest>,
) -> Response {
    software_action(
        &state,
        &request_id,
        &user,
        payload.current_password,
        &software_id,
        "install",
    )
    .await
}

async fn remove_software(
    State(state): State<AppState>,
    Path(software_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<SoftwareActionRequest>,
) -> Response {
    software_action(
        &state,
        &request_id,
        &user,
        payload.current_password,
        &software_id,
        "remove",
    )
    .await
}

async fn upgrade_software(
    State(state): State<AppState>,
    Path(software_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<SoftwareActionRequest>,
) -> Response {
    software_action(
        &state,
        &request_id,
        &user,
        payload.current_password,
        &software_id,
        "upgrade",
    )
    .await
}

/// Installing, removing, and upgrading software actually changes the host at
/// the root level, unlike the allowlist toggles above — so this requires
/// re-confirming the admin password the same way [`reboot_host`] does,
/// reusing [`confirm_password_or_respond`].
async fn software_action(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    current_password: String,
    software_id: &str,
    action: &str,
) -> Response {
    if !valid_software_id(software_id) {
        return invalid_software_id();
    }
    if let Err(response) =
        confirm_password_or_respond(state, request_id, user, current_password, "software_action")
            .await
    {
        return *response;
    }

    let path = format!("/v1/software/{software_id}/{action}");
    let response = proxy_agent(&state.agent, "POST", &path, request_id).await;
    if response.status().is_success() {
        state
            .audit
            .log_admin(
                request_id,
                user.source_ip,
                "software_action",
                "success",
                Some(format!("software={software_id} action={action}")),
                "software action completed",
            )
            .await;
    } else {
        state
            .audit
            .log_admin(
                request_id,
                user.source_ip,
                "software_action",
                "failure",
                Some(format!(
                    "software={software_id} action={action} status={}",
                    response.status().as_u16()
                )),
                "software action failed",
            )
            .await;
    }
    response
}

fn invalid_software_id() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "bad_request",
        "invalid package name",
    )
}

/// A loose but bounded character class covering both Debian's and RPM's
/// package naming rules — this is only a shallow syntax check before
/// talking to the Agent (which performs the real existence check against
/// the host's configured repositories), the same role [`valid_service_id`]
/// plays for systemd unit ids.
fn valid_software_id(software_id: &str) -> bool {
    !software_id.is_empty()
        && software_id.len() <= 100
        && software_id
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && software_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+-._".contains(&byte))
}

/// Lets an admin verify `DECKOX_WEBHOOK_URL` works without waiting for a
/// real threshold breach. Reuses [`notifier::send_once`] — the exact same
/// request the background ticker would make — but reports the outcome
/// straight back to the caller and audit-logs it under their own identity
/// instead of `system`.
async fn test_webhook(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    let Some(webhook_url) = &state.webhook_url else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "webhook_not_configured",
            "DECKOX_WEBHOOK_URL is not set",
        );
    };

    let result = notifier::send_once(
        &reqwest::Client::new(),
        webhook_url,
        "test",
        "Deckoxからのテスト通知です。".to_owned(),
    )
    .await;

    match result {
        Ok(()) => {
            state
                .audit
                .log_admin(
                    &request_id,
                    user.source_ip,
                    "webhook_test",
                    "success",
                    None,
                    "webhook test notification sent",
                )
                .await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(message) => {
            state
                .audit
                .log_admin(
                    &request_id,
                    user.source_ip,
                    "webhook_test",
                    "failure",
                    Some(message.clone()),
                    "webhook test notification failed",
                )
                .await;
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    code: "webhook_test_failed",
                    message,
                }),
            )
                .into_response()
        }
    }
}

async fn proxy_schedules(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/schedules", &request_id).await
}

async fn create_schedule(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateScheduleRequest>,
) -> Response {
    if !valid_service_id(&payload.service_id) {
        return invalid_service_id();
    }
    let service_id = payload.service_id.clone();
    let response = agent_result_to_response(
        state
            .agent
            .request_with_json_body("POST", "/v1/schedules", &request_id, &payload)
            .await,
    );
    log_schedule_event(
        &state,
        &request_id,
        &user,
        "schedule_create",
        &service_id,
        response.status(),
    )
    .await;
    response
}

async fn delete_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    let response = proxy_agent(
        &state.agent,
        "DELETE",
        &format!("/v1/schedules/{schedule_id}"),
        &request_id,
    )
    .await;
    log_schedule_event(
        &state,
        &request_id,
        &user,
        "schedule_delete",
        &schedule_id,
        response.status(),
    )
    .await;
    response
}

async fn enable_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    let response = proxy_agent(
        &state.agent,
        "POST",
        &format!("/v1/schedules/{schedule_id}/enable"),
        &request_id,
    )
    .await;
    log_schedule_event(
        &state,
        &request_id,
        &user,
        "schedule_enable",
        &schedule_id,
        response.status(),
    )
    .await;
    response
}

async fn disable_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    let response = proxy_agent(
        &state.agent,
        "POST",
        &format!("/v1/schedules/{schedule_id}/disable"),
        &request_id,
    )
    .await;
    log_schedule_event(
        &state,
        &request_id,
        &user,
        "schedule_disable",
        &schedule_id,
        response.status(),
    )
    .await;
    response
}

/// Shared by the four schedule-mutation handlers so each one only needs to
/// name its event and the resource it acted on. Takes the response's
/// `StatusCode` rather than the `Response` itself: `Response` wraps a body
/// type that is not `Sync`, so a `&Response` held across the `.await` below
/// would make the caller's handler future `!Send` — which axum requires and
/// reports, unhelpfully, as "the trait `Handler` is not satisfied".
async fn log_schedule_event(
    state: &AppState,
    request_id: &RequestId,
    user: &AuthenticatedUser,
    event: &'static str,
    detail_id: &str,
    status: StatusCode,
) {
    let result = if status.is_success() {
        "success"
    } else {
        "failure"
    };
    state
        .audit
        .log_admin(
            request_id,
            user.source_ip,
            event,
            result,
            Some(format!("id={detail_id} status={}", status.as_u16())),
            "schedule change",
        )
        .await;
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

/// Downloads the same bounded log window `proxy_service_logs` displays, as a
/// file. `service_id` is safe to embed in the `Content-Disposition` header
/// unescaped because `valid_service_id` already restricts it to
/// `[A-Za-z0-9@_.:-]` ending in `.service`.
async fn service_logs_report(
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
    match state
        .agent
        .get_json::<ServiceLogs>(&path, &request_id)
        .await
    {
        Ok(logs) => service_logs_attachment(&service_id, &logs),
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

fn service_logs_attachment(service_id: &str, logs: &ServiceLogs) -> Response {
    let Ok(body) = serde_json::to_vec_pretty(logs) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let disposition = format!("attachment; filename=\"deckox-service-logs-{service_id}.json\"");
    (
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        body,
    )
        .into_response()
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
    agent_result_to_response(client.request(method, path, request_id).await)
}

/// Converts an [`AgentClient`] call's result into the `Response` sent to the
/// browser: the Agent's own status/body verbatim on success, or a uniform
/// `agent_unavailable` error otherwise. Shared by [`proxy_agent`] and
/// [`trigger_update`], which calls the Agent with a JSON body instead of the
/// bodyless GET/POST that `proxy_agent` covers.
fn agent_result_to_response(result: Result<agent_client::AgentResponse, String>) -> Response {
    match result {
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
    use axum::{http::header, response::IntoResponse};
    use deckox_protocol::{ServiceLogPriority, ServiceLogs};

    use super::{log_priority_name, service_logs_attachment, valid_log_lines, valid_service_id};

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

    #[test]
    fn log_attachment_names_the_file_after_the_service() {
        let logs = ServiceLogs {
            service_id: "nginx.service".to_owned(),
            entries: vec![],
        };
        let response = service_logs_attachment("nginx.service", &logs).into_response();
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok()),
            Some("attachment; filename=\"deckox-service-logs-nginx.service.json\"")
        );
    }
}
