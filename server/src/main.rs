use almanac_server::config::ServerConfig;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = ServerConfig::from_env()?;
    tracing::info!(
        bind_addr = %config.bind_addr,
        data_dir = %config.data_dir.display(),
        "starting almanac server"
    );

    let app_state = almanac_server::state::load_state(&config.data_dir)
        .with_context(|| format!("loading catalog from {}", config.data_dir.display()))?;
    tracing::info!(
        providers = app_state.providers.len(),
        models = app_state.models.len(),
        aliases = app_state.aliases.len(),
        etag = %app_state.etag,
        "catalog loaded"
    );
    let shared = Arc::new(RwLock::new(app_state));

    #[cfg(unix)]
    {
        let shared_reload = Arc::clone(&shared);
        let data_dir_reload = config.data_dir.clone();
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
                match almanac_server::state::load_state(&data_dir_reload) {
                    Ok(new_state) => {
                        let providers = new_state.providers.len();
                        let models = new_state.models.len();
                        let aliases = new_state.aliases.len();
                        let etag = new_state.etag.clone();
                        *shared_reload.write().await = new_state;
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

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("binding listener on {}", config.bind_addr))?;
    tracing::info!(bind_addr = %config.bind_addr, "server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("server stopped");

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("almanac_server=info,tower_http=info,server=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
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
