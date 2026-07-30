use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
pub const AGENT_REQUEST_ID_HEADER: &str = "x-deckox-request-id";

#[derive(Clone)]
pub struct RequestId(pub String);

pub async fn assign_request_id(mut request: Request, next: Next) -> Response {
    let request_id = RequestId(format!("req-{}", hex::encode(rand::random::<[u8; 16]>())));
    request.extensions_mut().insert(request_id.clone());

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id.0) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}
