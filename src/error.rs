use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Whisper error: {0}")]
    Whisper(String),

    #[error("Ollama error: {0}")]
    Ollama(String),

    #[error("Drive error: {0}")]
    Drive(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Multipart error: {0}")]
    Multipart(String),

    #[error("Missing field: {0}")]
    MissingField(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Validation(_) | AppError::MissingField(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Io(_) | AppError::Db(_) | AppError::Whisper(_) | AppError::Ollama(_) | AppError::Drive(_) | AppError::Request(_) | AppError::Multipart(_) => {
                tracing::error!("{:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}
