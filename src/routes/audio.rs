use crate::error::AppError;
use crate::models::AudioAcceptedResponse;
use crate::services::transcrever;
use axum::extract::State;
use axum::response::IntoResponse;
use axum_extra::extract::Multipart;
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const ALLOWED_EXTENSIONS: &[&str] = &["wav", "mp3", "m4a", "ogg"];
const DEFAULT_MAX_UPLOAD_BYTES: u64 = 52_428_800; // 50MB

use super::AppState;

pub async fn post_audio(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<axum::response::Response, AppError> {
    let mut device_id: Option<String> = None;
    let mut file_data: Option<(Vec<u8>, String)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::Multipart(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "device_id" {
            let bytes = field.bytes().await.map_err(|e| AppError::Multipart(e.to_string()))?;
            device_id = Some(String::from_utf8_lossy(&bytes).trim().to_string());
        } else if name == "file" {
            let filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "audio".to_string());
            let ext = filename
                .rsplit('.')
                .next()
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
                return Err(AppError::Validation(format!(
                    "Extensão não permitida. Use: {}",
                    ALLOWED_EXTENSIONS.join(", ")
                )));
            }
            let bytes = field.bytes().await.map_err(|e| AppError::Multipart(e.to_string()))?;
            let max = state.max_upload_bytes;
            if bytes.len() as u64 > max {
                return Err(AppError::Validation(format!(
                    "Arquivo maior que o limite ({} bytes)",
                    max
                )));
            }
            file_data = Some((bytes.to_vec(), ext));
        }
    }

    let device_id = device_id.ok_or_else(|| AppError::MissingField("device_id".to_string()))?;
    let (data, ext) = file_data.ok_or_else(|| AppError::MissingField("file".to_string()))?;

    let id = Uuid::new_v4();
    let filename = format!("{}.{}", id, ext);
    let audio_path = state.uploads_dir.join(&filename);

    fs::create_dir_all(&state.uploads_dir)
        .await
        .map_err(AppError::Io)?;
    let mut file = fs::File::create(&audio_path).await.map_err(AppError::Io)?;
    file.write_all(&data).await.map_err(AppError::Io)?;
    file.sync_all().await.map_err(AppError::Io)?;
    drop(file);

    let path_str = audio_path.to_string_lossy().to_string();
    let db_path = state.db_path.clone();

    let row_id = tokio::task::spawn_blocking(move || {
        let conn = crate::db::open(&db_path)?;
        crate::db::insert_pendente(&conn, &device_id, &path_str)
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    let criado_em = tokio::task::spawn_blocking({
        let db_path = state.db_path.clone();
        move || {
            let conn = crate::db::open(&db_path)?;
            crate::db::get_criado_em(&conn, row_id)
        }
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??
    .unwrap_or_else(|| "".to_string());

    // Run pipeline in background
    let db_path = state.db_path.clone();
    let uploads_dir = state.uploads_dir.clone();
    let whisper_model = state.whisper_model.clone();
    let ollama_url = state.ollama_url.clone();
    let ollama_model = state.ollama_model.clone();
    tokio::spawn(async move {
        run_pipeline(
            row_id,
            audio_path,
            db_path,
            uploads_dir,
            &whisper_model,
            &ollama_url,
            &ollama_model,
        )
        .await;
    });

    let body = AudioAcceptedResponse {
        id: row_id,
        status: "pendente".to_string(),
        criado_em,
    };
    Ok((
        axum::http::StatusCode::ACCEPTED,
        axum::Json(body),
    )
        .into_response())
}

async fn run_pipeline(
    id: i64,
    audio_path: PathBuf,
    db_path: PathBuf,
    _uploads_dir: PathBuf,
    whisper_model: &str,
    ollama_url: &str,
    ollama_model: &str,
) {
    tracing::info!(id, "starting background pipeline");

    let bruta = match transcrever("", whisper_model, &audio_path).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(id, "whisper failed: {}", e);
            let _ = tokio::task::spawn_blocking({
                let db_path = db_path.clone();
                move || {
                    let conn = match crate::db::open(&db_path) {
                        Ok(c) => c,
                        Err(_) => return,
                    };
                    let _ = crate::db::update_processed(
                        &conn,
                        id,
                        "(erro na transcrição)",
                        None,
                        "erro",
                    );
                }
            })
            .await;
            return;
        }
    };

    let tratada = match crate::services::limpar_texto(
        &Client::new(),
        ollama_url,
        ollama_model,
        &bruta,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(id, "ollama failed, saving raw only: {}", e);
            let db_path2 = db_path.clone();
            let bruta_clone = bruta.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let conn = match crate::db::open(&db_path2) {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let _ = crate::db::update_processed(
                    &conn,
                    id,
                    &bruta_clone,
                    None,
                    "sem_tratamento",
                );
            })
            .await;
            return;
        }
    };

    let db_path2 = db_path.clone();
    let bruta_f = bruta.clone();
    let _ = tokio::task::spawn_blocking(move || {
        let conn = match crate::db::open(&db_path2) {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Err(e) = crate::db::update_processed(
            &conn,
            id,
            &bruta_f,
            Some(&tratada),
            "processado",
        ) {
            tracing::error!(id, "db update failed: {}", e);
        }
    })
    .await;

    tracing::info!(id, "pipeline done");
}
