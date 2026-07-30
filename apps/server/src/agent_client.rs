use std::{path::PathBuf, time::Duration};

use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[derive(Clone)]
pub struct AgentClient {
    socket_path: PathBuf,
}

pub struct AgentResponse {
    pub status: StatusCode,
    pub body: Value,
}

impl AgentClient {
    pub const fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub async fn request(&self, method: &str, path: &str) -> Result<AgentResponse, String> {
        let mut stream = tokio::time::timeout(
            Duration::from_secs(2),
            UnixStream::connect(&self.socket_path),
        )
        .await
        .map_err(|_| "agent connection timed out".to_owned())?
        .map_err(|error| format!("agent unavailable: {error}"))?;

        let request =
            format!("{method} {path} HTTP/1.1\r\nHost: agent\r\nConnection: close\r\n\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|error| format!("failed to request Agent API: {error}"))?;

        let mut response = Vec::with_capacity(4096);
        tokio::time::timeout(Duration::from_secs(35), stream.read_to_end(&mut response))
            .await
            .map_err(|_| "agent response timed out".to_owned())?
            .map_err(|error| format!("failed to read Agent API: {error}"))?;

        parse_response(&response)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self.request("GET", path).await?;
        if !response.status.is_success() {
            return Err(format!(
                "Agent API returned HTTP {}",
                response.status.as_u16()
            ));
        }
        serde_json::from_value(response.body)
            .map_err(|error| format!("invalid Agent API response: {error}"))
    }
}

fn parse_response(response: &[u8]) -> Result<AgentResponse, String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "invalid HTTP response from Agent".to_owned())?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|_| "Agent returned invalid HTTP headers".to_owned())?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .and_then(|value| StatusCode::from_u16(value).ok())
        .ok_or_else(|| "Agent returned an invalid HTTP status".to_owned())?;
    let body = serde_json::from_slice(&response[header_end + 4..])
        .map_err(|error| format!("Agent returned invalid JSON: {error}"))?;

    Ok(AgentResponse { status, body })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::parse_response;

    #[test]
    fn parses_agent_http_response() {
        let response = parse_response(
            b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 15\r\n\r\n{\"status\":\"ok\"}",
        )
        .expect("response should parse");

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body["status"], "ok");
    }

    #[test]
    fn preserves_agent_error_status() {
        let response = parse_response(
            b"HTTP/1.1 403 Forbidden\r\ncontent-type: application/json\r\n\r\n{\"code\":\"forbidden\",\"message\":\"denied\"}",
        )
        .expect("response should parse");

        assert_eq!(response.status, StatusCode::FORBIDDEN);
        assert_eq!(response.body["code"], "forbidden");
    }
}
