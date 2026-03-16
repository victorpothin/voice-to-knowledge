#!/bin/sh
set -e

DB_PATH="${DATABASE_PATH:-/app/data/voice.db}"
if [ ! -f "$DB_PATH" ]; then
  mkdir -p "$(dirname "$DB_PATH")"
  sqlite3 "$DB_PATH" < /app/migrations/001_initial.sql
fi

exec /app/voice-to-knowledge
