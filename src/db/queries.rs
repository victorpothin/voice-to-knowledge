use crate::error::AppError;
use crate::models::Transcription;
use rusqlite::Connection;
use std::path::Path;

/// Open database and enable WAL.
pub fn open(path: impl AsRef<Path>) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    Ok(conn)
}

/// Insert a new transcription with status 'pending'. Returns row id.
pub fn insert_pending(
    conn: &Connection,
    device_id: &str,
    audio_path: &str,
) -> Result<i64, AppError> {
    conn.execute(
        r#"
        INSERT INTO transcriptions (device_id, audio_path, raw_text, status)
        VALUES (?1, ?2, '', 'pending')
        "#,
        [device_id, audio_path],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Update transcription with raw text, processed text, and status.
pub fn update_processed(
    conn: &Connection,
    id: i64,
    raw_text: &str,
    processed_text: Option<&str>,
    status: &str,
) -> Result<(), AppError> {
    conn.execute(
        r#"
        UPDATE transcriptions
        SET raw_text = ?1, processed_text = ?2, status = ?3, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?4
        "#,
        rusqlite::params![raw_text, processed_text, status, id],
    )?;
    Ok(())
}

/// Mark transcriptions as synced by id list.
pub fn mark_synced(conn: &Connection, ids: &[i64]) -> Result<(), AppError> {
    let mut stmt = conn.prepare_cached(
        "UPDATE transcriptions SET synced = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )?;
    for id in ids {
        stmt.execute([id])?;
    }
    Ok(())
}

/// List transcriptions (non-archived only), with optional limit, offset, status filter.
pub fn list(
    conn: &Connection,
    limit: u32,
    offset: u32,
    status: Option<&str>,
) -> Result<Vec<Transcription>, AppError> {
    let limit = limit.min(500);

    // Split into two explicit branches so each Statement lives long enough
    if let Some(s) = status {
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, device_id, audio_path, raw_text, processed_text, duration_sec, status,
                   created_at, updated_at, synced, archived
            FROM transcriptions
            WHERE archived = 0 AND status = ?1
            ORDER BY created_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )?;
        let rows = stmt.query_map(rusqlite::params![s, limit, offset], row_to_transcription)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    } else {
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, device_id, audio_path, raw_text, processed_text, duration_sec, status,
                   created_at, updated_at, synced, archived
            FROM transcriptions
            WHERE archived = 0
            ORDER BY created_at DESC
            LIMIT ?1 OFFSET ?2
            "#,
        )?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], row_to_transcription)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn row_to_transcription(row: &rusqlite::Row) -> Result<Transcription, rusqlite::Error> {
    Ok(Transcription {
        id: row.get(0)?,
        device_id: row.get(1)?,
        audio_path: row.get(2)?,
        raw_text: row.get(3)?,
        processed_text: row.get(4)?,
        duration_sec: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        synced: row.get(9)?,
        archived: row.get(10)?,
    })
}

/// Get all non-synced transcriptions (for Drive sync).
pub fn list_unsynced(conn: &Connection) -> Result<Vec<Transcription>, AppError> {
    let mut stmt = conn.prepare_cached(
        r#"
        SELECT id, device_id, audio_path, raw_text, processed_text, duration_sec, status,
               created_at, updated_at, synced, archived
        FROM transcriptions
        WHERE archived = 0 AND synced = 0 AND (processed_text IS NOT NULL AND processed_text != '')
        ORDER BY created_at ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_transcription)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Get created_at for a transcription (for 202 response).
pub fn get_created_at(conn: &Connection, id: i64) -> Result<Option<String>, AppError> {
    let mut stmt =
        conn.prepare_cached("SELECT created_at FROM transcriptions WHERE id = ?1")?;
    let mut rows = stmt.query([id])?;
    Ok(rows
        .next()?
        .map(|r| r.get::<_, String>(0))
        .transpose()?)
}