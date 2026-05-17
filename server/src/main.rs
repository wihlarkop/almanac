use almanac_server::config::ServerConfig;
use almanac_server::request::{MemoryRateLimiter, RateLimiter, enforce_rate_limit};
use anyhow::{Context, Result};
use axum::middleware;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    #[cfg(feature = "metrics")]
    almanac_server::metrics::init();

    let config = ServerConfig::from_env()?;
    tracing::info!(
        bind_addr = %config.bind_addr,
        data_dir = %config.data_dir.display(),
        "starting almanac server"
    );

    let app_state =
        almanac_server::state::load_state_with_scope(&config.data_dir, &config.catalog_scope)
            .with_context(|| format!("loading catalog from {}", config.data_dir.display()))?;
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

    #[cfg(unix)]
    {
        let shared_reload = Arc::clone(&shared);
        let data_dir_reload = config.data_dir.clone();
        let catalog_scope_reload = config.catalog_scope.clone();
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

    let app = almanac_server::routes::router(Arc::clone(&shared));
    let app = match rate_limit_rps() {
        Some(rps) => {
            tracing::info!(rps, "rate limiting enabled");
            let limiter = Arc::new(RateLimiter::Memory(MemoryRateLimiter::new(rps)));
            app.layer(middleware::from_fn(move |req, next| {
                let lim = Arc::clone(&limiter);
                async move { enforce_rate_limit(lim, req, next).await }
            }))
        }
        None => {
            tracing::info!("rate limiting disabled");
            app
        }
    };

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("binding listener on {}", config.bind_addr))?;
    tracing::info!(bind_addr = %config.bind_addr, "server listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    tracing::info!("server stopped");

    Ok(())
}

fn rate_limit_rps() -> Option<u32> {
    std::env::var("RATE_LIMIT_RPS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&n| n > 0)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("almanac_server=info,tower_http=warn,server=info"));

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
