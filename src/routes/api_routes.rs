use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;

use crate::errors::AppError;
use crate::models::{ChatRequest, Conversation};
use crate::service::chat_service::ChatService;

// ── Form input ────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct ChatForm {
    #[serde(default)]
    pub conversation_id: String,
    pub message: String,
}

// ── Template structs ──────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "chat_response.html")]
struct ChatResponseTemplate {
    user_message: String,
    assistant_id: String,
    assistant_content: String,
    conversations: Vec<Conversation>,
    active_conversation_id: String,
}

#[derive(Template)]
#[template(path = "error_fragment.html")]
struct ErrorFragmentTemplate {
    error_message: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST `/api/chat` — accepts form data, returns HTML fragment(s) for HTMX
pub async fn chat_handler(
    State(svc): State<ChatService>,
    Form(form): Form<ChatForm>,
) -> Response {
    let conversation_id = if form.conversation_id.is_empty() {
        None
    } else {
        Some(form.conversation_id.clone())
    };

    let request = ChatRequest {
        conversation_id,
        message: form.message.clone(),
    };

    match svc.chat(request).await {
        Err(err) => error_response(&err),
        Ok(response) => {
            let conversations = svc.get_conversations().await.unwrap_or_default();

            let tmpl = ChatResponseTemplate {
                user_message: form.message.clone(),
                assistant_id: response.message.id.clone(),
                assistant_content: response.message.content.clone(),
                conversations,
                active_conversation_id: response.conversation_id.clone(),
            };

            match tmpl.render() {
                Ok(html) => {
                    // Tell Alpine.js / the hidden input about the conversation id
                    let mut resp = Html(html).into_response();
                    resp.headers_mut().insert(
                        "X-Conversation-Id",
                        response.conversation_id.parse().unwrap(),
                    );
                    resp
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Template error: {e}"),
                )
                    .into_response(),
            }
        }
    }
}

/// GET `/api/conversations` — REST: list conversations as JSON
pub async fn list_conversations_handler(
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    match svc.get_conversations().await {
        Ok(convs) => axum::Json(convs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET `/api/conversations/:id/messages` — REST: messages for a conversation
pub async fn list_messages_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    match svc.get_messages(&id).await {
        Ok(msgs) => axum::Json(msgs).into_response(),
        Err(e) if e.is_not_found() => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn error_response(err: &AppError) -> Response {
    let status = if err.is_validation() {
        StatusCode::BAD_REQUEST
    } else if err.is_not_found() {
        StatusCode::NOT_FOUND
    } else if err.is_agent_unavailable() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    let tmpl = ErrorFragmentTemplate { error_message: err.to_string() };
    match tmpl.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (status, err.to_string()).into_response(),
    }
}
