use crate::error::AppError;
use crate::models::{TranscricaoResponse, TranscricoesQuery};
use axum::extract::{Query, State};
use axum::Json;

use super::AppState;

pub async fn get_transcricoes(
    State(state): State<AppState>,
    Query(q): Query<TranscricoesQuery>,
) -> Result<Json<Vec<TranscricaoResponse>>, AppError> {
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);
    let status = q.status.clone().as_deref().map(String::from);

    let list = tokio::task::spawn_blocking(move || {
        let conn = crate::db::open(&state.db_path)?;
        crate::db::list(&conn, limit, offset, status.as_deref())
    })
    .await
    .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

    let out: Vec<TranscricaoResponse> = list.into_iter().map(TranscricaoResponse::from).collect();
    Ok(Json(out))
}
