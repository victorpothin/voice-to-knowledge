use crate::error::AppError;
use crate::models::Transcription;
use reqwest::Client;
use std::fmt::Write;

/// Build .txt content from transcriptions: one processed text per block, separated by "---".
pub fn build_transcriptions_txt(transcriptions: &[Transcription]) -> String {
    let mut out = String::new();
    for (i, t) in transcriptions.iter().enumerate() {
        if i > 0 {
            let _ = writeln!(out, "---");
        }
        if let Some(ref processed) = t.processed_text {
            let _ = write!(out, "{}", processed);
        }
    }
    out
}

/// Filename for sync file: transcriptions_YYYY-MM-DD.txt
pub fn sync_filename() -> String {
    let now = chrono::Utc::now();
    format!("transcriptions_{}.txt", now.format("%Y-%m-%d"))
}

/// Upload content to Google Drive using OAuth2.
/// Requires GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, GOOGLE_REFRESH_TOKEN, GOOGLE_DRIVE_FOLDER_ID.
/// Returns Ok(()) on success; caller should then mark transcriptions as synced.
pub async fn upload_to_drive(
    _client: &Client,
    _content: &str,
    _filename: &str,
    _folder_id: &str,
    _refresh_token: &str,
    _client_id: &str,
    _client_secret: &str,
) -> Result<(), AppError> {
    // TODO: implement OAuth2 refresh + Drive API v3 upload
    // For now we stub so the app compiles and sync endpoint returns 501 or runs without actually uploading
    Err(AppError::Drive(
        "Google Drive upload not yet implemented (OAuth2 + Drive API v3)".to_string(),
    ))
}
