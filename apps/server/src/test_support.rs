//! A stand-in for the Agent, for Server tests that need something on the
//! other end of the socket — without root, systemd or Linux. It answers
//! canned JSON per method and path and records every request it receives.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Router,
    extract::State,
    http::{Method, StatusCode, Uri},
    response::IntoResponse,
};
use serde_json::Value;
use tokio::net::UnixListener;

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub body: String,
}

#[derive(Clone)]
struct Canned {
    method: &'static str,
    path: String,
    status: u16,
    body: Value,
}

#[derive(Clone)]
struct Shared {
    routes: Arc<Vec<Canned>>,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

pub struct FakeAgent {
    pub socket: PathBuf,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

impl FakeAgent {
    /// Starts a fake Agent answering the given `(method, path, status, body)`
    /// routes; any other request gets `404`.
    pub fn start(routes: &[(&'static str, &str, u16, Value)]) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "deckox-fake-agent-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let socket = dir.join("agent.sock");
        let listener = UnixListener::bind(&socket).expect("bind fake agent socket");

        let recorded = Arc::new(Mutex::new(Vec::new()));
        let shared = Shared {
            routes: Arc::new(
                routes
                    .iter()
                    .map(|(method, path, status, body)| Canned {
                        method,
                        path: (*path).to_owned(),
                        status: *status,
                        body: body.clone(),
                    })
                    .collect(),
            ),
            recorded: Arc::clone(&recorded),
        };
        let app = Router::new().fallback(answer).with_state(shared);
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self { socket, recorded }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.recorded
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

async fn answer(
    State(shared): State<Shared>,
    method: Method,
    uri: Uri,
    body: String,
) -> impl IntoResponse {
    shared
        .recorded
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(Recorded {
            method: method.to_string(),
            path: uri.path().to_owned(),
            body,
        });
    let canned = shared
        .routes
        .iter()
        .find(|route| route.method == method.as_str() && route.path == uri.path());
    canned.map_or_else(
        || {
            (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({"code": "not_found", "message": "no such route"})),
            )
                .into_response()
        },
        |route| {
            (
                StatusCode::from_u16(route.status).unwrap_or(StatusCode::OK),
                axum::Json(route.body.clone()),
            )
                .into_response()
        },
    )
}
