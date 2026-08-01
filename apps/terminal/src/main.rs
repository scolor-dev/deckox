use std::{
    env,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use deckox_protocol::TerminalStatus;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
use tokio::{
    net::UnixListener,
    sync::{Semaphore, mpsc},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

const DEFAULT_SOCKET: &str = "/run/deckox-terminal/terminal.sock";
const DEFAULT_SHELL: &str = "/bin/sh";
const DEFAULT_HOME: &str = "/var/lib/deckox-terminal";
const MAX_SESSIONS: usize = 2;
const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_MESSAGE_BYTES: usize = 16 * 1024;
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;
const ALLOWED_SHELLS: &[&str] = &["/bin/sh", "/bin/bash", "/usr/bin/bash"];

#[derive(Clone)]
struct TerminalManager {
    inner: Arc<TerminalInner>,
}

struct TerminalInner {
    enabled: bool,
    shell: PathBuf,
    home: PathBuf,
    sessions: Arc<Semaphore>,
}

#[derive(Serialize)]
struct TerminalError {
    code: &'static str,
    message: &'static str,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TerminalControl {
    Resize { cols: u16, rows: u16 },
}

enum PtyEvent {
    Output(Vec<u8>),
    Exited,
    Failed,
}

struct SpawnedTerminal {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    reader: Box<dyn Read + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[tokio::main]
async fn main() {
    init_tracing();
    let socket_path = PathBuf::from(
        env::var("DECKOX_TERMINAL_SOCKET").unwrap_or_else(|_| DEFAULT_SOCKET.to_owned()),
    );
    if !socket_path.is_absolute() {
        eprintln!("DECKOX_TERMINAL_SOCKET must be an absolute path");
        std::process::exit(2);
    }
    let manager = TerminalManager::from_env().unwrap_or_else(|message| {
        eprintln!("failed to load terminal configuration: {message}");
        std::process::exit(2);
    });
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).unwrap_or_else(|error| {
            eprintln!(
                "failed to remove stale socket {}: {error}",
                socket_path.display()
            );
            std::process::exit(1);
        });
    }
    let listener = UnixListener::bind(&socket_path).unwrap_or_else(|error| {
        eprintln!("failed to bind {}: {error}", socket_path.display());
        std::process::exit(1);
    });
    set_socket_permissions(&socket_path);

    let app = Router::new()
        .route("/v1/status", get(status))
        .route("/v1/ws", get(websocket))
        .with_state(manager);
    info!(socket = %socket_path.display(), "deckox terminal service started");
    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!(%error, "terminal service stopped unexpectedly");
    }
    let _ = std::fs::remove_file(socket_path);
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("deckox_terminal=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

impl TerminalManager {
    fn from_env() -> Result<Self, String> {
        let requested = env::var("DECKOX_TERMINAL_ENABLED").is_ok_and(|value| value == "true");
        let shell = PathBuf::from(
            env::var("DECKOX_TERMINAL_SHELL").unwrap_or_else(|_| DEFAULT_SHELL.to_owned()),
        );
        if !ALLOWED_SHELLS
            .iter()
            .any(|allowed| shell == Path::new(allowed))
        {
            return Err(format!(
                "DECKOX_TERMINAL_SHELL must be one of {}",
                ALLOWED_SHELLS.join(", ")
            ));
        }
        let home = PathBuf::from(
            env::var("DECKOX_TERMINAL_HOME").unwrap_or_else(|_| DEFAULT_HOME.to_owned()),
        );
        if !home.is_absolute() {
            return Err("DECKOX_TERMINAL_HOME must be an absolute path".to_owned());
        }
        let enabled = requested && process_is_non_root();
        if requested && !enabled {
            warn!(
                event = "terminal_disabled",
                reason = "root_process",
                "terminal disabled because the service is running as root"
            );
        }
        Ok(Self {
            inner: Arc::new(TerminalInner {
                enabled,
                shell,
                home,
                sessions: Arc::new(Semaphore::new(MAX_SESSIONS)),
            }),
        })
    }

    fn status(&self) -> TerminalStatus {
        TerminalStatus {
            enabled: self.inner.enabled,
            isolated: true,
            privileged: false,
            active_sessions: MAX_SESSIONS.saturating_sub(self.inner.sessions.available_permits()),
            max_sessions: MAX_SESSIONS,
            idle_timeout_seconds: IDLE_TIMEOUT.as_secs(),
        }
    }
}

async fn status(State(manager): State<TerminalManager>) -> Json<TerminalStatus> {
    Json(manager.status())
}

async fn websocket(
    State(manager): State<TerminalManager>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !manager.inner.enabled {
        return terminal_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "terminal_disabled",
            "web terminal is disabled",
        );
    }
    let Ok(permit) = manager.inner.sessions.clone().try_acquire_owned() else {
        return terminal_error(
            StatusCode::TOO_MANY_REQUESTS,
            "terminal_session_limit",
            "the terminal session limit has been reached",
        );
    };
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("internal")
        .to_owned();
    upgrade
        .max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            run_terminal(socket, manager, request_id).await;
        })
}

async fn run_terminal(mut socket: WebSocket, manager: TerminalManager, request_id: String) {
    let shell = manager.inner.shell.clone();
    let home = manager.inner.home.clone();
    let spawned = tokio::task::spawn_blocking(move || spawn_terminal(&shell, &home)).await;
    let Ok(Ok(terminal)) = spawned else {
        warn!(
            event = "terminal_session",
            request_id,
            result = "failure",
            reason = "spawn",
            "terminal could not start"
        );
        let _ = socket
            .send(Message::Text(
                r#"{"type":"error","code":"terminal_start_failed"}"#.into(),
            ))
            .await;
        return;
    };

    let mut killer = terminal.child.clone_killer();
    let (events_tx, mut events_rx) = mpsc::channel::<PtyEvent>(32);
    start_output_reader(terminal.reader, events_tx.clone());
    start_child_waiter(terminal.child, events_tx);
    info!(
        event = "terminal_session",
        request_id,
        result = "opened",
        "terminal opened"
    );
    if socket
        .send(Message::Text(r#"{"type":"ready"}"#.into()))
        .await
        .is_err()
    {
        let _ = killer.kill();
        return;
    }

    let idle_timer = tokio::time::sleep(IDLE_TIMEOUT);
    tokio::pin!(idle_timer);
    let close_reason = loop {
        tokio::select! {
            client_message = socket.recv() => {
                let Some(Ok(message)) = client_message else { break "disconnected"; };
                idle_timer.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                if !handle_client_message(message, &terminal.master, &terminal.writer).await {
                    break "client_closed";
                }
            }
            event = events_rx.recv() => match event {
                Some(PtyEvent::Output(output)) => {
                    if socket.send(Message::Binary(output.into())).await.is_err() { break "disconnected"; }
                }
                Some(PtyEvent::Exited) => {
                    let _ = socket.send(Message::Text(r#"{"type":"exit"}"#.into())).await;
                    break "shell_exited";
                }
                Some(PtyEvent::Failed) | None => {
                    let _ = socket.send(Message::Text(r#"{"type":"error","code":"terminal_io_failed"}"#.into())).await;
                    break "io_failed";
                }
            },
            () = &mut idle_timer => {
                let _ = socket.send(Message::Text(r#"{"type":"error","code":"terminal_idle_timeout"}"#.into())).await;
                break "idle_timeout";
            }
        }
    };
    let _ = killer.kill();
    let _ = socket.send(Message::Close(None)).await;
    info!(
        event = "terminal_session",
        request_id,
        result = "closed",
        reason = close_reason,
        "terminal closed"
    );
}

async fn handle_client_message(
    message: Message,
    master: &Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
) -> bool {
    match message {
        Message::Binary(input) if input.len() <= MAX_MESSAGE_BYTES => {
            let writer = Arc::clone(writer);
            tokio::task::spawn_blocking(move || {
                let mut writer = writer.lock().map_err(|_| ())?;
                writer.write_all(&input).map_err(|_| ())?;
                writer.flush().map_err(|_| ())
            })
            .await
            .is_ok_and(|result| result.is_ok())
        }
        Message::Text(control) if control.len() <= MAX_MESSAGE_BYTES => master
            .lock()
            .map_err(|_| ())
            .and_then(|master| apply_control(&control, master.as_ref()))
            .is_ok(),
        Message::Ping(payload) => payload.len() <= MAX_MESSAGE_BYTES,
        Message::Pong(_) => true,
        _ => false,
    }
}

fn apply_control(control: &str, master: &dyn MasterPty) -> Result<(), ()> {
    master.resize(parse_resize(control)?).map_err(|_| ())
}

fn parse_resize(control: &str) -> Result<PtySize, ()> {
    let TerminalControl::Resize { cols, rows } = serde_json::from_str(control).map_err(|_| ())?;
    if !(20..=400).contains(&cols) || !(5..=200).contains(&rows) {
        return Err(());
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

fn spawn_terminal(shell: &PathBuf, home: &PathBuf) -> Result<SpawnedTerminal, String> {
    let pair = native_pty_system()
        .openpty(PtySize::default())
        .map_err(|error| format!("failed to open PTY: {error}"))?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("failed to clone PTY reader: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("failed to open PTY writer: {error}"))?;
    let mut command = CommandBuilder::new(shell);
    command.arg("-l");
    command.env_clear();
    command.env("HOME", home);
    command.env("LANG", "C.UTF-8");
    command.env("PATH", "/usr/local/bin:/usr/bin:/bin");
    command.env("SHELL", shell);
    command.env("TERM", "xterm-256color");
    command.cwd(home);
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("failed to start shell: {error}"))?;
    drop(pair.slave);
    Ok(SpawnedTerminal {
        master: Arc::new(Mutex::new(pair.master)),
        reader,
        writer: Arc::new(Mutex::new(writer)),
        child,
    })
}

fn start_output_reader(mut reader: Box<dyn Read + Send>, events: mpsc::Sender<PtyEvent>) {
    std::thread::spawn(move || {
        let mut buffer = vec![0; OUTPUT_CHUNK_BYTES];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if events
                        .blocking_send(PtyEvent::Output(buffer[..read].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    let _ = events.blocking_send(PtyEvent::Failed);
                    break;
                }
            }
        }
    });
}

fn start_child_waiter(
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    events: mpsc::Sender<PtyEvent>,
) {
    std::thread::spawn(move || {
        let event = if child.wait().is_ok() {
            PtyEvent::Exited
        } else {
            PtyEvent::Failed
        };
        let _ = events.blocking_send(event);
    });
}

fn terminal_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(TerminalError { code, message })).into_response()
}

#[cfg(unix)]
fn process_is_non_root() -> bool {
    !rustix::process::geteuid().is_root()
}

#[cfg(not(unix))]
const fn process_is_non_root() -> bool {
    false
}

#[cfg(unix)]
fn set_socket_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(error) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660)) {
        eprintln!("failed to set socket permissions: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(unix))]
fn set_socket_permissions(_: &Path) {}

async fn shutdown_signal() {
    let interrupt = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { () = interrupt => {}, () = terminate => {} }
}

#[cfg(test)]
mod tests {
    use super::parse_resize;

    #[test]
    fn accepts_bounded_resize() {
        let size = parse_resize(r#"{"type":"resize","cols":120,"rows":40}"#)
            .expect("resize should be accepted");
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
    }

    #[test]
    fn rejects_oversized_resize() {
        assert!(parse_resize(r#"{"type":"resize","cols":1000,"rows":40}"#).is_err());
    }
}
