use crate::{response::ApiResponse, state::AppState};
use axum::{
    extract::{ConnectInfo, Request, State},
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
use time::OffsetDateTime;
use tokio::sync::RwLock;

pub const MAX_REQUEST_BODY_BYTES: u64 = 64 * 1024;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub started_at: Instant,
}

pub async fn attach_request_context(mut request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let query = request
        .uri()
        .query()
        .filter(|q| !q.is_empty())
        .map(ToOwned::to_owned);
    let ip = extract_client_ip(&request);
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(new_request_id);

    let started_at = Instant::now();
    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        started_at,
    });

    let mut response = next.run(request).await;
    let status = response.status().as_u16();
    let latency_ms = started_at.elapsed().as_secs_f64() * 1000.0;
    match status {
        500..=599 => tracing::error!(
            request_id = %request_id,
            method = %method,
            path = %path,
            query = query.as_deref().unwrap_or(""),
            ip = %ip,
            user_agent = user_agent.as_deref().unwrap_or(""),
            status,
            latency_ms,
            "request completed"
        ),
        400..=499 => tracing::warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            query = query.as_deref().unwrap_or(""),
            ip = %ip,
            user_agent = user_agent.as_deref().unwrap_or(""),
            status,
            latency_ms,
            "request completed"
        ),
        _ => tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            query = query.as_deref().unwrap_or(""),
            ip = %ip,
            user_agent = user_agent.as_deref().unwrap_or(""),
            status,
            latency_ms,
            "request completed"
        ),
    }

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    set_security_headers(response.headers_mut());
    response
}

pub async fn enforce_request_timeout(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    match tokio::time::timeout(Duration::from_secs(10), next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            tracing::warn!(path = %path, timeout_secs = 10, "request timed out");
            (
                StatusCode::REQUEST_TIMEOUT,
                Json(ApiResponse::error("request timed out", "REQUEST_TIMEOUT")),
            )
                .into_response()
        }
    }
}

pub async fn reject_oversized_payload(request: Request, next: Next) -> Response {
    let content_length = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    if content_length.is_some_and(|length| length > MAX_REQUEST_BODY_BYTES) {
        let ip = extract_client_ip(&request);
        let path = request.uri().path().to_string();
        tracing::warn!(
            ip = %ip,
            path = %path,
            content_length = content_length.unwrap_or(0),
            limit = MAX_REQUEST_BODY_BYTES,
            "request body too large"
        );
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

pub struct MemoryRateLimiter {
    map: Arc<Mutex<HashMap<IpAddr, Window>>>,
    limit: u32,
    window: Duration,
}

impl MemoryRateLimiter {
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

        let window = map.entry(ip).or_insert(Window {
            count: 0,
            started_at: now,
        });

        if now.duration_since(window.started_at) >= self.window {
            window.count = 1;
            window.started_at = now;
            RateLimitResult::Allowed {
                remaining: self.limit - 1,
            }
        } else if window.count < self.limit {
            window.count += 1;
            RateLimitResult::Allowed {
                remaining: self.limit - window.count,
            }
        } else {
            let retry_after = self
                .window
                .saturating_sub(now.duration_since(window.started_at));
            RateLimitResult::Limited {
                retry_after_secs: retry_after.as_secs().max(1),
            }
        }
    }
}

pub struct RedisRateLimiter {
    conn: redis::aio::ConnectionManager,
    limit: u32,
    window_secs: u64,
}

impl RedisRateLimiter {
    pub async fn new(client: &redis::Client, limit: u32, window_secs: u64) -> anyhow::Result<Self> {
        let conn = client.get_connection_manager().await?;
        Ok(Self {
            conn,
            limit,
            window_secs,
        })
    }

    pub async fn check(&self, ip: IpAddr) -> RateLimitResult {
        // Atomic fixed-window counter: INCR sets TTL only on first call in window.
        let script = redis::Script::new(
            r#"
            local current = redis.call('INCR', KEYS[1])
            if current == 1 then
                redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            local ttl = redis.call('TTL', KEYS[1])
            return {current, ttl}
            "#,
        );
        let key = format!("almanac:rl:{ip}");
        let mut conn = self.conn.clone();
        match script
            .key(&key)
            .arg(self.window_secs)
            .invoke_async::<(i64, i64)>(&mut conn)
            .await
        {
            Ok((count, ttl)) => {
                let count = count as u32;
                if count <= self.limit {
                    RateLimitResult::Allowed {
                        remaining: self.limit.saturating_sub(count),
                    }
                } else {
                    RateLimitResult::Limited {
                        retry_after_secs: ttl.max(1) as u64,
                    }
                }
            }
            // Fail open: if Redis is unavailable, allow the request.
            Err(_) => RateLimitResult::Allowed {
                remaining: self.limit,
            },
        }
    }
}

pub enum RateLimiter {
    Memory(MemoryRateLimiter),
    Redis(RedisRateLimiter),
}

impl RateLimiter {
    pub async fn check(&self, ip: IpAddr) -> RateLimitResult {
        match self {
            Self::Memory(m) => m.check(ip),
            Self::Redis(r) => r.check(ip).await,
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
    match limiter.check(ip).await {
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
            let path = request.uri().path().to_string();
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .filter(|v| !v.trim().is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(new_request_id);
            tracing::warn!(
                request_id = %request_id,
                ip = %ip,
                path = %path,
                retry_after_secs,
                "rate limit exceeded"
            );
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiResponse::error(
                    "rate limit exceeded",
                    "RATE_LIMIT_EXCEEDED",
                )),
            )
                .into_response();
            if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
                response.headers_mut().insert("retry-after", value);
            }
            if let Ok(value) = HeaderValue::from_str(&request_id) {
                response.headers_mut().insert("x-request-id", value);
            }
            response
        }
    }
}

fn extract_client_ip(request: &Request) -> IpAddr {
    if trust_proxy_headers()
        && let Some(ip) = request
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
    {
        return ip;
    }

    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

fn trust_proxy_headers() -> bool {
    std::env::var("TRUST_PROXY_HEADERS").is_ok_and(|value| value.eq_ignore_ascii_case("true"))
}

pub async fn handle_method_not_allowed(response: Response) -> Response {
    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            Json(ApiResponse::error(
                "method not allowed",
                "METHOD_NOT_ALLOWED",
            )),
        )
            .into_response();
    }
    response
}

// --- Cache headers middleware ---

pub async fn inject_cache_headers(
    State(state): State<Arc<RwLock<AppState>>>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let mut response = next.run(request).await;

    if method != axum::http::Method::GET || !response.status().is_success() {
        return response;
    }

    let (etag, loaded_at) = {
        let s = state.read().await;
        (s.etag.clone(), s.loaded_at)
    };

    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300, stale-while-revalidate=60"),
    );
    if let Ok(value) = HeaderValue::from_str(&etag) {
        headers.insert(header::ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(&format_http_date(loaded_at)) {
        headers.insert(header::LAST_MODIFIED, value);
    }
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));

    response
}

fn format_http_date(dt: OffsetDateTime) -> String {
    let weekday = match dt.weekday() {
        time::Weekday::Monday => "Mon",
        time::Weekday::Tuesday => "Tue",
        time::Weekday::Wednesday => "Wed",
        time::Weekday::Thursday => "Thu",
        time::Weekday::Friday => "Fri",
        time::Weekday::Saturday => "Sat",
        time::Weekday::Sunday => "Sun",
    };
    let month = match dt.month() {
        time::Month::January => "Jan",
        time::Month::February => "Feb",
        time::Month::March => "Mar",
        time::Month::April => "Apr",
        time::Month::May => "May",
        time::Month::June => "Jun",
        time::Month::July => "Jul",
        time::Month::August => "Aug",
        time::Month::September => "Sep",
        time::Month::October => "Oct",
        time::Month::November => "Nov",
        time::Month::December => "Dec",
    };
    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday,
        dt.day(),
        month,
        dt.year(),
        dt.hour(),
        dt.minute(),
        dt.second(),
    )
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
