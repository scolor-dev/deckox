use std::{env, net::IpAddr, path::PathBuf, time::Duration};

use axum::{
    Json,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use deckox_protocol::TerminalStatus;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use tokio_tungstenite::{
    WebSocketStream, client_async,
    tungstenite::{self, client::IntoClientRequest},
};
use tracing::{info, warn};

use crate::{auth::AuthenticatedUser, request_context::RequestId};

const DEFAULT_SOCKET: &str = "/run/deckox-terminal/terminal.sock";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub struct TerminalClient {
    socket_path: PathBuf,
}

#[derive(Serialize)]
struct TerminalError {
    code: &'static str,
    message: &'static str,
}

impl TerminalClient {
    pub fn from_env() -> Result<Self, String> {
        let socket_path = PathBuf::from(
            env::var("DECKOX_TERMINAL_SOCKET").unwrap_or_else(|_| DEFAULT_SOCKET.to_owned()),
        );
        if !socket_path.is_absolute() {
            return Err("DECKOX_TERMINAL_SOCKET must be an absolute path".to_owned());
        }
        Ok(Self { socket_path })
    }

    async fn connect_websocket(
        &self,
        request_id: &str,
    ) -> Result<WebSocketStream<UnixStream>, tungstenite::Error> {
        let stream = tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(&self.socket_path))
            .await
            .map_err(|_| tungstenite::Error::ConnectionClosed)?
            .map_err(tungstenite::Error::Io)?;
        let mut request = "ws://deckox-terminal/v1/ws".into_client_request()?;
        if let Ok(value) = request_id.parse() {
            request.headers_mut().insert("x-request-id", value);
        }
        let (socket, _) = tokio::time::timeout(CONNECT_TIMEOUT, client_async(request, stream))
            .await
            .map_err(|_| tungstenite::Error::ConnectionClosed)??;
        Ok(socket)
    }

    async fn status(&self) -> Result<TerminalStatus, String> {
        let mut stream =
            tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(&self.socket_path))
                .await
                .map_err(|_| "terminal service connection timed out".to_owned())?
                .map_err(|error| format!("terminal service unavailable: {error}"))?;
        stream
            .write_all(
                b"GET /v1/status HTTP/1.1\r\nHost: deckox-terminal\r\nConnection: close\r\n\r\n",
            )
            .await
            .map_err(|error| format!("failed to request terminal status: {error}"))?;
        let mut response = Vec::new();
        tokio::time::timeout(CONNECT_TIMEOUT, stream.read_to_end(&mut response))
            .await
            .map_err(|_| "terminal status response timed out".to_owned())?
            .map_err(|error| format!("failed to read terminal status: {error}"))?;
        parse_status_response(&response)
    }
}

pub async fn status(State(client): State<TerminalClient>) -> Response {
    match client.status().await {
        Ok(status) => Json(status).into_response(),
        Err(error) => {
            warn!(event = "terminal_status", %error, "terminal service unavailable");
            terminal_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal_unavailable",
                "terminal service is unavailable",
            )
        }
    }
}

pub async fn websocket(
    State(client): State<TerminalClient>,
    axum::Extension(request_id): axum::Extension<RequestId>,
    axum::Extension(user): axum::Extension<AuthenticatedUser>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let internal = match client.connect_websocket(&request_id.0).await {
        Ok(socket) => socket,
        Err(tungstenite::Error::Http(response))
            if response.status() == StatusCode::TOO_MANY_REQUESTS =>
        {
            return terminal_error(
                StatusCode::TOO_MANY_REQUESTS,
                "terminal_session_limit",
                "the terminal session limit has been reached",
            );
        }
        Err(tungstenite::Error::Http(response))
            if response.status() == StatusCode::SERVICE_UNAVAILABLE =>
        {
            return terminal_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal_disabled",
                "web terminal is disabled",
            );
        }
        Err(error) => {
            warn!(event = "terminal_connect", request_id = %request_id.0, %error, "terminal service connection failed");
            return terminal_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "terminal_unavailable",
                "terminal service is unavailable",
            );
        }
    };
    upgrade
        .max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |browser| bridge(browser, internal, request_id.0, user.source_ip))
}

async fn bridge(
    browser: WebSocket,
    internal: WebSocketStream<UnixStream>,
    request_id: String,
    source_ip: IpAddr,
) {
    info!(event = "terminal_session", request_id, actor = "admin", source_ip = %source_ip, result = "opened", "web terminal proxy opened");
    let (mut browser_tx, mut browser_rx) = browser.split();
    let (mut internal_tx, mut internal_rx) = internal.split();
    let reason = loop {
        tokio::select! {
            message = browser_rx.next() => {
                let Some(Ok(message)) = message else { break "browser_disconnected"; };
                let Some(message) = to_internal(message) else { break "browser_closed"; };
                if internal_tx.send(message).await.is_err() { break "terminal_disconnected"; }
            }
            message = internal_rx.next() => {
                let Some(Ok(message)) = message else { break "terminal_disconnected"; };
                let Some(message) = to_browser(message) else { break "terminal_closed"; };
                if browser_tx.send(message).await.is_err() { break "browser_disconnected"; }
            }
        }
    };
    let _ = internal_tx.close().await;
    let _ = browser_tx.send(Message::Close(None)).await;
    info!(event = "terminal_session", request_id, actor = "admin", source_ip = %source_ip, result = "closed", reason, "web terminal proxy closed");
}

fn to_internal(message: Message) -> Option<tungstenite::Message> {
    match message {
        Message::Text(value) => Some(tungstenite::Message::Text(value.to_string().into())),
        Message::Binary(value) => Some(tungstenite::Message::Binary(value)),
        Message::Ping(value) => Some(tungstenite::Message::Ping(value)),
        Message::Pong(value) => Some(tungstenite::Message::Pong(value)),
        Message::Close(_) => None,
    }
}

fn to_browser(message: tungstenite::Message) -> Option<Message> {
    match message {
        tungstenite::Message::Text(value) => Some(Message::Text(value.to_string().into())),
        tungstenite::Message::Binary(value) => Some(Message::Binary(value)),
        tungstenite::Message::Ping(value) => Some(Message::Ping(value)),
        tungstenite::Message::Pong(value) => Some(Message::Pong(value)),
        tungstenite::Message::Close(_) | tungstenite::Message::Frame(_) => None,
    }
}

fn parse_status_response(response: &[u8]) -> Result<TerminalStatus, String> {
    let separator = b"\r\n\r\n";
    let body_start = response
        .windows(separator.len())
        .position(|window| window == separator)
        .map(|position| position + separator.len())
        .ok_or_else(|| "invalid terminal status response".to_owned())?;
    let status_line = response
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    if !status_line.starts_with(b"HTTP/1.1 200") {
        return Err("terminal service returned an error".to_owned());
    }
    serde_json::from_slice(&response[body_start..])
        .map_err(|error| format!("invalid terminal status response: {error}"))
}

fn terminal_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(TerminalError { code, message })).into_response()
}

#[cfg(test)]
mod tests {
    use super::parse_status_response;

    #[test]
    fn parses_terminal_status() {
        let response = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"enabled\":true,\"isolated\":true,\"privileged\":false,\"active_sessions\":0,\"max_sessions\":2,\"idle_timeout_seconds\":900}";
        let status = parse_status_response(response).expect("status should parse");
        assert!(status.enabled);
        assert!(status.isolated);
    }
}
