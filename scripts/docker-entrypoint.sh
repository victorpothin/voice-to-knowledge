#!/bin/sh
set -e

DB_PATH="${DATABASE_PATH:-/app/data/voice.db}"
if [ ! -f "$DB_PATH" ]; then
  mkdir -p "$(dirname "$DB_PATH")"
  sqlite3 "$DB_PATH" < /app/migrations/001_initial.sql
fi

# Download Whisper model if not present
if [ ! -f "${WHISPER_MODEL:-/app/models/ggml-small.bin}" ]; then
  echo "Downloading Whisper model..."
  mkdir -p "$(dirname "${WHISPER_MODEL:-/app/models/ggml-small.bin}")"
  cd "$(dirname "${WHISPER_MODEL:-/app/models/ggml-small.bin}")"
  curl -L -J -o "$(basename "${WHISPER_MODEL:-/app/models/ggml-small.bin}")" "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
  cd - > /dev/null
  echo "Model downloaded."
fi

exec /app/voice-to-knowledge
