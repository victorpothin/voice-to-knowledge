-- Voice to Knowledge: initial schema
-- Run with: sqlite3 data/voice.db < migrations/001_initial.sql

PRAGMA journal_mode=WAL;

CREATE TABLE IF NOT EXISTS transcriptions (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id      TEXT NOT NULL,
    audio_path     TEXT,
    raw_text       TEXT NOT NULL DEFAULT '',
    processed_text TEXT,
    duration_sec   REAL,
    status         TEXT NOT NULL DEFAULT 'pending',
    created_at     DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at     DATETIME DEFAULT CURRENT_TIMESTAMP,
    synced         INTEGER NOT NULL DEFAULT 0,
    archived       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_transcriptions_created_at ON transcriptions(created_at);
CREATE INDEX IF NOT EXISTS idx_transcriptions_synced ON transcriptions(synced);
CREATE INDEX IF NOT EXISTS idx_transcriptions_status ON transcriptions(status);
CREATE INDEX IF NOT EXISTS idx_transcriptions_archived ON transcriptions(archived);
