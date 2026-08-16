use axum::{
    extract::{Request, State},
    http::{
        HeaderName, HeaderValue,
        header::{
            CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
            X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
        },
    },
    middleware::Next,
    response::Response,
};

/// Deckox serves no third-party scripts, styles, fonts, or images — the Vue
/// bundle is fully self-contained and every API call is same-origin — so
/// every fetch directive can be locked to `'self'`.
const CONTENT_SECURITY_POLICY_VALUE: &str = "default-src 'self'; script-src 'self'; \
     style-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; \
     frame-ancestors 'none'; base-uri 'self'; form-action 'self'";

/// Adds baseline security headers to every response — API, SPA fallback,
/// and static assets alike, since this runs as the outermost layer after
/// the route handler. `secure` mirrors `AuthManager::secure_cookie` (set
/// via `DECKOX_SECURE_COOKIE`): `Strict-Transport-Security` is only ever
/// sent when a TLS-terminating proxy is known to sit in front of Deckox,
/// since sending it over plain HTTP would wrongly promise TLS.
pub async fn apply(State(secure): State<bool>, request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY_VALUE),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), camera=(), microphone=()"),
    );
    if secure {
        headers.insert(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        );
    }
    response
}
