# rust_ai_experiments

A Rust AI chat application with a **Leptos SPA frontend** and an **Axum JSON + WebSocket backend**, powered by a local Ollama (llama3.2) LLM and PostgreSQL.

## Architecture

```
┌─────────────────────┐        HTTP/JSON         ┌─────────────────────────┐
│   Leptos 0.8 SPA    │ ◄──────────────────────► │   Axum 0.8 Backend      │
│   (CSR, trunk)      │       WebSocket           │                         │
│   Port 8080         │ ◄════════════════════════►│   Port 3000             │
└─────────────────────┘                           │                         │
                                                  │  ┌─────────────────┐    │
                                                  │  │  rig-core 0.31  │    │
                                                  │  │  (StreamingChat) │   │
                                                  │  └────────┬────────┘   │
                                                  │           │            │
                                                  └───────────┼────────────┘
                                                              │
                                      ┌───────────────────────┼──────────────┐
                                      │                       ▼              │
                                      │  PostgreSQL 17    Ollama (llama3.2)  │
                                      │  Port 5432        Port 11434         │
                                      └──────────────────────────────────────┘
```

### Backend (`/` — root Cargo project)

- **Axum 0.8** — JSON REST API + WebSocket endpoint
- **rig-core 0.31** — Ollama client with native `StreamingChat` for token-by-token streaming
- **SQLx** — async PostgreSQL with compile-time–checked queries
- **Tower-HTTP** — CORS and tracing middleware

#### API Endpoints

| Method | Path                                | Description                  |
|--------|-------------------------------------|------------------------------|
| POST   | `/api/chat`                         | Send a chat message (REST)   |
| GET    | `/api/conversations`                | List all conversations       |
| GET    | `/api/conversations/{id}/messages`  | Get messages for a conversation |
| GET    | `/ws/chat`                          | WebSocket streaming chat     |

#### WebSocket Protocol

1. Client opens `ws://localhost:3000/ws/chat`
2. Client sends JSON: `{"message": "Hello", "conversation_id": null}`
3. Server responds with a stream of JSON events:
   - `{"type": "stream_start", "conversation_id": "..."}`
   - `{"type": "stream_chunk", "content": "..."}` (repeated)
   - `{"type": "stream_end", "full_content": "..."}`
   - `{"type": "error", "message": "..."}` (on failure)

### Frontend (`/frontend` — separate Cargo project)

- **Leptos 0.8.16** — reactive CSR SPA compiled to WASM via Trunk
- **gloo-net** — HTTP requests to the backend API
- **web-sys** — raw WebSocket API for streaming chat
- Dark themed UI with sidebar (conversation list) and main chat area

## Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.85+)
- [Docker & Docker Compose](https://docs.docker.com/get-docker/)
- [Trunk](https://trunkrs.dev/) — `cargo install trunk`
- WASM target — `rustup target add wasm32-unknown-unknown`

## Getting Started

### 1. Start Infrastructure

Spin up PostgreSQL and Ollama:

```bash
docker compose up -d
```

| Service        | Port  | Description                  |
|----------------|-------|------------------------------|
| PostgreSQL 17  | 5432  | Chat data storage            |
| Ollama         | 11434 | Local LLM inference server   |

> **GPU support:** If you have an NVIDIA GPU, uncomment the `deploy` section in `docker-compose.yml`.

### 2. Configure Environment

```bash
cp .env.example .env
```

Default `.env`:

```dotenv
DATABASE_URL=postgres://kai:kaipassword@localhost:5432/kaiexperiments
OLLAMA_API_BASE_URL=http://localhost:11434
PORT=3000
```

### 3. Run the Backend

```bash
cargo run
```

The backend will:
1. Connect to PostgreSQL and run migrations automatically
2. Start the API server at `http://localhost:3000`

### 4. Run the Frontend

In a separate terminal:

```bash
cd frontend
trunk serve --open
```

The Leptos SPA will:
1. Compile to WASM and start a dev server at `http://localhost:8080`
2. Hot-reload on code changes
3. Connect to the backend API at `http://localhost:3000`

> On Windows, if `--open` doesn't work, use `trunk serve` and open `http://localhost:8080` manually.

## Development

### Useful Commands

```bash
# Backend: run with verbose logging
RUST_LOG=debug cargo run

# Backend: check compilation
cargo check

# Frontend: check compilation (without trunk)
cd frontend && cargo check --target wasm32-unknown-unknown

# Frontend: build for production
cd frontend && trunk build --release

# Stop Docker services
docker compose down

# Stop and wipe data
docker compose down -v
```

### Project Structure

```
rust_ai_experiments/
├── Cargo.toml              # Backend manifest
├── docker-compose.yml      # PostgreSQL + Ollama
├── migrations/             # SQL migrations
│   └── 0001_initial.sql
├── src/                    # Backend source
│   ├── main.rs             # Entry point, router, CORS
│   ├── errors.rs           # AppError enum
│   ├── models.rs           # API types, WS events
│   ├── agent/              # Ollama LLM service (rig)
│   │   └── mod.rs
│   ├── db/                 # Database repositories
│   │   ├── mod.rs
│   │   ├── conversation_repository.rs
│   │   └── message_repository.rs
│   ├── routes/             # HTTP + WS handlers
│   │   ├── mod.rs
│   │   ├── api_routes.rs
│   │   └── ws_routes.rs
│   └── service/            # Business logic
│       ├── mod.rs
│       └── chat_service.rs
└── frontend/               # Leptos SPA (separate crate)
    ├── Cargo.toml
    ├── index.html          # Trunk entry HTML
    ├── style.css           # App styles
    └── src/
        ├── main.rs         # Mount App component
        ├── api.rs          # HTTP API client
        ├── ws.rs           # WebSocket client
        ├── state.rs        # Shared reactive state
        ├── models.rs       # Shared types
        └── components/
            ├── mod.rs
            ├── sidebar.rs  # Conversation list
            └── chat.rs     # Chat area + input
```