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
│   │   ├── transcricoes.rs  # GET /transcricoes — list transcriptions
│   │   └── sync.rs          # POST /sync — manual Google Drive sync
│   ├── services/
│   │   ├── whisper.rs       # whisper-rs transcription
│   │   ├── llm.rs           # Ollama HTTP calls
│   │   └── drive.rs         # Google Drive upload
│   ├── db/
│   │   ├── schema.rs        # SQLite table definitions
│   │   └── queries.rs       # Read/write functions
│   └── models.rs            # Data structs (Transcricao, Response types)
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
- `file`: audio file (WAV or MP3)
- `device_id`: device identifier (string)

**Response:** `202 Accepted`
```json
{
  "id": 1,
  "status": "processing",
  "criado_em": "2026-03-15T10:00:00Z"
}
```

### `GET /transcricoes`
Lists all saved transcriptions.

**Query params (optional):**
- `limit`: number of records (default: 50)
- `offset`: pagination offset
- `status`: filter by status (`processado` | `pendente` | `erro`)

**Response:**
```json
[
  {
    "id": 1,
    "transcricao_bruta": "então eu tava pensando que...",
    "transcricao_tratada": "Estava pensando que...",
    "criado_em": "2026-03-15T10:00:00Z",
    "status": "processado"
  }
]
```

### `POST /sync`
Manually triggers Google Drive synchronization for unsynced transcriptions.

---

## Database Schema

```sql
CREATE TABLE transcricoes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id    TEXT NOT NULL,
    audio_path   TEXT,
    bruta        TEXT NOT NULL,
    tratada      TEXT,
    duracao_seg  REAL,
    status       TEXT DEFAULT 'processado',
    criado_em    DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    sincronizado INTEGER DEFAULT 0,
    arquivado    INTEGER DEFAULT 0
);
```

---

## Internal Services

### whisper.rs
- Saves received audio to `/uploads/`
- Uses `whisper-rs` crate for transcription via FFI
- Configurable model via `WHISPER_MODEL` env var
- Returns transcribed text or error

### llm.rs
- Calls `http://localhost:11434/api/chat` (Ollama)
- Fixed system prompt for text cleaning (configurable)
- Returns cleaned/processed text

**Example system prompt:**
```
You are a text editor. Receive an audio transcription in Portuguese and:
1. Remove filler words (um, uh, like, you know...)
2. Fix punctuation and capitalization
3. Keep the original meaning intact
4. Respond ONLY with the corrected text, no comments
```

### drive.rs
- Authenticates via OAuth2 with Google Drive API v3
- Uploads `.txt` files with unsynced transcriptions
- Updates `sincronizado = 1` in database after successful upload
- Triggered manually via `POST /sync` or automated

---

## Configuration (.env)

```env
WHISPER_MODEL=./models/ggml-large-v3.bin
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b
DATABASE_PATH=./data/voice.db
UPLOADS_DIR=./uploads
MAX_UPLOAD_BYTES=52428800
ENABLE_GOOGLE_DRIVE_SYNC=false
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REFRESH_TOKEN=...
GOOGLE_DRIVE_FOLDER_ID=...
SERVER_PORT=8080
RUST_LOG=info
```

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
- Rust 1.70+
- Ollama installed and running
- Google Cloud credentials (for Drive sync)

### Setup

```bash
# 1. Clone the repository
git clone https://github.com/your-user/voice-to-knowledge
cd voice-to-knowledge

# 2. Download Whisper model
mkdir -p models
# Download ggml-large-v3.bin from HuggingFace or whisper.cpp repo

# 3. Install and start Ollama
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3.2:3b

# 4. Configure environment
cp .env.example .env
# Edit Google credentials and other settings

# 5. Run the server
cargo run --release
```

### Docker

```bash
# Build and run with Docker Compose
docker-compose up -d
```

---

## Architecture Decisions

- **Why Rust instead of Python?** The real bottlenecks are Whisper and Ollama. The API itself needs to be lightweight to avoid competing for CPU. Rust has minimal overhead at idle.
- **Why whisper-rs/whisper.cpp instead of openai-whisper?** The C++ version is 3-5x faster on pure CPU, which is the target scenario here.
- **Why SQLite instead of Postgres?** Low volume (personal notes), zero configuration, no separate process consuming RAM.
- **Why Google Drive as a bridge?** NotebookLM has no public API. Drive is the only stable way to programmatically inject sources.

---

## Future Enhancements (Out of Scope)

- Async processing queue (if audio volume grows)
- Automatic topic categorization via LLM
- Local web UI for viewing transcriptions
- Multi-user/device support
- Real-time transcription streaming
