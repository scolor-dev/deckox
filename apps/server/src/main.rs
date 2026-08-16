use std::{
    env,
    io::{Read, Write},
    net::SocketAddr,
    path::PathBuf,
};

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
use tracing::{error, info, warn};
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
    if run_cli_subcommand(env::args().nth(1).as_deref()).await {
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
        .route_layer(middleware::from_fn_with_state(
            auth.clone(),
            auth::require_auth,
        ))
        .fallback(api_not_found);
    let public_api = Router::new()
        .route("/auth/login", post(auth::login))
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

/// Handles `deckox-server <subcommand>` invocations that exit before the
/// server starts. Returns `true` when `argument` matched a subcommand, so
/// `main` knows to return instead of continuing to serve.
async fn run_cli_subcommand(argument: Option<&str>) -> bool {
    match argument {
        Some("hash-password") => hash_password_from_stdin(),
        Some("reset-password") => reset_password_from_stdin().await,
        Some("disable-totp") => disable_totp_interactive().await,
        _ => return false,
    }
    true
}

fn load_update_checker() -> update::UpdateChecker {
    update::UpdateChecker::new().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    })
}

fn hash_password_from_stdin() {
    let mut password = String::new();
    if let Err(error) = std::io::stdin().take(1025).read_to_string(&mut password) {
        eprintln!("failed to read password: {error}");
        std::process::exit(2);
    }
    let password = password.trim_end_matches(['\r', '\n']);
    match auth::hash_password(password) {
        Ok(hash) => println!("{hash}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

/// `deckox-server reset-password` — run from an SSH-connected console when
/// the admin password is lost. Reads the new password from stdin, exactly
/// like `hash-password`, and updates only `password_hash` in the account
/// file; TOTP settings are left untouched (see `disable-totp` for that
/// recovery path). The running server keeps the old password in memory
/// until restarted, so this always ends with a restart reminder.
async fn reset_password_from_stdin() {
    let account_path = auth::account_path_from_env().unwrap_or_else(|| {
        eprintln!(
            "password cannot be reset while DECKOX_ADMIN_PASSWORD_HASH is configured; \
             remove it and use DECKOX_ADMIN_PASSWORD_HASH_FILE instead"
        );
        std::process::exit(2);
    });

    let mut password = String::new();
    if let Err(error) = std::io::stdin().take(1025).read_to_string(&mut password) {
        eprintln!("failed to read password: {error}");
        std::process::exit(2);
    }
    let password = password.trim_end_matches(['\r', '\n']);
    if let Err(error) = auth::validate_new_password(password) {
        eprintln!("{error}");
        std::process::exit(2);
    }

    let mut account = load_admin_account_or_exit(&account_path);
    let new_hash = auth::hash_password(password).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    account.password_hash = new_hash;
    account.updated_at_ms = auth::now_ms();
    if let Err(error) = auth::write_account(&account_path, &account).await {
        eprintln!("failed to save the new password: {error}");
        std::process::exit(2);
    }

    AuditLog::from_env()
        .record_cli("password_reset_cli", "success", None)
        .await;
    println!("Password updated. Restart deckox-server for the change to take effect:");
    println!("  sudo systemctl restart deckox-server");
}

/// `deckox-server disable-totp` — the last-resort recovery path when both
/// the authenticator app and every recovery code are lost. Interactive
/// confirmation guards against running it by accident, since it lowers the
/// account's security to password-only.
async fn disable_totp_interactive() {
    let account_path = auth::account_path_from_env().unwrap_or_else(|| {
        eprintln!(
            "TOTP cannot be changed while DECKOX_ADMIN_PASSWORD_HASH is configured; \
             remove it and use DECKOX_ADMIN_PASSWORD_HASH_FILE instead"
        );
        std::process::exit(2);
    });

    let mut account = load_admin_account_or_exit(&account_path);
    if account.totp.is_none() {
        println!("Two-factor authentication is not enabled for the admin account.");
        return;
    }

    print!("Disable two-factor authentication for the admin account? [y/N] ");
    if std::io::stdout().flush().is_err() {
        eprintln!("failed to write prompt");
        std::process::exit(2);
    }
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        eprintln!("failed to read confirmation");
        std::process::exit(2);
    }
    if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
        println!("Cancelled.");
        return;
    }

    account.totp = None;
    account.updated_at_ms = auth::now_ms();
    if let Err(error) = auth::write_account(&account_path, &account).await {
        eprintln!("failed to save the account: {error}");
        std::process::exit(2);
    }

    AuditLog::from_env()
        .record_cli("totp_disabled_cli", "success", None)
        .await;
    println!(
        "Two-factor authentication disabled. Restart deckox-server for the change to take effect:"
    );
    println!("  sudo systemctl restart deckox-server");
}

fn load_admin_account_or_exit(account_path: &std::path::Path) -> auth::Account {
    let account_file = auth::load_account_file(account_path).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    account_file
        .accounts
        .get(auth::ADMIN_ACCOUNT)
        .cloned()
        .unwrap_or_else(|| {
            eprintln!(
                "{} does not contain an admin account",
                account_path.display()
            );
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
    match state
        .auth
        .confirm_current_password(user.source_ip, payload.current_password)
        .await
    {
        PasswordConfirmationResult::Invalid => {
            warn!(
                event = "system_reboot",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                result = "failure",
                reason = "invalid_password",
                "system reboot confirmation failed"
            );
            state
                .audit
                .record_admin(
                    "system_reboot",
                    user.source_ip,
                    "failure",
                    Some("invalid_password".to_owned()),
                )
                .await;
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    code: "invalid_current_password",
                    message: "current password is incorrect".to_owned(),
                }),
            )
                .into_response();
        }
        PasswordConfirmationResult::RateLimited => {
            warn!(
                event = "system_reboot",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                result = "rate_limited",
                "system reboot confirmation rate limited"
            );
            state
                .audit
                .record_admin("system_reboot", user.source_ip, "rate_limited", None)
                .await;
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    code: "rate_limited",
                    message: "too many password confirmation attempts".to_owned(),
                }),
            )
                .into_response();
        }
        PasswordConfirmationResult::Confirmed => {}
    }

    let response = proxy_agent(&state.agent, "POST", "/v1/system/reboot", &request_id).await;
    if response.status().is_success() {
        info!(
            event = "system_reboot",
            request_id = %request_id.0,
            actor = "admin",
            source_ip = %user.source_ip,
            result = "accepted",
            "system reboot accepted"
        );
        state
            .audit
            .record_admin("system_reboot", user.source_ip, "accepted", None)
            .await;
    } else {
        warn!(
            event = "system_reboot",
            request_id = %request_id.0,
            actor = "admin",
            source_ip = %user.source_ip,
            result = "failure",
            status = response.status().as_u16(),
            "system reboot failed"
        );
        state
            .audit
            .record_admin(
                "system_reboot",
                user.source_ip,
                "failure",
                Some(format!("status={}", response.status().as_u16())),
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
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "bad_request",
                message: "log lines must be one of 50, 100, 200, or 500".to_owned(),
            }),
        )
            .into_response();
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
            info!(
                event = "service_action",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                service = service_id,
                action,
                result = "success",
                "service action completed"
            );
            audit
                .record_admin(
                    "service_action",
                    user.source_ip,
                    "success",
                    Some(format!("service={service_id} action={action}")),
                )
                .await;
        } else {
            warn!(
                event = "service_action",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                service = service_id,
                action,
                result = "failure",
                status = response.status().as_u16(),
                "service action failed"
            );
            audit
                .record_admin(
                    "service_action",
                    user.source_ip,
                    "failure",
                    Some(format!(
                        "service={service_id} action={action} status={}",
                        response.status().as_u16()
                    )),
                )
                .await;
        }
    }
    response
}

fn invalid_service_id() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            code: "bad_request",
            message: "invalid systemd service id".to_owned(),
        }),
    )
        .into_response()
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
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            code: "not_found",
            message: "resource not found".to_owned(),
        }),
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
