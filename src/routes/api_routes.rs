use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::errors::AppError;
use crate::models::ChatRequest;
use crate::service::chat_service::ChatService;

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST `/api/chat` — accepts JSON, returns JSON (non-streaming fallback)
pub async fn chat_handler(
    State(svc): State<ChatService>,
    Json(request): Json<ChatRequest>,
) -> impl IntoResponse {
    match svc.chat(request).await {
        Ok(response) => Json(response).into_response(),
        Err(err) => error_response(&err),
    }
}

/// GET `/api/conversations` — list conversations as JSON
pub async fn list_conversations_handler(
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    match svc.get_conversations().await {
        Ok(convs) => Json(convs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET `/api/conversations/:id/messages` — messages for a conversation
pub async fn list_messages_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    match svc.get_messages(&id).await {
        Ok(msgs) => Json(msgs).into_response(),
        Err(e) if e.is_not_found() => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn error_response(err: &AppError) -> axum::response::Response {
    let status = if err.is_validation() {
        StatusCode::BAD_REQUEST
    } else if err.is_not_found() {
        StatusCode::NOT_FOUND
    } else if err.is_agent_unavailable() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    let body = serde_json::json!({ "error": err.to_string() });
    (status, Json(body)).into_response()
}
