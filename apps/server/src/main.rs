use std::{env, io::Read, net::SocketAddr, path::PathBuf};

use axum::{
    Extension, Json, Router,
    extract::{FromRef, Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use deckox_protocol::{AddSshKeyRequest, AgentStatus, HealthResponse};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::agent_client::AgentClient;
use crate::{
    auth::{AuthManager, AuthenticatedUser},
    request_context::RequestId,
};

mod agent_client;
mod auth;
mod request_context;

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_AGENT_SOCKET: &str = "/run/deckox/agent.sock";
const DEFAULT_WEB_DIR: &str = "/usr/local/share/deckox/web";

#[derive(Clone)]
struct AppState {
    agent: AgentClient,
    auth: AuthManager,
}

impl FromRef<AppState> for AuthManager {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
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

#[tokio::main]
async fn main() {
    if env::args().nth(1).as_deref() == Some("hash-password") {
        hash_password_from_stdin();
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
    let auth = AuthManager::load().unwrap_or_else(|error| {
        eprintln!("failed to load authentication configuration: {error}");
        std::process::exit(2);
    });
    let state = AppState {
        agent: AgentClient::new(PathBuf::from(
            env::var("DECKOX_AGENT_SOCKET").unwrap_or_else(|_| DEFAULT_AGENT_SOCKET.to_owned()),
        )),
        auth: auth.clone(),
    };

    let protected_api = Router::new()
        .route("/status", get(status))
        .route("/system", get(proxy_system))
        .route("/system/metrics", get(proxy_metrics))
        .route("/storage", get(proxy_storage))
        .route("/services", get(proxy_services))
        .route("/services/{service_id}", get(proxy_service_details))
        .route("/services/{service_id}/start", post(proxy_start_service))
        .route("/services/{service_id}/stop", post(proxy_stop_service))
        .route(
            "/services/{service_id}/restart",
            post(proxy_restart_service),
        )
        .route("/auth/logout", post(auth::logout))
        .route("/settings/password", post(auth::change_password))
        .route(
            "/settings/ssh/keys",
            get(proxy_ssh_keys).post(proxy_add_ssh_key),
        )
        .route(
            "/settings/ssh/keys/{key_id}",
            axum::routing::delete(proxy_remove_ssh_key),
        )
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

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("deckox_server=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
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

async fn proxy_ssh_keys(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_agent(&state.agent, "GET", "/v1/ssh/keys", &request_id).await
}

async fn proxy_add_ssh_key(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<AddSshKeyRequest>,
) -> Response {
    let response = match state
        .agent
        .request_json("POST", "/v1/ssh/keys", &request_id, &payload)
        .await
    {
        Ok(response) => (response.status, Json(response.body)).into_response(),
        Err(message) => agent_unavailable(message),
    };
    log_ssh_action("add", &response, &request_id, &user);
    response
}

async fn proxy_remove_ssh_key(
    State(state): State<AppState>,
    Path(key_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    if key_id.len() != 64 || !key_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "bad_request",
                message: "invalid SSH key ID".to_owned(),
            }),
        )
            .into_response();
    }
    let response = proxy_agent(
        &state.agent,
        "DELETE",
        &format!("/v1/ssh/keys/{key_id}"),
        &request_id,
    )
    .await;
    log_ssh_action("remove", &response, &request_id, &user);
    response
}

fn log_ssh_action(
    action: &str,
    response: &Response,
    request_id: &RequestId,
    user: &AuthenticatedUser,
) {
    if response.status().is_success() {
        info!(
            event = "ssh_key_action",
            request_id = %request_id.0,
            actor = "admin",
            source_ip = %user.source_ip,
            action,
            result = "success",
            "SSH public key action completed"
        );
    } else {
        warn!(
            event = "ssh_key_action",
            request_id = %request_id.0,
            actor = "admin",
            source_ip = %user.source_ip,
            action,
            result = "failure",
            status = response.status().as_u16(),
            "SSH public key action failed"
        );
    }
}

async fn proxy_service_details(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Response {
    proxy_service_request(&state.agent, "GET", &service_id, None, &request_id, None).await
}

async fn proxy_start_service(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Response {
    proxy_service_request(
        &state.agent,
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
        "POST",
        &service_id,
        Some("restart"),
        &request_id,
        Some(&user),
    )
    .await
}

async fn proxy_service_request(
    client: &AgentClient,
    method: &str,
    service_id: &str,
    action: Option<&str>,
    request_id: &RequestId,
    user: Option<&AuthenticatedUser>,
) -> Response {
    if !valid_service_id(service_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "bad_request",
                message: "invalid systemd service id".to_owned(),
            }),
        )
            .into_response();
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
        }
    }
    response
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

fn agent_unavailable(message: String) -> Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorResponse {
            code: "agent_unavailable",
            message,
        }),
    )
        .into_response()
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
    use super::valid_service_id;

    #[test]
    fn validates_service_ids_before_proxying() {
        assert!(valid_service_id("nginx.service"));
        assert!(valid_service_id("postgresql@main.service"));
        assert!(!valid_service_id("-nginx.service"));
        assert!(!valid_service_id("nginx.service/restart"));
        assert!(!valid_service_id("nginx.service\r\nHost: bad"));
    }
}
