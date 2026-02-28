mod agent;
mod db;
mod errors;
mod models;
mod routes;
mod service;

use axum::{Router, routing::get, routing::post};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::agent::OllamaAgentService;
use crate::db::conversation_repository::ConversationRepository;
use crate::db::message_repository::MessageRepository;
use crate::routes::api_routes::{chat_handler, list_conversations_handler, list_messages_handler};
use crate::routes::chat_routes::{index_handler, load_chat_handler, new_chat_handler};
use crate::service::chat_service::ChatService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (development convenience)
    dotenvy::dotenv().ok();

    // Initialise tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_ai_experiments=debug,tower_http=debug".into()),
        )
        .init();

    // ── Database ──────────────────────────────────────────────────────────────
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set (copy .env.example to .env)");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    info!("Database connection established and migrations applied");

    // ── Dependency wiring (matching Kotlin Routing.kt) ────────────────────────
    let ollama_base_url = std::env::var("OLLAMA_API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    let conversation_repo = ConversationRepository::new(pool.clone());
    let message_repo = MessageRepository::new(pool.clone());
    let agent = OllamaAgentService::new(&ollama_base_url);
    let chat_service = ChatService::new(conversation_repo, message_repo, agent);

    // ── Router ────────────────────────────────────────────────────────────────
    let app = Router::new()
        // Page routes
        .route("/", get(index_handler))
        .route("/chat/new", get(new_chat_handler))
        .route("/chat/{id}", get(load_chat_handler))
        // API / HTMX routes
        .route("/api/chat", post(chat_handler))
        .route("/api/conversations", get(list_conversations_handler))
        .route("/api/conversations/{id}/messages", get(list_messages_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(chat_service);

    // ── Listen ────────────────────────────────────────────────────────────────
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Listening on http://{addr}/");

    axum::serve(listener, app).await?;
    Ok(())
}
