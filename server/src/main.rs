use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

fn main_data_dir() -> PathBuf {
    std::env::var("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let data_dir = main_data_dir();
    let app_state = almanac_server::state::load_state(&data_dir)?;
    let shared = Arc::new(RwLock::new(app_state));

    let app = almanac_server::routes::router(Arc::clone(&shared));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
