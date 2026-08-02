use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use deckox_protocol::{UpdateCheckStatus, UpdateStatus};
use reqwest::{Client, header};
use semver::Version;
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::warn;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/scolor-dev/deckox/releases/latest";
const RELEASE_PAGE_PREFIX: &str = "https://github.com/scolor-dev/deckox/releases/tag/";
const GITHUB_API_VERSION: &str = "2022-11-28";
const CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES_U64: u64 = 64 * 1024;

type FetchFuture = Pin<Box<dyn Future<Output = Result<LatestRelease, FetchError>> + Send>>;
type Fetcher = dyn Fn() -> FetchFuture + Send + Sync;

#[derive(Clone)]
pub struct UpdateChecker {
    current_version: String,
    fetcher: Arc<Fetcher>,
    cache: Arc<Mutex<Option<CachedStatus>>>,
    cache_ttl: Duration,
}

struct CachedStatus {
    stored_at: Instant,
    status: UpdateStatus,
}

#[derive(Clone, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

enum FetchError {
    Request,
    ResponseTooLarge,
    InvalidResponse,
}

impl UpdateChecker {
    pub fn new() -> Result<Self, &'static str> {
        let user_agent = format!("Deckox/{}", env!("CARGO_PKG_VERSION"));
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(user_agent)
            .build()
            .map_err(|_| "failed to initialize update checker")?;
        let fetcher: Arc<Fetcher> = Arc::new(move || {
            let client = client.clone();
            Box::pin(async move { fetch_latest_release(&client).await })
        });

        Ok(Self {
            current_version: env!("CARGO_PKG_VERSION").to_owned(),
            fetcher,
            cache: Arc::new(Mutex::new(None)),
            cache_ttl: CACHE_TTL,
        })
    }

    pub async fn check(&self) -> UpdateStatus {
        let mut cache = self.cache.lock().await;
        if let Some(cached) = cache.as_ref()
            && cached.stored_at.elapsed() < self.cache_ttl
        {
            return cached.status.clone();
        }

        let checked_at_ms = current_timestamp_ms();
        let status = if let Ok(release) = (self.fetcher)().await {
            evaluate_update(&self.current_version, &release.tag_name, checked_at_ms)
        } else {
            warn!("GitHub update check failed");
            unavailable_status(&self.current_version, checked_at_ms)
        };
        *cache = Some(CachedStatus {
            stored_at: Instant::now(),
            status: status.clone(),
        });
        status
    }

    #[cfg(test)]
    fn with_fetcher<F>(current_version: &str, cache_ttl: Duration, fetcher: F) -> Self
    where
        F: Fn() -> FetchFuture + Send + Sync + 'static,
    {
        Self {
            current_version: current_version.to_owned(),
            fetcher: Arc::new(fetcher),
            cache: Arc::new(Mutex::new(None)),
            cache_ttl,
        }
    }
}

async fn fetch_latest_release(client: &Client) -> Result<LatestRelease, FetchError> {
    let mut response = client
        .get(LATEST_RELEASE_URL)
        .header(header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .send()
        .await
        .map_err(|_| FetchError::Request)?;

    if !response.status().is_success() {
        return Err(FetchError::Request);
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES_U64)
    {
        return Err(FetchError::ResponseTooLarge);
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| FetchError::Request)? {
        append_response_chunk(&mut body, &chunk)?;
    }

    serde_json::from_slice(&body).map_err(|_| FetchError::InvalidResponse)
}

fn append_response_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), FetchError> {
    let next_length = body
        .len()
        .checked_add(chunk.len())
        .ok_or(FetchError::ResponseTooLarge)?;
    if next_length > MAX_RESPONSE_BYTES {
        return Err(FetchError::ResponseTooLarge);
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn evaluate_update(current: &str, latest_tag: &str, checked_at_ms: u64) -> UpdateStatus {
    let Some(current_version) = parse_version(current) else {
        return unavailable_status(current, checked_at_ms);
    };
    let Some(latest_version) = parse_version(latest_tag) else {
        return unavailable_status(current, checked_at_ms);
    };

    let update_available = latest_version > current_version;
    let status = if update_available {
        UpdateCheckStatus::Available
    } else {
        UpdateCheckStatus::UpToDate
    };
    let safe_tag = if latest_tag.starts_with('v') {
        format!("v{latest_version}")
    } else {
        latest_version.to_string()
    };

    UpdateStatus {
        current_version: current.to_owned(),
        latest_version: Some(safe_tag.clone()),
        update_available,
        release_url: Some(format!("{RELEASE_PAGE_PREFIX}{safe_tag}")),
        checked_at_ms: Some(checked_at_ms),
        status,
    }
}

fn parse_version(value: &str) -> Option<Version> {
    Version::parse(value.strip_prefix('v').unwrap_or(value)).ok()
}

fn unavailable_status(current_version: &str, checked_at_ms: u64) -> UpdateStatus {
    UpdateStatus {
        current_version: current_version.to_owned(),
        latest_version: None,
        update_available: false,
        release_url: None,
        checked_at_ms: Some(checked_at_ms),
        status: UpdateCheckStatus::Unavailable,
    }
}

fn current_timestamp_ms() -> u64 {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use axum::{Json, http::StatusCode, response::IntoResponse};
    use deckox_protocol::UpdateCheckStatus;

    use super::{
        FetchError, LatestRelease, MAX_RESPONSE_BYTES, UpdateChecker, append_response_chunk,
        evaluate_update,
    };

    #[test]
    fn stable_release_is_newer_than_prerelease() {
        let status = evaluate_update("0.3.8-beta.1", "v0.3.8", 10);

        assert_eq!(status.status, UpdateCheckStatus::Available);
        assert!(status.update_available);
        assert_eq!(status.latest_version.as_deref(), Some("v0.3.8"));
        assert_eq!(
            status.release_url.as_deref(),
            Some("https://github.com/scolor-dev/deckox/releases/tag/v0.3.8")
        );
    }

    #[test]
    fn prerelease_does_not_replace_same_stable_release() {
        let status = evaluate_update("0.3.8", "v0.3.8-beta.2", 10);

        assert_eq!(status.status, UpdateCheckStatus::UpToDate);
        assert!(!status.update_available);
    }

    #[test]
    fn invalid_tag_returns_no_untrusted_release_url() {
        let status = evaluate_update("0.3.7", "../../settings", 10);

        assert_eq!(status.status, UpdateCheckStatus::Unavailable);
        assert!(status.latest_version.is_none());
        assert!(status.release_url.is_none());
    }

    #[test]
    fn rejects_response_body_over_limit() {
        let mut body = vec![0; MAX_RESPONSE_BYTES];

        assert!(append_response_chunk(&mut body, &[1]).is_err());
        assert_eq!(body.len(), MAX_RESPONSE_BYTES);
    }

    #[tokio::test]
    async fn caches_successful_result() {
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch_calls = calls.clone();
        let checker = UpdateChecker::with_fetcher("0.3.7", Duration::from_secs(900), move || {
            fetch_calls.fetch_add(1, Ordering::Relaxed);
            Box::pin(async {
                Ok(LatestRelease {
                    tag_name: "v0.3.8".to_owned(),
                })
            })
        });

        let first = checker.check().await;
        let second = checker.check().await;

        assert_eq!(first.status, UpdateCheckStatus::Available);
        assert_eq!(second.checked_at_ms, first.checked_at_ms);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn external_failure_is_safe_and_cached() {
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch_calls = calls.clone();
        let checker = UpdateChecker::with_fetcher("0.3.7", Duration::from_secs(900), move || {
            fetch_calls.fetch_add(1, Ordering::Relaxed);
            Box::pin(async { Err(FetchError::Request) })
        });

        let first = checker.check().await;
        let second = checker.check().await;

        assert_eq!(first.status, UpdateCheckStatus::Unavailable);
        assert!(first.latest_version.is_none());
        assert!(first.release_url.is_none());
        assert_eq!(Json(first.clone()).into_response().status(), StatusCode::OK);
        assert_eq!(second.checked_at_ms, first.checked_at_ms);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
