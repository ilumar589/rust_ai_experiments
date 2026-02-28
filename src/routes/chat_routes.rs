use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

use crate::models::{Conversation, Message};
use crate::service::chat_service::ChatService;

// ── Template structs ──────────────────────────────────────────────────────────

/// View model for a message, flattened for askama template use.
pub struct MessageView {
    pub id: String,
    pub role: String,
    pub content: String,
}

impl From<&Message> for MessageView {
    fn from(m: &Message) -> Self {
        Self {
            id: m.id.clone(),
            role: m.role.as_str().to_string(),
            content: m.content.clone(),
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    conversations: Vec<Conversation>,
    /// The currently active conversation id (empty string = none)
    active_conversation_id: String,
    /// Whether there is an active conversation
    has_conversation: bool,
    conversation_title: String,
    conversation_id: String,
    messages: Vec<MessageView>,
}

#[derive(Template)]
#[template(path = "chat_panel.html")]
pub struct ChatPanelTemplate {
    pub has_conversation: bool,
    pub conversation_title: String,
    pub conversation_id: String,
    pub messages: Vec<MessageView>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET `/` — full chat page
pub async fn index_handler(
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    let conversations = svc.get_conversations().await.unwrap_or_default();
    let tmpl = IndexTemplate {
        conversations,
        active_conversation_id: String::new(),
        has_conversation: false,
        conversation_title: String::new(),
        conversation_id: String::new(),
        messages: vec![],
    };
    render(tmpl)
}

/// GET `/chat/new` — empty panel (HTMX swap into `#chat-panel`)
pub async fn new_chat_handler() -> impl IntoResponse {
    let tmpl = ChatPanelTemplate {
        has_conversation: false,
        conversation_title: String::new(),
        conversation_id: String::new(),
        messages: vec![],
    };
    render(tmpl)
}

/// GET `/chat/:id` — load existing conversation (HTMX swap into `#chat-panel`)
pub async fn load_chat_handler(
    Path(id): Path<String>,
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    let messages = svc.get_messages(&id).await.unwrap_or_default();
    let conversations = svc.get_conversations().await.unwrap_or_default();
    let conv = conversations.iter().find(|c| c.id == id).cloned();

    let tmpl = ChatPanelTemplate {
        has_conversation: conv.is_some(),
        conversation_title: conv.as_ref().map(|c| c.title.clone()).unwrap_or_default(),
        conversation_id: conv.map(|c| c.id).unwrap_or_default(),
        messages: messages.iter().map(MessageView::from).collect(),
    };
    render(tmpl)
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn render(tmpl: impl Template) -> impl IntoResponse {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {e}"),
        )
            .into_response(),
    }
}
