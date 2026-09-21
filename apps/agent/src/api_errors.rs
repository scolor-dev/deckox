//! Makes every error the Agent sends look the same: `{"code": ..., "message": ...}`.
//!
//! Handlers already answer that way (see `error.rs`), but errors that the
//! framework raises before a handler runs (a malformed JSON body, a wrong
//! method) are plain text. This turns those into the standard shape too.

use axum::{
    Json,
    body::to_bytes,
    extract::Request,
    http::{StatusCode, header::CONTENT_TYPE},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

const MAX_MESSAGE_BYTES: usize = 300;

pub fn code_for(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
        StatusCode::UNPROCESSABLE_ENTITY => "invalid_request",
        StatusCode::TOO_MANY_REQUESTS => "rate_limited",
        status if status.is_server_error() => "internal_error",
        _ => "error",
    }
}

pub async fn normalize(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let is_plain = content_type
        .as_deref()
        .is_none_or(|value| value.starts_with("text/plain"));
    if !is_plain {
        return response;
    }

    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 4096).await.unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes);
    let mut message = text
        .trim()
        .chars()
        .take(MAX_MESSAGE_BYTES)
        .collect::<String>();
    if message.is_empty() {
        status
            .canonical_reason()
            .unwrap_or("error")
            .clone_into(&mut message);
    }
    let mut rebuilt = (
        status,
        Json(json!({"code": code_for(status), "message": message})),
    )
        .into_response();
    for (name, value) in &parts.headers {
        if name != CONTENT_TYPE && name != axum::http::header::CONTENT_LENGTH {
            rebuilt.headers_mut().append(name.clone(), value.clone());
        }
    }
    rebuilt
}

#[cfg(test)]
mod tests {
    use axum::{Json, Router, middleware, routing::post};
    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::normalize;

    #[tokio::test]
    async fn a_malformed_body_gets_the_standard_error_shape() {
        let app = Router::new()
            .route("/json", post(|Json(_): Json<Value>| async { "ok" }))
            .layer(middleware::from_fn(normalize));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let mut stream = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect");
        let body = "{not json";
        let request = format!(
            "POST /json HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.expect("write");
        let mut response = String::new();
        stream.read_to_string(&mut response).await.expect("read");

        let payload = response.split("\r\n\r\n").nth(1).expect("body");
        let json: Value = serde_json::from_str(payload.trim()).expect("json");
        assert_eq!(json["code"], "bad_request", "{response}");
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
    }
}
