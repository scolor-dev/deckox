//! The Agent's own copy of the admin password check.
//!
//! The Server also verifies the password, but a Server that has been taken
//! over could skip that. So dangerous operations carry the password to the
//! Agent, which checks it against a hash the Server cannot write: a
//! root-owned file (`/etc/deckox/admin-verifier` by default). Failed attempts
//! lock the check for a while, so a takeover cannot guess passwords through
//! the Agent.
//!
//! Where the hash comes from, in order: the verifier file; the
//! `DECKOX_ADMIN_PASSWORD_HASH` environment variable (development and
//! Compose); otherwise it is copied once from the Server's account file at
//! start-up, which is how an existing installation gets one on upgrade.

use std::{
    collections::VecDeque,
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::Deserialize;
use tracing::{info, warn};

use crate::error::AgentError;

const DEFAULT_VERIFIER_FILE: &str = "/etc/deckox/admin-verifier";
const DEFAULT_ACCOUNT_FILE: &str = "/var/lib/deckox/admin-password.hash";
const MAX_FAILURES: usize = 5;
const FAILURE_WINDOW: Duration = Duration::from_secs(600);
const LOCK_DURATION: Duration = Duration::from_secs(300);
const MIN_PASSWORD_BYTES: usize = 12;
const MAX_PASSWORD_BYTES: usize = 1024;

#[derive(Default)]
struct State {
    hash: Option<String>,
    /// Modification time of the verifier file when `hash` was read from it.
    file_modified: Option<SystemTime>,
    failures: VecDeque<Instant>,
    locked_until: Option<Instant>,
}

#[derive(Clone)]
pub struct Verifier {
    path: PathBuf,
    /// Whether the hash lives in the environment (so it cannot be replaced).
    from_environment: bool,
    state: Arc<Mutex<State>>,
}

#[derive(Deserialize)]
struct AccountFile {
    accounts: std::collections::HashMap<String, Account>,
}

#[derive(Deserialize)]
struct Account {
    password_hash: String,
}

impl Verifier {
    pub fn from_environment() -> Self {
        let path = PathBuf::from(
            env::var("DECKOX_AGENT_VERIFIER_FILE")
                .unwrap_or_else(|_| DEFAULT_VERIFIER_FILE.to_owned()),
        );
        let account = PathBuf::from(
            env::var("DECKOX_ADMIN_PASSWORD_HASH_FILE")
                .unwrap_or_else(|_| DEFAULT_ACCOUNT_FILE.to_owned()),
        );
        Self::load(path, &account, env::var("DECKOX_ADMIN_PASSWORD_HASH").ok())
    }

    pub fn load(path: PathBuf, account_file: &Path, environment_hash: Option<String>) -> Self {
        let mut state = State::default();
        let mut from_environment = false;
        if let Some(hash) = read_verifier(&path) {
            state.file_modified = modified(&path);
            state.hash = Some(hash);
        } else if let Some(hash) = environment_hash.filter(|hash| PasswordHash::new(hash).is_ok()) {
            state.hash = Some(hash);
            from_environment = true;
        } else if let Some(hash) = read_account_hash(account_file) {
            match write_verifier(&path, &hash) {
                Ok(()) => {
                    info!(path = %path.display(), "created the Agent's admin verifier from the Server's account file");
                    state.file_modified = modified(&path);
                }
                Err(error) => {
                    warn!(%error, "could not save the Agent's admin verifier; it will be used from memory only");
                }
            }
            state.hash = Some(hash);
        }
        Self {
            path,
            from_environment,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn is_provisioned(&self) -> bool {
        self.refresh();
        self.lock().hash.is_some()
    }

    /// Seconds until the lock ends, when the check is currently locked.
    pub fn locked_for(&self) -> Option<u64> {
        let state = self.lock();
        state
            .locked_until
            .and_then(|until| until.checked_duration_since(Instant::now()))
            .map(|remaining| remaining.as_secs() + 1)
    }

    /// Checks `password` against the verifier, counting failures.
    pub async fn verify(&self, password: &str) -> Result<(), AgentError> {
        self.refresh();
        if let Some(seconds) = self.locked_for() {
            return Err(AgentError::too_many_requests(format!(
                "too many wrong passwords; try again in {seconds} seconds"
            )));
        }
        let Some(hash) = self.lock().hash.clone() else {
            return Err(AgentError::unavailable(
                "the Agent has no admin password to check against; run `sudo deckox-server reset-password` as root",
            ));
        };
        if verify_hash(hash, password.to_owned()).await {
            self.lock().failures.clear();
            return Ok(());
        }
        self.record_failure();
        Err(AgentError::unauthorized(
            "invalid_password",
            "the admin password was not accepted by the Agent",
        ))
    }

    /// Replaces the verifier with `new_password`, after `current_password`
    /// checks out. Used when the admin changes the password in the Web UI.
    pub async fn replace(
        &self,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AgentError> {
        if self.from_environment {
            return Err(AgentError::conflict(
                "the admin password comes from the environment and cannot be changed here",
            ));
        }
        if new_password.len() < MIN_PASSWORD_BYTES || new_password.len() > MAX_PASSWORD_BYTES {
            return Err(AgentError::bad_request(format!(
                "password must contain between {MIN_PASSWORD_BYTES} and {MAX_PASSWORD_BYTES} bytes"
            )));
        }
        if self.is_provisioned() {
            self.verify(current_password).await?;
        }
        let password = new_password.to_owned();
        let hash = tokio::task::spawn_blocking(move || {
            Argon2::default()
                .hash_password(password.as_bytes())
                .map(|hash| hash.to_string())
        })
        .await
        .map_err(|error| AgentError::internal(format!("hashing task failed: {error}")))?
        .map_err(|error| AgentError::internal(format!("failed to hash the password: {error}")))?;
        write_verifier(&self.path, &hash).map_err(|error| {
            AgentError::internal(format!("failed to save the verifier: {error}"))
        })?;
        {
            let mut state = self.lock();
            state.file_modified = modified(&self.path);
            state.hash = Some(hash);
        }
        Ok(())
    }

    /// Picks up a verifier file that changed on disk (the `reset-password`
    /// command writes it while the Agent is running).
    fn refresh(&self) {
        if self.from_environment {
            return;
        }
        let current = modified(&self.path);
        let mut state = self.lock();
        if current.is_some() && current != state.file_modified {
            if let Some(hash) = read_verifier(&self.path) {
                state.hash = Some(hash);
                state.file_modified = current;
                state.failures.clear();
                state.locked_until = None;
            }
        }
    }

    fn record_failure(&self) {
        let now = Instant::now();
        let mut state = self.lock();
        state.failures.push_back(now);
        while state
            .failures
            .front()
            .is_some_and(|first| now.duration_since(*first) > FAILURE_WINDOW)
        {
            state.failures.pop_front();
        }
        let locked = state.failures.len() >= MAX_FAILURES;
        if locked {
            state.locked_until = Some(now + LOCK_DURATION);
            state.failures.clear();
        }
        drop(state);
        if locked {
            warn!("admin password locked after repeated failures");
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

async fn verify_hash(hash: String, password: String) -> bool {
    tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn read_verifier(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let hash = text.trim();
    PasswordHash::new(hash).ok()?;
    Some(hash.to_owned())
}

/// The admin hash in the Server's account file, which is either JSON
/// (`{"accounts": {"admin": {"password_hash": ...}}}`) or a bare hash from
/// before the account file had a structure.
fn read_account_hash(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    let hash = serde_json::from_str::<AccountFile>(text).map_or_else(
        |_| text.to_owned(),
        |file| {
            file.accounts
                .get("admin")
                .map(|account| account.password_hash.clone())
                .unwrap_or_default()
        },
    );
    PasswordHash::new(&hash).ok()?;
    Some(hash)
}

fn write_verifier(path: &Path, hash: &str) -> std::io::Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};

    let temporary = path.with_extension("tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(hash.as_bytes())?;
    file.write_all(b"\n")?;
    drop(file);
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use argon2::{Argon2, PasswordHasher};

    use super::Verifier;

    fn hash_of(password: &str) -> String {
        Argon2::default()
            .hash_password(password.as_bytes())
            .expect("hash")
            .to_string()
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("deckox-verifier-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[tokio::test]
    async fn accepts_the_right_password_and_rejects_others() {
        let dir = temp_dir("basic");
        let verifier = Verifier::load(
            dir.join("verifier"),
            &dir.join("none"),
            Some(hash_of("correct-horse-battery")),
        );
        assert!(verifier.is_provisioned());
        assert!(verifier.verify("correct-horse-battery").await.is_ok());
        assert!(verifier.verify("wrong").await.is_err());
    }

    #[tokio::test]
    async fn locks_after_repeated_failures_even_for_the_right_password() {
        let dir = temp_dir("lock");
        let verifier = Verifier::load(
            dir.join("verifier"),
            &dir.join("none"),
            Some(hash_of("correct-horse-battery")),
        );
        for _ in 0..5 {
            assert!(verifier.verify("guess").await.is_err());
        }
        assert!(verifier.locked_for().is_some());
        let error = verifier
            .verify("correct-horse-battery")
            .await
            .expect_err("locked");
        assert!(error.message().contains("try again"), "{}", error.message());
    }

    #[tokio::test]
    async fn without_any_hash_it_refuses_instead_of_accepting() {
        let dir = temp_dir("missing");
        let verifier = Verifier::load(dir.join("verifier"), &dir.join("none"), None);
        assert!(!verifier.is_provisioned());
        assert!(verifier.verify("anything").await.is_err());
    }

    #[tokio::test]
    async fn an_existing_installation_is_provisioned_from_the_account_file() {
        let dir = temp_dir("migrate");
        let account = dir.join("admin-password.hash");
        let hash = hash_of("correct-horse-battery");
        std::fs::write(
            &account,
            format!("{{\"accounts\":{{\"admin\":{{\"password_hash\":\"{hash}\"}}}}}}"),
        )
        .expect("write account");
        let verifier_path = dir.join("verifier");

        let verifier = Verifier::load(verifier_path.clone(), &account, None);
        assert!(verifier.verify("correct-horse-battery").await.is_ok());
        assert!(verifier_path.exists(), "the verifier file was created");

        let legacy = dir.join("legacy.hash");
        std::fs::write(&legacy, format!("{hash}\n")).expect("write legacy");
        let other = Verifier::load(dir.join("verifier2"), &legacy, None);
        assert!(
            other.verify("correct-horse-battery").await.is_ok(),
            "bare hash format works"
        );
    }

    #[tokio::test]
    async fn replacing_needs_the_current_password_and_survives_a_restart() {
        let dir = temp_dir("replace");
        let path = dir.join("verifier");
        std::fs::write(&path, format!("{}\n", hash_of("old-password-123"))).expect("seed");
        let verifier = Verifier::load(path.clone(), &dir.join("none"), None);

        assert!(
            verifier
                .replace("wrong-current-1", "new-password-1234")
                .await
                .is_err()
        );
        verifier
            .replace("old-password-123", "new-password-1234")
            .await
            .expect("replaced");
        assert!(verifier.verify("new-password-1234").await.is_ok());
        assert!(verifier.verify("old-password-123").await.is_err());

        let restarted = Verifier::load(path, &dir.join("none"), None);
        assert!(restarted.verify("new-password-1234").await.is_ok());
        assert!(
            verifier
                .replace("new-password-1234", "short")
                .await
                .is_err(),
            "a weak new password is refused"
        );
    }

    #[tokio::test]
    async fn a_verifier_file_changed_on_disk_is_picked_up() {
        let dir = temp_dir("refresh");
        let path = dir.join("verifier");
        std::fs::write(&path, format!("{}\n", hash_of("first-password-123"))).expect("seed");
        let verifier = Verifier::load(path.clone(), &dir.join("none"), None);
        assert!(verifier.verify("first-password-123").await.is_ok());

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, format!("{}\n", hash_of("second-password-123"))).expect("rewrite");
        assert!(
            verifier.verify("second-password-123").await.is_ok(),
            "reset-password takes effect without a restart"
        );
    }
}
