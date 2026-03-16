# Voice to Knowledge — Backend

A self-hosted pipeline for capturing audio from a mobile app, transcribing with AI, processing the text, and syncing with Google NotebookLM via Google Drive. Everything runs on a local server without paid transcription services, ensuring complete privacy.

---

## Data Flow

```
[Mobile App]
    → POST /audio (multipart, WAV/MP3)
    → [Rust API Server]
        → whisper-rs (transcription)
        → Ollama LLM (text cleaning/processing)
        → SQLite (persistence)
        → Google Drive API (periodic sync)
            → Google NotebookLM (knowledge base)
```

---

## Tech Stack

| Layer | Technology | Rationale |
|---|---|---|
| HTTP Server | Rust + Axum | C++-level performance, native async, memory safety |
| Transcription | whisper-rs | Rust bindings for whisper.cpp, fast CPU inference |
| Whisper Model | `large-v3` | High quality transcription, supports multiple languages |
| Local LLM | Ollama + llama3.2:3b or qwen2.5:3b | Text cleaning, filler word removal, formatting |
| Database | SQLite via `rusqlite` | Zero overhead, no separate server |
| Cloud Sync | Google Drive API v3 | Bridge to NotebookLM (reads sources from Drive) |
| Local Network | Tailscale (optional) | Secure exposure outside local WiFi |

---

## Project Structure

```
voice-to-knowledge/
├── src/
│   ├── main.rs              # Entry point, Axum server initialization
│   ├── routes/
│   │   ├── audio.rs         # POST /audio — receive and process audio
│   │   ├── transcriptions.rs # GET /transcriptions — list transcriptions
│   │   └── sync.rs          # POST /sync — manual Google Drive sync
│   ├── services/
│   │   ├── whisper.rs       # whisper-rs transcription
│   │   ├── llm.rs           # Ollama HTTP calls
│   │   └── drive.rs         # Google Drive upload
│   ├── db/
│   │   ├── schema.rs        # SQLite table definitions
│   │   └── queries.rs       # Read/write functions
│   └── models.rs            # Data structs (Transcription, Response types)
├── migrations/
│   └── 001_initial.sql      # Database schema migration
├── models/                  # Whisper .bin models (downloaded separately)
├── uploads/                 # Received audio files (temporary)
├── data/                    # SQLite database storage
├── .env                     # Google credentials, configuration
├── docker-compose.yml       # Docker Compose setup
├── Dockerfile               # Container image definition
├── Cargo.toml
└── SKILLS.md                # AI agent skills documentation
```

---

## API Endpoints

### `POST /audio`
Receives an audio file and executes the full pipeline asynchronously.

**Request:** `multipart/form-data`
- `file`: audio file (WAV, MP3, M4A, or OGG)
- `device_id`: device identifier (string)

**Response:** `202 Accepted`
```json
{
  "id": 1,
  "status": "pending",
  "created_at": "2026-03-16T22:00:00Z"
}
```

### `GET /transcriptions`
Lists all saved transcriptions.

**Query params (optional):**
- `limit`: number of records (default: 50, max: 500)
- `offset`: pagination offset
- `status`: filter by status (`processed` | `pending` | `error` | `unprocessed`)

**Response:**
```json
[
  {
    "id": 1,
    "raw_text": "um então eu tava pensando que...",
    "processed_text": "Estava pensando que...",
    "created_at": "2026-03-16T22:00:00Z",
    "status": "processed"
  }
]
```

### `POST /sync`
Manually triggers Google Drive synchronization for unsynced transcriptions.

**Response:**
```json
{
  "synced": 5,
  "message": "5 transcriptions uploaded to Drive."
}
```

---

## Database Schema

```sql
CREATE TABLE transcriptions (
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
```

**Indexes:**
- `idx_transcriptions_created_at` — for sorting by date
- `idx_transcriptions_synced` — for sync queries
- `idx_transcriptions_status` — for status filtering
- `idx_transcriptions_archived` — for archive filtering

---

## Internal Services

### whisper.rs
- Saves received audio to `/uploads/`
- Uses `whisper-rs` crate for transcription via FFI
- Configurable model via `WHISPER_MODEL` env var
- Returns transcribed text or error

### llm.rs
- Calls `http://ollama:11434/api/chat` (Ollama)
- Fixed system prompt for text cleaning (configurable)
- Returns cleaned/processed text

**Example system prompt:**
```
You are a text editor. Receive an audio transcription in Portuguese and:
1. Remove filler words (um, uh, like, you know, so, well...)
2. Fix punctuation and capitalization
3. Keep the original meaning intact
4. Respond ONLY with the corrected text, no comments
```

### drive.rs
- Authenticates via OAuth2 with Google Drive API v3
- Uploads `.txt` files with unsynced transcriptions
- Updates `synced = 1` in database after successful upload
- Triggered manually via `POST /sync`

---

## Configuration (.env)

```env
WHISPER_MODEL=/app/models/ggml-large-v3.bin
OLLAMA_URL=http://ollama:11434
OLLAMA_MODEL=llama3.2:3b
DATABASE_PATH=/app/data/voice.db
UPLOADS_DIR=/app/uploads
MAX_UPLOAD_BYTES=52428800
ENABLE_GOOGLE_DRIVE_SYNC=false
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REFRESH_TOKEN=...
GOOGLE_DRIVE_FOLDER_ID=...
SERVER_PORT=8081
RUST_LOG=info
```

| Variable | Default | Description |
|---|---|---|
| `WHISPER_MODEL` | `/app/models/ggml-large-v3.bin` | Path to Whisper model file |
| `OLLAMA_URL` | `http://ollama:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `llama3.2:3b` | Model to use for text processing |
| `DATABASE_PATH` | `/app/data/voice.db` | SQLite database path |
| `UPLOADS_DIR` | `/app/uploads` | Directory for uploaded audio files |
| `MAX_UPLOAD_BYTES` | `52428800` | Max upload size (50MB) |
| `ENABLE_GOOGLE_DRIVE_SYNC` | `false` | Enable/disable Drive sync |
| `GOOGLE_*` | — | Google OAuth2 credentials |
| `SERVER_PORT` | `8081` | HTTP server port |
| `RUST_LOG` | `info` | Log level |

---

## Dependencies (Cargo.toml)

```toml
[dependencies]
axum = "0.7"
axum-extra = { version = "0.9", features = ["multipart", "typed-header"] }
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dotenv = "0.15"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
chrono = { version = "0.4", features = ["serde"] }
whisper-rs = "0.13"
hound = "3.5"
```

---

## Getting Started

### Prerequisites
- Rust 1.70+ (for local development)
- Docker & Docker Compose (recommended)
- Ollama (for local development without Docker)
- Google Cloud credentials (optional, for Drive sync)

### Quick Start with Docker

```bash
# 1. Clone the repository
git clone https://github.com/your-user/voice-to-knowledge
cd voice-to-knowledge

# 2. Create .env file
cp .env.example .env
# Edit with your settings (Google credentials optional)

# 3. Start all services
docker compose up -d --build

# 4. Check logs
docker compose logs -f api

# 5. Test the API
curl http://localhost:8081/transcriptions
```

### Local Development

```bash
# 1. Install dependencies
# - Rust 1.70+
# - Ollama: curl -fsSL https://ollama.com/install.sh | sh
# - Pull model: ollama pull llama3.2:3b

# 2. Download Whisper model
mkdir -p models
# Download ggml-large-v3.bin from HuggingFace

# 3. Configure environment
cp .env.example .env
# Edit settings as needed

# 4. Run migrations
sqlite3 data/voice.db < migrations/001_initial.sql

# 5. Run the server
cargo run --release
```

---

## Architecture Decisions

- **Why Rust instead of Python?** The real bottlenecks are Whisper and Ollama. The API itself needs to be lightweight to avoid competing for CPU. Rust has minimal overhead at idle.
- **Why whisper-rs/whisper.cpp instead of openai-whisper?** The C++ version is 3-5x faster on pure CPU, which is the target scenario here.
- **Why SQLite instead of Postgres?** Low volume (personal notes), zero configuration, no separate process consuming RAM.
- **Why Google Drive as a bridge?** NotebookLM has no public API. Drive is the only stable way to programmatically inject sources.

---

## Status Values

| Status | Description |
|---|---|
| `pending` | Audio received, waiting for processing |
| `processed` | Transcription and cleaning completed |
| `error` | Transcription failed |
| `unprocessed` | Transcription succeeded, but LLM cleaning failed (raw text saved) |

---

## Future Enhancements (Out of Scope)

- Async processing queue (if audio volume grows)
- Automatic topic categorization via LLM
- Local web UI for viewing transcriptions
- Multi-user/device support
- Real-time transcription streaming
- Google Drive OAuth2 implementation (currently stubbed)
