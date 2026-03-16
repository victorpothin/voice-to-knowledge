# Voice to Knowledge — Backend

## Visão Geral

Pipeline local para capturar áudios enviados por um app mobile, transcrever com IA, tratar o texto e sincronizar com o Google NotebookLM via Google Drive. Tudo roda em servidor doméstico, sem serviços pagos de transcrição, com privacidade total.

---

## Fluxo de Dados

```
[App Android] 
    → POST /audio (multipart, WAV/MP3)
    → [Rust API Server]
        → whisper.cpp (transcrição)
        → Ollama LLM (limpeza/tratamento do texto)
        → SQLite (persistência)
        → Google Drive API (sync periódico)
            → Google NotebookLM (base de conhecimento)
```

---

## Stack Técnica

| Camada | Tecnologia | Justificativa |
|---|---|---|
| Servidor HTTP | Rust + Axum | Performance equivalente a C++, async nativo, segurança de memória |
| Transcrição | whisper.cpp | Versão C++ do Whisper da OpenAI, muito mais rápida que Python em CPU |
| Modelo Whisper | `small` (465MB) | Boa qualidade em PT-BR, leve o suficiente para CPU |
| LLM local | Ollama + llama3.2:3b ou qwen2.5:3b | Limpeza de texto, remoção de vícios de linguagem, formatação |
| Banco de dados | SQLite via `rusqlite` | Zero overhead, sem servidor separado |
| Sync cloud | Google Drive API v3 | Ponte para o NotebookLM (que lê fontes do Drive) |
| Rede local | Tailscale (opcional) | Expor servidor fora do WiFi local com segurança |

---

## Estrutura de Pastas

```
voice-to-knowledge/
├── src/
│   ├── main.rs              # Entry point, inicializa servidor Axum
│   ├── routes/
│   │   ├── audio.rs         # POST /audio — recebe e processa áudio
│   │   └── transcricoes.rs  # GET /transcricoes — lista registros
│   ├── services/
│   │   ├── whisper.rs       # Chama whisper.cpp via subprocess ou FFI
│   │   ├── llm.rs           # Chama Ollama via HTTP local
│   │   └── drive.rs         # Upload para Google Drive
│   ├── db/
│   │   ├── schema.rs        # Definição das tabelas SQLite
│   │   └── queries.rs       # Funções de leitura/escrita
│   └── models.rs            # Structs de dados (Transcricao, AudioPayload, etc.)
├── whisper.cpp/             # Submódulo git do whisper.cpp
├── models/                  # Modelos .bin do Whisper (baixados separadamente)
├── uploads/                 # Áudios recebidos (temporários)
├── .env                     # Credenciais Google, configs
├── Cargo.toml
└── SKILLS.md                # Skills para agentes de IA
```

---

## Endpoints da API

### `POST /audio`
Recebe arquivo de áudio, executa pipeline completo.

**Request:** `multipart/form-data`
- `file`: arquivo de áudio (WAV ou MP3)
- `device_id`: identificador do dispositivo (string)

**Response:**
```json
{
  "id": 1,
  "transcricao_bruta": "então eu tava pensando que...",
  "transcricao_tratada": "Estava pensando que...",
  "criado_em": "2026-03-15T10:00:00Z",
  "status": "processado"
}
```

### `GET /transcricoes`
Lista todas as transcrições salvas.

**Query params opcionais:**
- `limit`: número de registros (default: 50)
- `offset`: paginação
- `status`: `processado` | `pendente` | `erro`

---

## Schema do Banco de Dados

```sql
CREATE TABLE transcricoes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id   TEXT NOT NULL,
    audio_path  TEXT,
    bruta       TEXT NOT NULL,
    tratada     TEXT,
    duracao_seg REAL,
    status      TEXT DEFAULT 'processado',
    criado_em   DATETIME DEFAULT CURRENT_TIMESTAMP,
    sincronizado INTEGER DEFAULT 0
);
```

---

## Serviços Internos

### whisper.rs
- Salva o áudio recebido em `/uploads/`
- Chama o binário `whisper.cpp/main` via `std::process::Command`
- Passa flags: `-m models/ggml-small.bin -l pt -f <arquivo>`
- Retorna o texto transcrito ou erro

### llm.rs
- Chama `http://localhost:11434/api/chat` (Ollama)
- System prompt fixo de limpeza de texto (configurável via .env ou arquivo)
- Retorna texto tratado

**Exemplo de system prompt:**
```
Você é um editor de texto. Receba uma transcrição de áudio em português e:
1. Remova vícios de linguagem (então, né, tipo, é...)
2. Corrija pontuação e capitalização
3. Mantenha o significado original intacto
4. Responda APENAS com o texto corrigido, sem comentários
```

### drive.rs
- Autentica via OAuth2 com Google Drive API v3
- Faz upload de arquivo `.txt` com as transcrições não sincronizadas
- Atualiza campo `sincronizado = 1` no banco após upload bem-sucedido
- Roda via cron job ou endpoint manual `POST /sync`

---

## Configuração (.env)

```env
WHISPER_BIN=./whisper.cpp/main
WHISPER_MODEL=./models/ggml-small.bin
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b
DATABASE_PATH=./data/voice.db
UPLOADS_DIR=./uploads
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REFRESH_TOKEN=...
GOOGLE_DRIVE_FOLDER_ID=...
SERVER_PORT=8080
```

---

## Dependências Rust (Cargo.toml)

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dotenv = "0.15"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4"] }
```

---

## Setup Inicial

```bash
# 1. Clonar o projeto e o whisper.cpp
git clone https://github.com/seu-user/voice-to-knowledge
cd voice-to-knowledge
git submodule add https://github.com/ggerganov/whisper.cpp

# 2. Compilar whisper.cpp
cd whisper.cpp && make && cd ..

# 3. Baixar modelo Whisper
bash whisper.cpp/models/download-ggml-model.sh small

# 4. Instalar e rodar Ollama
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3.2:3b

# 5. Configurar .env
cp .env.example .env
# editar credenciais Google

# 6. Rodar o servidor
cargo run --release
```

---

## Decisões de Arquitetura

- **Por que Rust e não Python?** O gargalo real é o Whisper e o Ollama. A API em si precisa ser leve para não disputar CPU com eles. Rust tem overhead mínimo em idle.
- **Por que whisper.cpp e não openai-whisper?** A versão C++ é 3-5x mais rápida em CPU puro, que é o cenário aqui.
- **Por que SQLite e não Postgres?** Volume baixo (notas pessoais), zero configuração, zero processo separado consumindo RAM.
- **Por que Google Drive como ponte?** NotebookLM não tem API pública. O Drive é a única forma estável de injetar fontes de forma programática.

---

## Próximos Passos (fora do escopo inicial)

- Fila de processamento assíncrono (se volume de áudios crescer)
- Categorização automática por tópico via LLM
- Interface web local para visualizar transcrições
- Suporte a múltiplos usuários/dispositivos
