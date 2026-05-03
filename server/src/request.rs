use crate::response::ApiResponse;
use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const MAX_REQUEST_BODY_BYTES: u64 = 64 * 1024;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub started_at: Instant,
}

pub async fn attach_request_context(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(new_request_id);

    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        started_at: Instant::now(),
    });

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    set_security_headers(response.headers_mut());
    response
}

pub async fn enforce_request_timeout(request: Request, next: Next) -> Response {
    match tokio::time::timeout(Duration::from_secs(10), next.run(request)).await {
        Ok(response) => response,
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            Json(ApiResponse::error("request timed out", "REQUEST_TIMEOUT")),
        )
            .into_response(),
    }
}

pub async fn reject_oversized_payload(request: Request, next: Next) -> Response {
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_REQUEST_BODY_BYTES)
    {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiResponse::error(
                "request body too large",
                "PAYLOAD_TOO_LARGE",
            )),
        )
            .into_response();
    }

    next.run(request).await
}

fn new_request_id() -> String {
    let count = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("req-{nanos:x}-{count:x}")
}

fn set_security_headers(headers: &mut axum::http::HeaderMap) {
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
}
