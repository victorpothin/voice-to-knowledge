mod db;
mod error;
mod models;
mod routes;
mod services;

use routes::AppState;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_path: PathBuf = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "./data/voice.db".into())
        .into();
    let uploads_dir: PathBuf = std::env::var("UPLOADS_DIR")
        .unwrap_or_else(|_| "./uploads".into())
        .into();
    let whisper_model =
        std::env::var("WHISPER_MODEL").unwrap_or_else(|_| "./models/ggml-large-v3.bin".into());
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let ollama_model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".into());
    let max_upload_bytes = std::env::var("MAX_UPLOAD_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(52_428_800u64);
    let enable_google_drive_sync = std::env::var("ENABLE_GOOGLE_DRIVE_SYNC")
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            _ => Some(false),
        })
        .unwrap_or(false);
    let google_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let google_client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
    let google_refresh_token = std::env::var("GOOGLE_REFRESH_TOKEN").unwrap_or_default();
    let google_drive_folder_id = std::env::var("GOOGLE_DRIVE_FOLDER_ID").unwrap_or_default();

    let port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // Ensure DB exists and WAL is set (migrations are manual)
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = db::open(&db_path)?;

    let state = AppState {
        db_path,
        uploads_dir,
        whisper_model,
        ollama_url,
        ollama_model,
        max_upload_bytes,
        enable_google_drive_sync,
        google_client_id,
        google_client_secret,
        google_refresh_token,
        google_drive_folder_id,
    };

    let app = routes::router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    Ok(())
}
