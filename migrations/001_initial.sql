-- Voice to Knowledge: initial schema
-- Run with: sqlite3 data/voice.db < migrations/001_initial.sql

PRAGMA journal_mode=WAL;

CREATE TABLE IF NOT EXISTS transcricoes (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id      TEXT NOT NULL,
    audio_path     TEXT,
    bruta          TEXT NOT NULL DEFAULT '',
    tratada        TEXT,
    duracao_seg    REAL,
    status         TEXT NOT NULL DEFAULT 'pendente',
    criado_em      DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em  DATETIME DEFAULT CURRENT_TIMESTAMP,
    sincronizado   INTEGER NOT NULL DEFAULT 0,
    arquivado      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_transcricoes_criado_em ON transcricoes(criado_em);
CREATE INDEX IF NOT EXISTS idx_transcricoes_sincronizado ON transcricoes(sincronizado);
CREATE INDEX IF NOT EXISTS idx_transcricoes_status ON transcricoes(status);
CREATE INDEX IF NOT EXISTS idx_transcricoes_arquivado ON transcricoes(arquivado);
