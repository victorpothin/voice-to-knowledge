use crate::error::AppError;
use crate::services::{build_transcricoes_txt, sync_filename, upload_to_drive};
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
            message: "Sincronização com Google Drive desativada. As transcrições ficam apenas no banco de dados. Use GET /transcricoes para analisá-las.".to_string(),
        }));
    }

    let db_path = state.db_path.clone();

    let transcricoes = tokio::task::spawn_blocking({
        let db_path = db_path.clone();
        move || {
            let conn = crate::db::open(&db_path)?;
            crate::db::list_nao_sincronizadas(&conn)
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    if transcricoes.is_empty() {
        return Ok(Json(SyncResponse {
            synced: 0,
            message: "Nenhuma transcrição pendente para sincronizar.".to_string(),
        }));
    }

    let content = build_transcricoes_txt(&transcricoes);
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

    let ids: Vec<i64> = transcricoes.iter().map(|t| t.id).collect();
    let db_path = state.db_path.clone();
    tokio::task::spawn_blocking(move || {
        let conn = crate::db::open(&db_path)?;
        crate::db::mark_sincronizado(&conn, &ids)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    Ok(Json(SyncResponse {
        synced: transcricoes.len(),
        message: format!("{} transcrições enviadas ao Drive.", transcricoes.len()),
    }))
}
