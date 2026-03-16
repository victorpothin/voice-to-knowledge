use crate::error::AppError;
use crate::services::{build_transcriptions_txt, sync_filename, upload_to_drive};
use axum::extract::State;
use axum::Json;
use reqwest::Client;
use serde::Serialize;

use super::AppState;

#[derive(Serialize)]
pub struct SyncResponse {
    pub synced: usize,
    pub message: String,
}

pub async fn post_sync(State(state): State<AppState>) -> Result<Json<SyncResponse>, AppError> {
    if !state.enable_google_drive_sync {
        return Ok(Json(SyncResponse {
            synced: 0,
            message: "Google Drive sync is disabled. Transcriptions are stored only in the database. Use GET /transcriptions to view them.".to_string(),
        }));
    }

    let db_path = state.db_path.clone();

    let transcriptions = tokio::task::spawn_blocking({
        let db_path = db_path.clone();
        move || {
            let conn = crate::db::open(&db_path)?;
            crate::db::list_unsynced(&conn)
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    if transcriptions.is_empty() {
        return Ok(Json(SyncResponse {
            synced: 0,
            message: "No pending transcriptions to sync.".to_string(),
        }));
    }

    let content = build_transcriptions_txt(&transcriptions);
    let filename = sync_filename();
    let client = Client::new();

    upload_to_drive(
        &client,
        &content,
        &filename,
        &state.google_drive_folder_id,
        &state.google_refresh_token,
        &state.google_client_id,
        &state.google_client_secret,
    )
    .await?;

    let ids: Vec<i64> = transcriptions.iter().map(|t| t.id).collect();
    let db_path = state.db_path.clone();
    tokio::task::spawn_blocking(move || {
        let conn = crate::db::open(&db_path)?;
        crate::db::mark_synced(&conn, &ids)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    Ok(Json(SyncResponse {
        synced: transcriptions.len(),
        message: format!("{} transcriptions uploaded to Drive.", transcriptions.len()),
    }))
}
