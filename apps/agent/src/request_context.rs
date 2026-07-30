use axum::{extract::Request, middleware::Next, response::Response};

pub const AGENT_REQUEST_ID_HEADER: &str = "x-deckox-request-id";

#[derive(Clone)]
pub struct RequestId(pub String);

pub async fn assign_request_id(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(AGENT_REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 80
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        .map_or_else(
            || format!("agent-{}", hex::encode(rand::random::<[u8; 16]>())),
            str::to_owned,
        );

    request.extensions_mut().insert(RequestId(request_id));
    next.run(request).await
}
