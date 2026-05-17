use crate::scope::CatalogScope;
use anyhow::Result;
use envconfig::Envconfig;
use std::{net::SocketAddr, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RateLimitBackend {
    Memory,
    Redis,
}

impl FromStr for RateLimitBackend {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s == "redis" {
            Self::Redis
        } else {
            Self::Memory
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheBackendKind {
    None,
    Memory,
    Redis,
}

impl FromStr for CacheBackendKind {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "redis" => Self::Redis,
            "memory" => Self::Memory,
            _ => Self::None,
        })
    }
}

#[derive(Envconfig, Clone, Debug)]
pub struct ServerConfig {
    #[envconfig(from = "DATA_DIR", default = ".")]
    pub data_dir: String,

    #[envconfig(from = "PORT", default = "8080")]
    pub port: u16,

    #[envconfig(from = "CATALOG_INCLUDE_PROVIDERS")]
    pub catalog_include_providers: Option<String>,

    #[envconfig(from = "CATALOG_EXCLUDE_PROVIDERS")]
    pub catalog_exclude_providers: Option<String>,

    #[envconfig(from = "CATALOG_INCLUDE_MODELS")]
    pub catalog_include_models: Option<String>,

    #[envconfig(from = "CATALOG_EXCLUDE_MODELS")]
    pub catalog_exclude_models: Option<String>,

    #[envconfig(from = "CATALOG_SCOPE_FILE")]
    pub catalog_scope_file: Option<String>,

    #[envconfig(from = "REDIS_URL")]
    pub redis_url: Option<String>,

    #[envconfig(from = "RATE_LIMIT_RPS")]
    pub rate_limit_rps: Option<u32>,

    #[envconfig(from = "RATE_LIMIT_BACKEND", default = "memory")]
    pub rate_limit_backend: RateLimitBackend,

    #[envconfig(from = "CACHE_BACKEND", default = "none")]
    pub cache_backend: CacheBackendKind,

    #[envconfig(from = "CACHE_TTL_SECS", default = "300")]
    pub cache_ttl_secs: u64,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self::init_from_env()?)
    }

    pub fn data_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.data_dir)
    }

    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }

    pub fn catalog_scope(&self) -> Result<CatalogScope> {
        CatalogScope::from_values(
            self.catalog_include_providers.clone(),
            self.catalog_exclude_providers.clone(),
            self.catalog_include_models.clone(),
            self.catalog_exclude_models.clone(),
            self.catalog_scope_file.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(data_dir: &str, port: u16) -> ServerConfig {
        ServerConfig {
            data_dir: data_dir.into(),
            port,
            catalog_include_providers: None,
            catalog_exclude_providers: None,
            catalog_include_models: None,
            catalog_exclude_models: None,
            catalog_scope_file: None,
            redis_url: None,
            rate_limit_rps: None,
            rate_limit_backend: RateLimitBackend::Memory,
            cache_backend: CacheBackendKind::None,
            cache_ttl_secs: 300,
        }
    }

    #[test]
    fn defaults_to_current_directory_and_port_8080() {
        let config = cfg(".", 8080);
        assert_eq!(config.data_dir_path(), PathBuf::from("."));
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_addr(), SocketAddr::from(([0, 0, 0, 0], 8080)));
        assert!(config.catalog_scope().unwrap().is_disabled());
    }

    #[test]
    fn reads_custom_data_dir_and_port() {
        let config = cfg("fixtures", 9090);
        assert_eq!(config.data_dir_path(), PathBuf::from("fixtures"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.bind_addr(), SocketAddr::from(([0, 0, 0, 0], 9090)));
    }

    #[test]
    fn reads_catalog_scope_include_providers() {
        let mut config = cfg(".", 8080);
        config.catalog_include_providers = Some("openai".into());
        assert!(
            config
                .catalog_scope()
                .unwrap()
                .include_providers
                .contains("openai")
        );
    }

    #[test]
    fn defaults_have_no_redis_and_memory_rate_limit() {
        let config = cfg(".", 8080);
        assert!(config.redis_url.is_none());
        assert_eq!(config.rate_limit_backend, RateLimitBackend::Memory);
        assert!(config.rate_limit_rps.is_none());
        assert_eq!(config.cache_backend, CacheBackendKind::None);
        assert_eq!(config.cache_ttl_secs, 300);
    }

    #[test]
    fn reads_redis_url_and_cache_settings() {
        let config = ServerConfig {
            data_dir: ".".into(),
            port: 8080,
            catalog_include_providers: None,
            catalog_exclude_providers: None,
            catalog_include_models: None,
            catalog_exclude_models: None,
            catalog_scope_file: None,
            redis_url: Some("rediss://example.upstash.io".into()),
            rate_limit_rps: Some(10),
            rate_limit_backend: RateLimitBackend::Redis,
            cache_backend: CacheBackendKind::Redis,
            cache_ttl_secs: 600,
        };
        assert_eq!(
            config.redis_url.as_deref(),
            Some("rediss://example.upstash.io")
        );
        assert_eq!(config.rate_limit_backend, RateLimitBackend::Redis);
        assert_eq!(config.rate_limit_rps, Some(10));
        assert_eq!(config.cache_backend, CacheBackendKind::Redis);
        assert_eq!(config.cache_ttl_secs, 600);
    }

    #[test]
    fn unknown_backend_string_defaults_to_memory() {
        let backend: RateLimitBackend = "typo".parse().unwrap();
        assert_eq!(backend, RateLimitBackend::Memory);
    }

    #[test]
    fn unknown_cache_backend_string_defaults_to_none() {
        let backend: CacheBackendKind = "typo".parse().unwrap();
        assert_eq!(backend, CacheBackendKind::None);
    }
}
