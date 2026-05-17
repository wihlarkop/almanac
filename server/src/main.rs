use almanac_server::{
    cache::Cache,
    config::{CacheBackendKind, RateLimitBackend, ServerConfig},
    request::{MemoryRateLimiter, RateLimiter, RedisRateLimiter, enforce_rate_limit},
};
use anyhow::{Context, Result};
use axum::middleware;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();
    dotenvy::dotenv().ok();
    init_tracing();

    #[cfg(feature = "metrics")]
    almanac_server::metrics::init();

    let config = ServerConfig::from_env()?;
    let bind_addr = config.bind_addr();
    let catalog_scope = config.catalog_scope()?;
    let data_dir = config.data_dir_path();

    tracing::info!(
        bind_addr = %bind_addr,
        data_dir = %data_dir.display(),
        "starting almanac server"
    );

    let app_state = almanac_server::state::load_state_with_scope(&data_dir, &catalog_scope)
        .with_context(|| format!("loading catalog from {}", data_dir.display()))?;
    tracing::info!(
        providers = app_state.providers.len(),
        models = app_state.models.len(),
        aliases = app_state.aliases.len(),
        etag = %app_state.etag,
        "catalog loaded"
    );
    #[cfg(feature = "metrics")]
    almanac_server::metrics::set_catalog_counts(
        app_state.models.len(),
        app_state.providers.len(),
        app_state.aliases.len(),
    );
    let shared = Arc::new(RwLock::new(app_state));

    // Build optional Redis client shared by both rate limiter and cache.
    let redis_client = match &config.redis_url {
        Some(url) => match redis::Client::open(url.as_str()) {
            Ok(client) => {
                tracing::info!("redis client created");
                Some(client)
            }
            Err(err) => {
                tracing::warn!(%err, "failed to create redis client; falling back to in-memory backends");
                None
            }
        },
        None => {
            tracing::info!("REDIS_URL not set; using in-memory rate limiting and no caching");
            None
        }
    };

    let cache = Arc::new(build_cache(&config, redis_client.as_ref()).await);
    let rate_limiter = build_rate_limiter(&config, redis_client.as_ref()).await;

    #[cfg(unix)]
    {
        let shared_reload = Arc::clone(&shared);
        let data_dir_reload = data_dir.clone();
        let catalog_scope_reload = catalog_scope.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{SignalKind, signal};
            let mut stream = match signal(SignalKind::hangup()) {
                Ok(stream) => stream,
                Err(error) => {
                    tracing::error!(%error, "failed to install SIGHUP reload handler");
                    return;
                }
            };
            loop {
                stream.recv().await;
                tracing::info!(
                    data_dir = %data_dir_reload.display(),
                    "SIGHUP received; reloading catalog"
                );
                match almanac_server::state::load_state_with_scope(
                    &data_dir_reload,
                    &catalog_scope_reload,
                ) {
                    Ok(new_state) => {
                        let providers = new_state.providers.len();
                        let models = new_state.models.len();
                        let aliases = new_state.aliases.len();
                        let etag = new_state.etag.clone();
                        *shared_reload.write().await = new_state;
                        #[cfg(feature = "metrics")]
                        almanac_server::metrics::set_catalog_counts(models, providers, aliases);
                        tracing::info!(
                            providers,
                            models,
                            aliases,
                            etag = %etag,
                            "catalog reloaded"
                        );
                    }
                    Err(error) => tracing::error!(%error, "catalog reload failed"),
                }
            }
        });
    }

    let app = almanac_server::routes::router(Arc::clone(&shared), cache);
    let app = match rate_limiter {
        Some(limiter) => {
            let limiter = Arc::new(limiter);
            app.layer(middleware::from_fn(move |req, next| {
                let lim = Arc::clone(&limiter);
                async move { enforce_rate_limit(lim, req, next).await }
            }))
        }
        None => {
            tracing::info!("rate limiting disabled (RATE_LIMIT_RPS not set)");
            app
        }
    };

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("binding listener on {}", bind_addr))?;
    tracing::info!(bind_addr = %bind_addr, "server listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    tracing::info!("server stopped");

    Ok(())
}

async fn build_cache(config: &ServerConfig, redis_client: Option<&redis::Client>) -> Cache {
    let ttl = Duration::from_secs(config.cache_ttl_secs);
    match config.cache_backend {
        CacheBackendKind::Redis => match redis_client {
            Some(client) => match client.get_connection_manager().await {
                Ok(conn) => {
                    tracing::info!(ttl_secs = config.cache_ttl_secs, "redis cache enabled");
                    Cache::redis(conn, ttl)
                }
                Err(err) => {
                    tracing::warn!(%err, "redis cache connection failed; caching disabled");
                    Cache::noop()
                }
            },
            None => {
                tracing::warn!("CACHE_BACKEND=redis but REDIS_URL is not set; caching disabled");
                Cache::noop()
            }
        },
        CacheBackendKind::Memory => {
            tracing::info!(ttl_secs = config.cache_ttl_secs, "in-memory cache enabled");
            Cache::memory(ttl)
        }
        CacheBackendKind::None => {
            tracing::info!("caching disabled");
            Cache::noop()
        }
    }
}

async fn build_rate_limiter(
    config: &ServerConfig,
    redis_client: Option<&redis::Client>,
) -> Option<RateLimiter> {
    let rps = config.rate_limit_rps?;
    match config.rate_limit_backend {
        RateLimitBackend::Redis => match redis_client {
            Some(client) => match RedisRateLimiter::new(client, rps, 1).await {
                Ok(limiter) => {
                    tracing::info!(rps, "redis rate limiting enabled");
                    Some(RateLimiter::Redis(limiter))
                }
                Err(err) => {
                    tracing::warn!(%err, rps, "redis rate limiter failed; falling back to memory");
                    Some(RateLimiter::Memory(MemoryRateLimiter::new(rps)))
                }
            },
            None => {
                tracing::warn!(
                    rps,
                    "RATE_LIMIT_BACKEND=redis but REDIS_URL not set; falling back to memory"
                );
                Some(RateLimiter::Memory(MemoryRateLimiter::new(rps)))
            }
        },
        RateLimitBackend::Memory => {
            tracing::info!(rps, "in-memory rate limiting enabled");
            Some(RateLimiter::Memory(MemoryRateLimiter::new(rps)))
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("almanac_server=info,tower_http=warn"));

    let json = std::env::var("LOG_FORMAT").as_deref() == Ok("json");

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl+C shutdown handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to install SIGTERM shutdown handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Ctrl+C received; shutting down"),
        _ = terminate => tracing::info!("SIGTERM received; shutting down"),
    }
}
