use serde::{Deserialize, Serialize};

/// Row from DB (snake_case for rusqlite).
#[derive(Debug, Clone)]
pub struct Transcription {
    pub id: i64,
    pub device_id: String,
    pub audio_path: Option<String>,
    pub raw_text: String,
    pub processed_text: Option<String>,
    pub duration_sec: Option<f64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub synced: i32,
    pub archived: i32,
}

/// API response for a single transcription.
#[derive(Debug, Serialize)]
pub struct TranscriptionResponse {
    pub id: i64,
    #[serde(rename = "raw_text")]
    pub raw_text: String,
    #[serde(rename = "processed_text")]
    pub processed_text: Option<String>,
    pub created_at: String,
    pub status: String,
}

impl From<Transcription> for TranscriptionResponse {
    fn from(t: Transcription) -> Self {
        TranscriptionResponse {
            id: t.id,
            raw_text: t.raw_text,
            processed_text: t.processed_text,
            created_at: t.created_at,
            status: t.status,
        }
    }
}

/// 202 Accepted response for POST /audio.
#[derive(Debug, Serialize)]
pub struct AudioAcceptedResponse {
    pub id: i64,
    pub status: String,
    pub created_at: String,
}

/// Query params for GET /transcriptions.
#[derive(Debug, Default, Deserialize)]
pub struct TranscriptionsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
}
