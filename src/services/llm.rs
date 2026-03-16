use crate::error::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SYSTEM_PROMPT: &str = r#"You are a text editor. Receive an audio transcription in Portuguese and:
1. Remove filler words (um, uh, like, you know, so, well...)
2. Fix punctuation and capitalization
3. Keep the original meaning intact
4. Respond ONLY with the corrected text, no comments"#;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Option<MessageContent>,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

/// Call Ollama to clean/format the raw transcription. On failure returns Err so caller can save raw and set status 'unprocessed'.
pub async fn limpar_texto(
    client: &Client,
    base_url: &str,
    model: &str,
    raw_text: &str,
) -> Result<String, AppError> {
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: raw_text.to_string(),
            },
        ],
        stream: false,
    };

    let res = client
        .post(&url)
        .json(&req)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| AppError::Ollama(e.to_string()))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AppError::Ollama(format!("{}: {}", status, body)));
    }

    let body: ChatResponse = res
        .json()
        .await
        .map_err(|e| AppError::Ollama(e.to_string()))?;

    let content = body
        .message
        .map(|m| m.content.trim().to_string())
        .unwrap_or_default();

    Ok(if content.is_empty() {
        raw_text.to_string()
    } else {
        content
    })
}
