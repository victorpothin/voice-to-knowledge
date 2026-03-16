use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Row from DB (snake_case for rusqlite).
#[derive(Debug, Clone)]
pub struct Transcricao {
    pub id: i64,
    pub device_id: String,
    pub audio_path: Option<String>,
    pub bruta: String,
    pub tratada: Option<String>,
    pub duracao_seg: Option<f64>,
    pub status: String,
    pub criado_em: String,
    pub atualizado_em: String,
    pub sincronizado: i32,
    pub arquivado: i32,
}

/// API response for a single transcription.
#[derive(Debug, Serialize)]
pub struct TranscricaoResponse {
    pub id: i64,
    #[serde(rename = "transcricao_bruta")]
    pub bruta: String,
    #[serde(rename = "transcricao_tratada")]
    pub tratada: Option<String>,
    pub criado_em: String,
    pub status: String,
}

impl From<Transcricao> for TranscricaoResponse {
    fn from(t: Transcricao) -> Self {
        TranscricaoResponse {
            id: t.id,
            bruta: t.bruta,
            tratada: t.tratada,
            criado_em: t.criado_em,
            status: t.status,
        }
    }
}

/// 202 Accepted response for POST /audio.
#[derive(Debug, Serialize)]
pub struct AudioAcceptedResponse {
    pub id: i64,
    pub status: String,
    pub criado_em: String,
}

/// Query params for GET /transcricoes.
#[derive(Debug, Default, Deserialize)]
pub struct TranscricoesQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
}
