use crate::error::AppError;
use crate::models::Transcricao;
use rusqlite::Connection;
use std::path::Path;

/// Open database and enable WAL.
pub fn open(path: impl AsRef<Path>) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    Ok(conn)
}

/// Insert a new transcription with status 'pendente'. Returns row id.
pub fn insert_pendente(
    conn: &Connection,
    device_id: &str,
    audio_path: &str,
) -> Result<i64, AppError> {
    conn.execute(
        r#"
        INSERT INTO transcricoes (device_id, audio_path, bruta, status)
        VALUES (?1, ?2, '', 'pendente')
        "#,
        [device_id, audio_path],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Update transcription with raw text, treated text, and status.
pub fn update_processed(
    conn: &Connection,
    id: i64,
    bruta: &str,
    tratada: Option<&str>,
    status: &str,
) -> Result<(), AppError> {
    conn.execute(
        r#"
        UPDATE transcricoes
        SET bruta = ?1, tratada = ?2, status = ?3, atualizado_em = CURRENT_TIMESTAMP
        WHERE id = ?4
        "#,
        rusqlite::params![bruta, tratada, status, id],
    )?;
    Ok(())
}

/// Mark transcriptions as synced by id list.
pub fn mark_sincronizado(conn: &Connection, ids: &[i64]) -> Result<(), AppError> {
    let mut stmt = conn.prepare_cached(
        "UPDATE transcricoes SET sincronizado = 1, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?",
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
) -> Result<Vec<Transcricao>, AppError> {
    let limit = limit.min(500);

    // Split into two explicit branches so each Statement lives long enough
    if let Some(s) = status {
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, device_id, audio_path, bruta, tratada, duracao_seg, status,
                   criado_em, atualizado_em, sincronizado, arquivado
            FROM transcricoes
            WHERE arquivado = 0 AND status = ?1
            ORDER BY criado_em DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )?;
        let rows = stmt.query_map(rusqlite::params![s, limit, offset], row_to_transcricao)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    } else {
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, device_id, audio_path, bruta, tratada, duracao_seg, status,
                   criado_em, atualizado_em, sincronizado, arquivado
            FROM transcricoes
            WHERE arquivado = 0
            ORDER BY criado_em DESC
            LIMIT ?1 OFFSET ?2
            "#,
        )?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], row_to_transcricao)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn row_to_transcricao(row: &rusqlite::Row) -> Result<Transcricao, rusqlite::Error> {
    Ok(Transcricao {
        id: row.get(0)?,
        device_id: row.get(1)?,
        audio_path: row.get(2)?,
        bruta: row.get(3)?,
        tratada: row.get(4)?,
        duracao_seg: row.get(5)?,
        status: row.get(6)?,
        criado_em: row.get(7)?,
        atualizado_em: row.get(8)?,
        sincronizado: row.get(9)?,
        arquivado: row.get(10)?,
    })
}

/// Get all non-synced transcriptions (for Drive sync).
pub fn list_nao_sincronizadas(conn: &Connection) -> Result<Vec<Transcricao>, AppError> {
    let mut stmt = conn.prepare_cached(
        r#"
        SELECT id, device_id, audio_path, bruta, tratada, duracao_seg, status,
               criado_em, atualizado_em, sincronizado, arquivado
        FROM transcricoes
        WHERE arquivado = 0 AND sincronizado = 0 AND (tratada IS NOT NULL AND tratada != '')
        ORDER BY criado_em ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_transcricao)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Get created_at for a transcription (for 202 response).
pub fn get_criado_em(conn: &Connection, id: i64) -> Result<Option<String>, AppError> {
    let mut stmt =
        conn.prepare_cached("SELECT criado_em FROM transcricoes WHERE id = ?1")?;
    let mut rows = stmt.query([id])?;
    Ok(rows
        .next()?
        .map(|r| r.get::<_, String>(0))
        .transpose()?)
}