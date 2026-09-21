//! Makes every error the API sends look the same: `{"code": ..., "message": ...}`.
//!
//! Handlers already answer that way, but errors that the framework raises
//! before a handler runs (a malformed JSON body, a wrong method, an oversized
//! request) are plain text. This turns those into the standard shape too.

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
        StatusCode::UNAUTHORIZED => "authentication_required",
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
    use axum::{Json, Router, middleware, routing::get, routing::post};
    use serde_json::Value;

    use super::normalize;

    async fn start() -> std::net::SocketAddr {
        let app = Router::new()
            .route("/json", post(|Json(_): Json<Value>| async { "ok" }))
            .route(
                "/gone",
                get(|| async { (axum::http::StatusCode::NOT_FOUND, "no such thing") }),
            )
            .route(
                "/already",
                get(|| async {
                    (
                        axum::http::StatusCode::CONFLICT,
                        Json(serde_json::json!({"code": "conflict", "message": "mine"})),
                    )
                }),
            )
            .layer(middleware::from_fn(normalize));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        address
    }

    async fn body(response: reqwest::Response) -> (u16, Value) {
        let status = response.status().as_u16();
        (
            status,
            serde_json::from_str(&response.text().await.expect("text")).expect("json"),
        )
    }

    #[tokio::test]
    async fn framework_errors_get_the_standard_shape() {
        let address = start().await;
        let client = reqwest::Client::new();

        let malformed = client
            .post(format!("http://{address}/json"))
            .header("content-type", "application/json")
            .body("{not json")
            .send()
            .await
            .expect("request");
        let (status, json) = body(malformed).await;
        assert!((400..500).contains(&status));
        assert!(
            json["code"].is_string()
                && json["message"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty()),
            "{json}"
        );

        let wrong_method = client
            .post(format!("http://{address}/gone"))
            .send()
            .await
            .expect("request");
        let (status, json) = body(wrong_method).await;
        assert_eq!(status, 405);
        assert_eq!(json["code"], "method_not_allowed");

        let plain = client
            .get(format!("http://{address}/gone"))
            .send()
            .await
            .expect("request");
        let (status, json) = body(plain).await;
        assert_eq!(
            (status, json["code"].as_str(), json["message"].as_str()),
            (404, Some("not_found"), Some("no such thing"))
        );
    }

    #[tokio::test]
    async fn errors_that_are_already_standard_are_left_alone() {
        let address = start().await;
        let response = reqwest::get(format!("http://{address}/already"))
            .await
            .expect("request");
        let (status, json) = body(response).await;
        assert_eq!(status, 409);
        assert_eq!(json["message"], "mine");
    }
}
