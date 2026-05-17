use hex;
use redis::aio::ConnectionManager;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// A cached HTTP response: headers and serialized JSON body.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedEntry {
    pub headers: Vec<(String, String)>,
    pub body: String,
}

// --- Memory backend ---

struct MemoryCache {
    map: Mutex<HashMap<String, (CachedEntry, Instant)>>,
}

impl MemoryCache {
    fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    fn get(&self, key: &str) -> Option<CachedEntry> {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(key) {
            Some((entry, expires_at)) if *expires_at > Instant::now() => Some(entry.clone()),
            _ => {
                map.remove(key);
                None
            }
        }
    }

    fn set(&self, key: &str, entry: &CachedEntry, ttl: Duration) {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(key.to_string(), (entry.clone(), Instant::now() + ttl));
    }
}

// --- Redis backend ---

struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    async fn get(&self, key: &str) -> Option<CachedEntry> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .ok()?;
        raw.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn set(&self, key: &str, entry: &CachedEntry, ttl: Duration) {
        let Ok(serialized) = serde_json::to_string(entry) else {
            return;
        };
        let secs = ttl.as_secs().max(1);
        let mut conn = self.conn.clone();
        let _: redis::RedisResult<()> = redis::cmd("SET")
            .arg(key)
            .arg(serialized)
            .arg("EX")
            .arg(secs)
            .query_async(&mut conn)
            .await;
    }
}

// --- Public enum ---

enum CacheInner {
    Noop,
    Memory(MemoryCache),
    Redis(RedisCache),
}

pub struct Cache {
    inner: CacheInner,
    ttl: Duration,
}

impl Cache {
    pub fn noop() -> Self {
        Self {
            inner: CacheInner::Noop,
            ttl: Duration::ZERO,
        }
    }

    pub fn memory(ttl: Duration) -> Self {
        Self {
            inner: CacheInner::Memory(MemoryCache::new()),
            ttl,
        }
    }

    pub fn redis(conn: ConnectionManager, ttl: Duration) -> Self {
        Self {
            inner: CacheInner::Redis(RedisCache::new(conn)),
            ttl,
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub async fn get(&self, key: &str) -> Option<CachedEntry> {
        match &self.inner {
            CacheInner::Noop => None,
            CacheInner::Memory(c) => c.get(key),
            CacheInner::Redis(c) => c.get(key).await,
        }
    }

    pub async fn set(&self, key: &str, entry: &CachedEntry, ttl: Duration) {
        match &self.inner {
            CacheInner::Noop => {}
            CacheInner::Memory(c) => c.set(key, entry, ttl),
            CacheInner::Redis(c) => c.set(key, entry, ttl).await,
        }
    }
}

// --- Cache key ---

/// Builds a deterministic cache key. Query params are sorted so param order doesn't affect the key.
/// The ETag is embedded so cache entries auto-invalidate when the catalog is reloaded via SIGHUP.
pub fn build_cache_key(etag: &str, path: &str, query: &str) -> String {
    let mut pairs: Vec<&str> = query.split('&').filter(|s| !s.is_empty()).collect();
    pairs.sort_unstable();
    let normalized = pairs.join("&");

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let query_hash = hex::encode(hasher.finalize());

    let etag_clean = etag.trim_matches('"');
    format!("almanac:{etag_clean}:{path}:{query_hash}")
}

// --- Cache middleware ---

use crate::state::AppState;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use std::str::FromStr;
use tokio::sync::RwLock;

/// Tower middleware: intercepts cacheable GET requests, returns cached responses on HIT,
/// stores new responses on MISS. Non-cacheable requests pass through unchanged.
pub async fn cache_request(
    cache: Arc<Cache>,
    state: Arc<RwLock<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let is_get = request.method() == Method::GET;
    let path = request.uri().path().to_string();
    // Cache all /api/v1/ GET routes except the health liveness endpoint.
    let is_cacheable = is_get
        && path.starts_with("/api/v1/")
        && path != "/api/v1/health";

    if !is_cacheable {
        return next.run(request).await;
    }

    let query = request.uri().query().unwrap_or("").to_string();
    let etag = state.read().await.etag.clone();
    let key = build_cache_key(&etag, &path, &query);

    if let Some(entry) = cache.get(&key).await {
        let mut response = Response::new(Body::from(entry.body));
        *response.status_mut() = StatusCode::OK;
        for (name, value) in &entry.headers {
            if let (Ok(n), Ok(v)) = (HeaderName::from_str(name), HeaderValue::from_str(value)) {
                response.headers_mut().insert(n, v);
            }
        }
        response.headers_mut().insert(
            HeaderName::from_static("x-cache"),
            HeaderValue::from_static("HIT"),
        );
        return response;
    }

    let response = next.run(request).await;

    if response.status() == StatusCode::OK {
        let (parts, body) = response.into_parts();
        match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
            Ok(bytes) => {
                let body_str = String::from_utf8_lossy(&bytes).into_owned();
                let headers: Vec<(String, String)> = parts
                    .headers
                    .iter()
                    .filter_map(|(name, value)| {
                        value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
                    })
                    .collect();
                let entry = CachedEntry { headers, body: body_str };
                cache.set(&key, &entry, cache.ttl()).await;

                let mut rebuilt = Response::from_parts(parts, Body::from(bytes));
                rebuilt.headers_mut().insert(
                    HeaderName::from_static("x-cache"),
                    HeaderValue::from_static("MISS"),
                );
                rebuilt
            }
            Err(_) => Response::from_parts(parts, Body::empty()),
        }
    } else {
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn noop_always_returns_none() {
        let cache = Cache::noop();
        cache
            .set(
                "k",
                &CachedEntry { headers: vec![], body: "v".into() },
                Duration::from_secs(60),
            )
            .await;
        assert!(cache.get("k").await.is_none());
    }

    #[tokio::test]
    async fn memory_miss_returns_none() {
        let cache = Cache::memory(Duration::from_secs(60));
        assert!(cache.get("missing").await.is_none());
    }

    #[tokio::test]
    async fn memory_stores_and_retrieves_entry() {
        let cache = Cache::memory(Duration::from_secs(60));
        let entry = CachedEntry {
            headers: vec![("content-type".into(), "application/json".into())],
            body: r#"{"ok":true}"#.into(),
        };
        cache.set("k", &entry, Duration::from_secs(60)).await;
        let got = cache.get("k").await.expect("should be Some");
        assert_eq!(got.body, r#"{"ok":true}"#);
        assert_eq!(got.headers[0].0, "content-type");
    }

    #[tokio::test]
    async fn memory_entry_expires() {
        let cache = Cache::memory(Duration::from_secs(60));
        let entry = CachedEntry { headers: vec![], body: "val".into() };
        cache.set("k", &entry, Duration::from_millis(1)).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(cache.get("k").await.is_none());
    }

    #[test]
    fn build_cache_key_normalizes_query_order() {
        let k1 = build_cache_key("etag1", "/api/v1/models", "b=2&a=1");
        let k2 = build_cache_key("etag1", "/api/v1/models", "a=1&b=2");
        assert_eq!(k1, k2);
    }

    #[test]
    fn build_cache_key_differs_by_etag() {
        let k1 = build_cache_key("etag1", "/api/v1/models", "");
        let k2 = build_cache_key("etag2", "/api/v1/models", "");
        assert_ne!(k1, k2);
    }

    #[test]
    fn build_cache_key_differs_by_path() {
        let k1 = build_cache_key("etag1", "/api/v1/models", "");
        let k2 = build_cache_key("etag1", "/api/v1/providers", "");
        assert_ne!(k1, k2);
    }
}
