use std::{
    collections::HashMap,
    env, fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{ConnectInfo, Request, State},
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{COOKIE, HOST, ORIGIN, SET_COOKIE, UPGRADE},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncWriteExt,
    sync::{Mutex, RwLock},
};
use tracing::{info, warn};

use crate::{ErrorResponse, request_context::RequestId};

const DEFAULT_PASSWORD_HASH_FILE: &str = "/var/lib/deckox/admin-password.hash";
const SESSION_COOKIE: &str = "deckox_session";
const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
const FAILURE_WINDOW: Duration = Duration::from_secs(5 * 60);
const MAX_FAILURES: u32 = 5;
const MAX_PASSWORD_BYTES: usize = 1024;
const MIN_PASSWORD_BYTES: usize = 12;

#[derive(Clone)]
pub struct AuthManager {
    inner: Arc<AuthInner>,
}

struct AuthInner {
    password_hash: RwLock<String>,
    password_hash_path: Option<PathBuf>,
    secure_cookie: bool,
    sessions: Mutex<HashMap<String, Instant>>,
    failures: Mutex<HashMap<IpAddr, FailedLogins>>,
    password_change_failures: Mutex<HashMap<IpAddr, FailedLogins>>,
    password_change: Mutex<()>,
}

struct FailedLogins {
    count: u32,
    started_at: Instant,
}

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub source_ip: IpAddr,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Serialize)]
pub struct AuthStatus {
    authenticated: bool,
}

enum LoginResult {
    Authenticated(String),
    Invalid,
    RateLimited,
}

enum ChangePasswordResult {
    Changed,
    InvalidCurrentPassword,
    RateLimited,
    InvalidNewPassword,
    NotPersistent,
    Failed(String),
}

pub enum PasswordConfirmationResult {
    Confirmed,
    Invalid,
    RateLimited,
}

impl AuthManager {
    pub fn load() -> Result<Self, String> {
        let (password_hash, password_hash_path) =
            if let Ok(password_hash) = env::var("DECKOX_ADMIN_PASSWORD_HASH") {
                (password_hash, None)
            } else {
                let hash_path = PathBuf::from(
                    env::var("DECKOX_ADMIN_PASSWORD_HASH_FILE")
                        .unwrap_or_else(|_| DEFAULT_PASSWORD_HASH_FILE.to_owned()),
                );
                let password_hash = fs::read_to_string(&hash_path)
                    .map_err(|error| format!("failed to read {}: {error}", hash_path.display()))?;
                (password_hash, Some(hash_path))
            };
        let password_hash = password_hash.trim().to_owned();
        PasswordHash::new(&password_hash)
            .map_err(|error| format!("invalid admin password hash: {error}"))?;

        let secure_cookie = env::var("DECKOX_SECURE_COOKIE").is_ok_and(|value| value == "true");
        Ok(Self {
            inner: Arc::new(AuthInner {
                password_hash: RwLock::new(password_hash),
                password_hash_path,
                secure_cookie,
                sessions: Mutex::new(HashMap::new()),
                failures: Mutex::new(HashMap::new()),
                password_change_failures: Mutex::new(HashMap::new()),
                password_change: Mutex::new(()),
            }),
        })
    }

    async fn login(&self, source_ip: IpAddr, password: String) -> LoginResult {
        if password.is_empty() || password.len() > MAX_PASSWORD_BYTES {
            record_failure(&self.inner.failures, source_ip).await;
            return LoginResult::Invalid;
        }
        if is_rate_limited(&self.inner.failures, source_ip).await {
            return LoginResult::RateLimited;
        }

        let password_hash = self.inner.password_hash.read().await.clone();
        let valid = verify_password(password_hash, password).await;

        if !valid {
            record_failure(&self.inner.failures, source_ip).await;
            return LoginResult::Invalid;
        }

        self.inner.failures.lock().await.remove(&source_ip);
        self.inner
            .password_change_failures
            .lock()
            .await
            .remove(&source_ip);
        let token = hex::encode(rand::random::<[u8; 32]>());
        self.inner
            .sessions
            .lock()
            .await
            .insert(token.clone(), Instant::now() + SESSION_TTL);
        LoginResult::Authenticated(token)
    }

    async fn change_password(
        &self,
        source_ip: IpAddr,
        current_password: String,
        new_password: String,
    ) -> ChangePasswordResult {
        let _change_guard = self.inner.password_change.lock().await;
        let Some(hash_path) = self.inner.password_hash_path.clone() else {
            return ChangePasswordResult::NotPersistent;
        };
        if new_password.len() < MIN_PASSWORD_BYTES || new_password.len() > MAX_PASSWORD_BYTES {
            return ChangePasswordResult::InvalidNewPassword;
        }
        if is_rate_limited(&self.inner.password_change_failures, source_ip).await {
            return ChangePasswordResult::RateLimited;
        }

        let current_hash = self.inner.password_hash.read().await.clone();
        if !verify_password(current_hash, current_password).await {
            record_failure(&self.inner.password_change_failures, source_ip).await;
            return ChangePasswordResult::InvalidCurrentPassword;
        }
        self.inner
            .password_change_failures
            .lock()
            .await
            .remove(&source_ip);

        let password_to_hash = new_password;
        let new_hash =
            match tokio::task::spawn_blocking(move || hash_password(&password_to_hash)).await {
                Ok(Ok(hash)) => hash,
                Ok(Err(error)) => return ChangePasswordResult::Failed(error),
                Err(error) => {
                    return ChangePasswordResult::Failed(format!(
                        "password hashing task failed: {error}"
                    ));
                }
            };
        if let Err(error) = write_password_hash(&hash_path, &new_hash).await {
            return ChangePasswordResult::Failed(error);
        }

        *self.inner.password_hash.write().await = new_hash;
        self.inner.sessions.lock().await.clear();
        self.inner.failures.lock().await.clear();
        ChangePasswordResult::Changed
    }

    pub async fn confirm_current_password(
        &self,
        source_ip: IpAddr,
        password: String,
    ) -> PasswordConfirmationResult {
        if password.is_empty() || password.len() > MAX_PASSWORD_BYTES {
            record_failure(&self.inner.password_change_failures, source_ip).await;
            return PasswordConfirmationResult::Invalid;
        }
        if is_rate_limited(&self.inner.password_change_failures, source_ip).await {
            return PasswordConfirmationResult::RateLimited;
        }

        let password_hash = self.inner.password_hash.read().await.clone();
        if !verify_password(password_hash, password).await {
            record_failure(&self.inner.password_change_failures, source_ip).await;
            return PasswordConfirmationResult::Invalid;
        }
        self.inner
            .password_change_failures
            .lock()
            .await
            .remove(&source_ip);
        PasswordConfirmationResult::Confirmed
    }

    async fn is_authenticated(&self, headers: &HeaderMap) -> bool {
        let Some(token) = session_token(headers) else {
            return false;
        };
        let now = Instant::now();
        let mut sessions = self.inner.sessions.lock().await;
        sessions.retain(|_, expires_at| *expires_at > now);
        sessions
            .get(&token)
            .is_some_and(|expires_at| *expires_at > now)
    }

    async fn logout(&self, headers: &HeaderMap) {
        if let Some(token) = session_token(headers) {
            self.inner.sessions.lock().await.remove(&token);
        }
    }

    fn session_cookie(&self, token: &str) -> String {
        let secure = if self.inner.secure_cookie {
            "; Secure"
        } else {
            ""
        };
        format!(
            "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
            SESSION_TTL.as_secs(),
            secure
        )
    }

    fn expired_cookie(&self) -> String {
        let secure = if self.inner.secure_cookie {
            "; Secure"
        } else {
            ""
        };
        format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}")
    }
}

async fn is_rate_limited(
    failures: &Mutex<HashMap<IpAddr, FailedLogins>>,
    source_ip: IpAddr,
) -> bool {
    let now = Instant::now();
    let mut failures = failures.lock().await;
    failures.retain(|_, entry| now.duration_since(entry.started_at) < FAILURE_WINDOW);
    failures
        .get(&source_ip)
        .is_some_and(|entry| entry.count >= MAX_FAILURES)
}

async fn record_failure(failures: &Mutex<HashMap<IpAddr, FailedLogins>>, source_ip: IpAddr) {
    let now = Instant::now();
    let mut failures = failures.lock().await;
    let entry = failures.entry(source_ip).or_insert(FailedLogins {
        count: 0,
        started_at: now,
    });
    if now.duration_since(entry.started_at) >= FAILURE_WINDOW {
        entry.count = 0;
        entry.started_at = now;
    }
    entry.count = entry.count.saturating_add(1);
    drop(failures);
}

async fn verify_password(password_hash: String, password: String) -> bool {
    tokio::task::spawn_blocking(move || {
        let Ok(parsed_hash) = PasswordHash::new(&password_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

async fn write_password_hash(path: &Path, password_hash: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "password hash path has no parent directory".to_owned())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "password hash path has an invalid file name".to_owned())?;
    let temporary_path = parent.join(format!(
        ".{file_name}.{}",
        hex::encode(rand::random::<[u8; 8]>())
    ));

    let mut options = tokio::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);

    let result = async {
        let mut file = options
            .open(&temporary_path)
            .await
            .map_err(|error| format!("failed to create password hash file: {error}"))?;
        file.write_all(password_hash.as_bytes())
            .await
            .map_err(|error| format!("failed to write password hash: {error}"))?;
        file.write_all(b"\n")
            .await
            .map_err(|error| format!("failed to write password hash: {error}"))?;
        file.sync_all()
            .await
            .map_err(|error| format!("failed to sync password hash: {error}"))?;
        drop(file);
        tokio::fs::rename(&temporary_path, path)
            .await
            .map_err(|error| format!("failed to replace password hash: {error}"))
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    result
}

pub fn hash_password(password: &str) -> Result<String, String> {
    if password.is_empty() || password.len() > MAX_PASSWORD_BYTES {
        return Err("password must contain between 1 and 1024 bytes".to_owned());
    }
    let salt = SaltString::encode_b64(&rand::random::<[u8; 16]>())
        .map_err(|error| format!("failed to create password salt: {error}"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("failed to hash password: {error}"))
}

pub async fn login(
    State(auth): State<AuthManager>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    axum::Extension(request_id): axum::Extension<RequestId>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Response {
    if !same_origin(&headers) {
        warn!(
            event = "auth_login",
            request_id = %request_id.0,
            source_ip = %peer.ip(),
            result = "rejected",
            reason = "origin",
            "login rejected"
        );
        return error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "request origin is not allowed",
        );
    }

    match auth.login(peer.ip(), payload.password).await {
        LoginResult::Authenticated(token) => {
            info!(
                event = "auth_login",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %peer.ip(),
                result = "success",
                "administrator logged in"
            );
            let mut response = Json(AuthStatus {
                authenticated: true,
            })
            .into_response();
            if let Ok(value) = HeaderValue::from_str(&auth.session_cookie(&token)) {
                response.headers_mut().insert(SET_COOKIE, value);
            }
            response
        }
        LoginResult::Invalid => {
            warn!(
                event = "auth_login",
                request_id = %request_id.0,
                source_ip = %peer.ip(),
                result = "failure",
                "administrator login failed"
            );
            error_response(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "password is incorrect",
            )
        }
        LoginResult::RateLimited => {
            warn!(
                event = "auth_login",
                request_id = %request_id.0,
                source_ip = %peer.ip(),
                result = "rate_limited",
                "administrator login rate limited"
            );
            error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many login attempts; try again later",
            )
        }
    }
}

pub async fn status(State(auth): State<AuthManager>, headers: HeaderMap) -> Json<AuthStatus> {
    Json(AuthStatus {
        authenticated: auth.is_authenticated(&headers).await,
    })
}

pub async fn logout(
    State(auth): State<AuthManager>,
    axum::Extension(request_id): axum::Extension<RequestId>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    headers: HeaderMap,
) -> Response {
    auth.logout(&headers).await;
    info!(
        event = "auth_logout",
        request_id = %request_id.0,
        actor = "admin",
        source_ip = %user.source_ip,
        result = "success",
        "administrator logged out"
    );
    let mut response = Json(AuthStatus {
        authenticated: false,
    })
    .into_response();
    if let Ok(value) = HeaderValue::from_str(&auth.expired_cookie()) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    response
}

pub async fn change_password(
    State(auth): State<AuthManager>,
    axum::Extension(request_id): axum::Extension<RequestId>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Response {
    match auth
        .change_password(
            user.source_ip,
            payload.current_password,
            payload.new_password,
        )
        .await
    {
        ChangePasswordResult::Changed => {
            info!(
                event = "password_change",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                result = "success",
                "administrator password changed"
            );
            let mut response = Json(AuthStatus {
                authenticated: false,
            })
            .into_response();
            if let Ok(value) = HeaderValue::from_str(&auth.expired_cookie()) {
                response.headers_mut().insert(SET_COOKIE, value);
            }
            response
        }
        ChangePasswordResult::InvalidCurrentPassword => {
            warn!(
                event = "password_change",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                result = "failure",
                reason = "invalid_current_password",
                "administrator password confirmation failed"
            );
            error_response(
                StatusCode::UNAUTHORIZED,
                "invalid_current_password",
                "current password is incorrect",
            )
        }
        ChangePasswordResult::RateLimited => {
            warn!(
                event = "password_change",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                result = "rate_limited",
                "administrator password confirmation rate limited"
            );
            error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many password confirmation attempts; try again later",
            )
        }
        ChangePasswordResult::InvalidNewPassword => error_response(
            StatusCode::BAD_REQUEST,
            "invalid_new_password",
            "new password must contain between 12 and 1024 bytes",
        ),
        ChangePasswordResult::NotPersistent => error_response(
            StatusCode::CONFLICT,
            "password_change_unavailable",
            "password cannot be changed while DECKOX_ADMIN_PASSWORD_HASH is configured",
        ),
        ChangePasswordResult::Failed(error) => {
            warn!(
                event = "password_change",
                request_id = %request_id.0,
                actor = "admin",
                source_ip = %user.source_ip,
                result = "failure",
                reason = %error,
                "administrator password change failed"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "password_change_failed",
                "failed to save the new password",
            )
        }
    }
}

pub async fn require_auth(
    State(auth): State<AuthManager>,
    mut request: Request,
    next: Next,
) -> Response {
    let source_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| IpAddr::from([0, 0, 0, 0]), |peer| peer.ip());
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map_or("unknown", |value| value.0.as_str());

    if !auth.is_authenticated(request.headers()).await {
        warn!(
            event = "auth_required",
            request_id,
            source_ip = %source_ip,
            result = "rejected",
            "unauthenticated API request rejected"
        );
        return error_response(
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "authentication is required",
        );
    }
    if requires_same_origin(request.method(), request.headers()) && !same_origin(request.headers())
    {
        warn!(
            event = "auth_origin",
            request_id,
            actor = "admin",
            source_ip = %source_ip,
            result = "rejected",
            "cross-origin API request rejected"
        );
        return error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "request origin is not allowed",
        );
    }

    request
        .extensions_mut()
        .insert(AuthenticatedUser { source_ip });
    next.run(request).await
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            (name == SESSION_COOKIE && !value.is_empty()).then(|| value.to_owned())
        })
}

fn same_origin(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(HOST).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    origin == format!("http://{host}") || origin == format!("https://{host}")
}

const fn is_state_changing(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn requires_same_origin(method: &Method, headers: &HeaderMap) -> bool {
    is_state_changing(method)
        || headers
            .get(UPGRADE)
            .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"websocket"))
}

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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::IpAddr, sync::Arc};

    use axum::http::{HeaderMap, HeaderValue, Method, header};
    use tokio::sync::{Mutex, RwLock};

    use super::{
        AuthInner, AuthManager, MAX_FAILURES, PasswordConfirmationResult, hash_password,
        is_rate_limited, record_failure, requires_same_origin, same_origin, session_token,
    };

    #[test]
    fn hashes_password_with_argon2id() {
        let hash = hash_password("test-password").expect("password should hash");
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn extracts_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("theme=light; deckox_session=abc123"),
        );
        assert_eq!(session_token(&headers).as_deref(), Some("abc123"));
    }

    #[test]
    fn validates_same_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("192.168.1.21:8080"));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://192.168.1.21:8080"),
        );
        assert!(same_origin(&headers));
    }

    #[test]
    fn websocket_upgrade_requires_same_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));

        assert!(requires_same_origin(&Method::GET, &headers));
        assert!(!requires_same_origin(&Method::GET, &HeaderMap::new()));
    }

    #[tokio::test]
    async fn rate_limits_password_confirmation_failures() {
        let failures = Mutex::new(HashMap::new());
        let source_ip = IpAddr::from([192, 0, 2, 1]);
        for _ in 0..MAX_FAILURES {
            record_failure(&failures, source_ip).await;
        }
        assert!(is_rate_limited(&failures, source_ip).await);
    }

    #[tokio::test]
    async fn confirms_current_password_and_clears_failures() {
        let password = "correct-test-password";
        let auth = AuthManager {
            inner: Arc::new(AuthInner {
                password_hash: RwLock::new(hash_password(password).expect("password should hash")),
                password_hash_path: None,
                secure_cookie: false,
                sessions: Mutex::new(HashMap::new()),
                failures: Mutex::new(HashMap::new()),
                password_change_failures: Mutex::new(HashMap::new()),
                password_change: Mutex::new(()),
            }),
        };
        let source_ip = IpAddr::from([192, 0, 2, 2]);

        assert!(matches!(
            auth.confirm_current_password(source_ip, "wrong-password".to_owned())
                .await,
            PasswordConfirmationResult::Invalid
        ));
        assert!(matches!(
            auth.confirm_current_password(source_ip, password.to_owned())
                .await,
            PasswordConfirmationResult::Confirmed
        ));
        assert!(
            !is_rate_limited(&auth.inner.password_change_failures, source_ip).await,
            "successful confirmation should clear failures"
        );
    }
}
