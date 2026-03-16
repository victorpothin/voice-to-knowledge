# Build stage
FROM rust:1-bookworm AS build
WORKDIR /build

# Install dependencies for whisper-rs (requires cmake for whisper.cpp bindings and libclang for bindgen)
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    libclang-dev \
    llvm-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    sqlite3 \
    curl \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /build/target/release/voice-to-knowledge /app/voice-to-knowledge
COPY migrations /app/migrations
COPY scripts/docker-entrypoint.sh /app/docker-entrypoint.sh

RUN mkdir -p /app/data /app/uploads /app/models && chmod +x /app/docker-entrypoint.sh

ENV WHISPER_MODEL=/app/models/ggml-large-v3.bin \
    DATABASE_PATH=/app/data/voice.db \
    UPLOADS_DIR=/app/uploads \
    SERVER_PORT=8080

EXPOSE 8080

ENTRYPOINT ["/app/docker-entrypoint.sh"]
