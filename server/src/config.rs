use crate::scope::CatalogScope;
use anyhow::{Context, Result};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RateLimitBackend {
    Memory,
    Redis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheBackendKind {
    None,
    Memory,
    Redis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub port: u16,
    pub bind_addr: SocketAddr,
    pub catalog_scope: CatalogScope,
    pub redis_url: Option<String>,
    pub rate_limit_rps: Option<u32>,
    pub rate_limit_backend: RateLimitBackend,
    pub cache_backend: CacheBackendKind,
    pub cache_ttl_secs: u64,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_values(
            std::env::var("DATA_DIR").ok(),
            std::env::var("PORT").ok(),
            std::env::var("CATALOG_INCLUDE_PROVIDERS").ok(),
            std::env::var("CATALOG_EXCLUDE_PROVIDERS").ok(),
            std::env::var("CATALOG_INCLUDE_MODELS").ok(),
            std::env::var("CATALOG_EXCLUDE_MODELS").ok(),
            std::env::var("CATALOG_SCOPE_FILE").ok(),
            std::env::var("REDIS_URL").ok(),
            std::env::var("RATE_LIMIT_BACKEND").ok(),
            std::env::var("RATE_LIMIT_RPS").ok(),
            std::env::var("CACHE_BACKEND").ok(),
            std::env::var("CACHE_TTL_SECS").ok(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_values(
        data_dir: Option<String>,
        port: Option<String>,
        include_providers: Option<String>,
        exclude_providers: Option<String>,
        include_models: Option<String>,
        exclude_models: Option<String>,
        scope_file: Option<String>,
        redis_url: Option<String>,
        rate_limit_backend: Option<String>,
        rate_limit_rps: Option<String>,
        cache_backend: Option<String>,
        cache_ttl_secs: Option<String>,
    ) -> Result<Self> {
        let data_dir = data_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let port = port
            .unwrap_or_else(|| "8080".to_string())
            .parse::<u16>()
            .context("PORT must be a valid TCP port")?;
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], port));
        let catalog_scope = CatalogScope::from_values(
            include_providers,
            exclude_providers,
            include_models,
            exclude_models,
            scope_file,
        )?;

        let rate_limit_rps = rate_limit_rps
            .as_deref()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&n| n > 0);

        let rate_limit_backend = match rate_limit_backend.as_deref() {
            Some("redis") => RateLimitBackend::Redis,
            _ => RateLimitBackend::Memory,
        };

        let cache_backend = match cache_backend.as_deref() {
            Some("redis") => CacheBackendKind::Redis,
            Some("memory") => CacheBackendKind::Memory,
            _ => CacheBackendKind::None,
        };

        let cache_ttl_secs = cache_ttl_secs
            .as_deref()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(300);

        Ok(Self {
            data_dir,
            port,
            bind_addr,
            catalog_scope,
            redis_url,
            rate_limit_rps,
            rate_limit_backend,
            cache_backend,
            cache_ttl_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(overrides: impl Fn(&mut [Option<String>; 12])) -> Result<ServerConfig> {
        let mut args: [Option<String>; 12] = Default::default();
        overrides(&mut args);
        let [data_dir, port, inc_prov, exc_prov, inc_mod, exc_mod, scope_file, redis_url, rl_backend, rl_rps, cache_backend, cache_ttl] =
            args;
        ServerConfig::from_values(
            data_dir, port, inc_prov, exc_prov, inc_mod, exc_mod, scope_file,
            redis_url, rl_backend, rl_rps, cache_backend, cache_ttl,
        )
    }

    #[test]
    fn defaults_to_current_directory_and_port_8080() {
        let config = cfg(|_| {}).unwrap();
        assert_eq!(config.data_dir, PathBuf::from("."));
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_addr, SocketAddr::from(([0, 0, 0, 0], 8080)));
        assert!(config.catalog_scope.is_disabled());
    }

    #[test]
    fn reads_custom_data_dir_and_port() {
        let config = cfg(|a| {
            a[0] = Some("fixtures".into());
            a[1] = Some("9090".into());
        })
        .unwrap();
        assert_eq!(config.data_dir, PathBuf::from("fixtures"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.bind_addr, SocketAddr::from(([0, 0, 0, 0], 9090)));
    }

    #[test]
    fn rejects_invalid_port() {
        let err = cfg(|a| a[1] = Some("not-a-port".into())).unwrap_err();
        assert!(err.to_string().contains("PORT must be a valid TCP port"));
    }

    #[test]
    fn reads_catalog_scope_from_values() {
        let config = cfg(|a| {
            a[1] = Some("9090".into());
            a[2] = Some("openai".into());
        })
        .unwrap();
        assert!(config.catalog_scope.include_providers.contains("openai"));
    }

    #[test]
    fn defaults_have_no_redis_and_memory_rate_limit() {
        let config = cfg(|_| {}).unwrap();
        assert!(config.redis_url.is_none());
        assert_eq!(config.rate_limit_backend, RateLimitBackend::Memory);
        assert!(config.rate_limit_rps.is_none());
        assert_eq!(config.cache_backend, CacheBackendKind::None);
        assert_eq!(config.cache_ttl_secs, 300);
    }

    #[test]
    fn reads_redis_url_and_cache_settings() {
        let config = cfg(|a| {
            a[7] = Some("rediss://example.upstash.io".into());
            a[8] = Some("redis".into());
            a[9] = Some("10".into());
            a[10] = Some("redis".into());
            a[11] = Some("600".into());
        })
        .unwrap();
        assert_eq!(config.redis_url.as_deref(), Some("rediss://example.upstash.io"));
        assert_eq!(config.rate_limit_backend, RateLimitBackend::Redis);
        assert_eq!(config.rate_limit_rps, Some(10));
        assert_eq!(config.cache_backend, CacheBackendKind::Redis);
        assert_eq!(config.cache_ttl_secs, 600);
    }

    #[test]
    fn unknown_backend_defaults_to_memory_for_rate_limit() {
        let config = cfg(|a| a[8] = Some("typo".into())).unwrap();
        assert_eq!(config.rate_limit_backend, RateLimitBackend::Memory);
    }

    #[test]
    fn unknown_cache_backend_defaults_to_none() {
        let config = cfg(|a| a[10] = Some("typo".into())).unwrap();
        assert_eq!(config.cache_backend, CacheBackendKind::None);
    }
}
