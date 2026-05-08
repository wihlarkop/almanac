use crate::response::ApiResponse;
use axum::{
    extract::{ConnectInfo, Request},
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
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

// --- Rate limiting ---

const CLEANUP_THRESHOLD: usize = 50_000;

struct Window {
    count: u32,
    started_at: Instant,
}

pub struct RateLimiter {
    map: Arc<Mutex<HashMap<IpAddr, Window>>>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            map: Arc::new(Mutex::new(HashMap::new())),
            limit: requests_per_second,
            window: Duration::from_secs(1),
        }
    }

    pub fn check(&self, ip: IpAddr) -> RateLimitResult {
        let now = Instant::now();
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());

        if map.len() >= CLEANUP_THRESHOLD {
            let window = self.window;
            map.retain(|_, w| now.duration_since(w.started_at) < window);
        }

        let window = map.entry(ip).or_insert(Window { count: 0, started_at: now });

        if now.duration_since(window.started_at) >= self.window {
            window.count = 1;
            window.started_at = now;
            RateLimitResult::Allowed { remaining: self.limit - 1 }
        } else if window.count < self.limit {
            window.count += 1;
            RateLimitResult::Allowed { remaining: self.limit - window.count }
        } else {
            let retry_after = self.window.saturating_sub(now.duration_since(window.started_at));
            RateLimitResult::Limited { retry_after_secs: retry_after.as_secs().max(1) }
        }
    }
}

pub enum RateLimitResult {
    Allowed { remaining: u32 },
    Limited { retry_after_secs: u64 },
}

pub async fn enforce_rate_limit(
    limiter: Arc<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = extract_client_ip(&request);
    match limiter.check(ip) {
        RateLimitResult::Allowed { remaining } => {
            let mut response = next.run(request).await;
            if let Ok(value) = HeaderValue::from_str(&remaining.to_string()) {
                response
                    .headers_mut()
                    .insert("x-ratelimit-remaining", value);
            }
            response
        }
        RateLimitResult::Limited { retry_after_secs } => {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiResponse::error("rate limit exceeded", "RATE_LIMIT_EXCEEDED")),
            )
                .into_response();
            if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
                response.headers_mut().insert("retry-after", value);
            }
            response
        }
    }
}

fn extract_client_ip(request: &Request) -> IpAddr {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<IpAddr>().ok())
        })
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

// --- Security headers ---

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
