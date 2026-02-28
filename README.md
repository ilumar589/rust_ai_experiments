# rust_ai_experiments

A Rust web application that provides an AI chat interface powered by Ollama (llama3.2), built with Axum, HTMX, and PostgreSQL.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Docker & Docker Compose](https://docs.docker.com/get-docker/)

## Getting Started

### 1. Start infrastructure with Docker

Spin up PostgreSQL and Ollama (+ auto-pulls the llama3.2 model):

```bash
docker compose up -d
```

This starts:

| Service        | Port  | Description                  |
|----------------|-------|------------------------------|
| PostgreSQL 17  | 5432  | Chat data storage            |
| Ollama         | 11434 | Local LLM inference server   |

> **GPU support:** If you have an NVIDIA GPU, uncomment the `deploy` section in `docker-compose.yml` to enable GPU acceleration for Ollama.

### 2. Configure environment variables

Copy the example env file and adjust values if needed:

```bash
cp .env.example .env
```

Default `.env` contents:

```dotenv
DATABASE_URL=postgres://kai:kaipassword@localhost:5432/kaiexperiments
OLLAMA_API_BASE_URL=http://localhost:11434
PORT=8080
```

The defaults work out of the box with the provided `docker-compose.yml`.

### 3. Run the application

```bash
cargo run
```

The server will:
1. Connect to PostgreSQL and run migrations automatically.
2. Start listening on [http://localhost:8080](http://localhost:8080).

### Useful commands

```bash
# Rebuild and run in release mode
cargo run --release

# Run with verbose logging
RUST_LOG=debug cargo run

# Stop Docker services
docker compose down

# Stop Docker services and delete volumes (wipes data)
docker compose down -v
```