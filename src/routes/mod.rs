mod audio;
mod sync;
mod transcricoes;

use std::path::PathBuf;

pub use audio::post_audio;
pub use sync::{post_sync, SyncResponse};
pub use transcricoes::get_transcricoes;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub uploads_dir: PathBuf,
    pub whisper_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub max_upload_bytes: u64,
    pub enable_google_drive_sync: bool,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_refresh_token: String,
    pub google_drive_folder_id: String,
}

pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/audio", axum::routing::post(post_audio))
        .route("/transcricoes", axum::routing::get(get_transcricoes))
        .route("/sync", axum::routing::post(post_sync))
        .with_state(state)
}
