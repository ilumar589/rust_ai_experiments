use gloo_net::http::Request;

use crate::models::{ChatRequest, ChatResponse, Conversation, Message};

/// Base URL of the backend API server.
const API_BASE: &str = "http://localhost:3000";

/// Fetches the list of all conversations from the backend.
pub async fn fetch_conversations() -> Result<Vec<Conversation>, String> {
    let resp = Request::get(&format!("{API_BASE}/api/conversations"))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }

    resp.json::<Vec<Conversation>>()
        .await
        .map_err(|e| format!("Parse error: {e}"))
}

/// Fetches all messages for a given conversation.
pub async fn fetch_messages(conversation_id: &str) -> Result<Vec<Message>, String> {
    let resp = Request::get(&format!(
        "{API_BASE}/api/conversations/{conversation_id}/messages"
    ))
    .send()
    .await
    .map_err(|e| format!("Network error: {e}"))?;

    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }

    resp.json::<Vec<Message>>()
        .await
        .map_err(|e| format!("Parse error: {e}"))
}

/// Sends a chat message via the REST API (non-streaming).
pub async fn send_chat(
    message: &str,
    conversation_id: Option<&str>,
) -> Result<ChatResponse, String> {
    let body = ChatRequest {
        message: message.to_string(),
        conversation_id: conversation_id.map(|s| s.to_string()),
    };

    let resp = Request::post(&format!("{API_BASE}/api/chat"))
        .json(&body)
        .map_err(|e| format!("Serialize error: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !resp.ok() {
        return Err(format!("Server error: {}", resp.status()));
    }

    resp.json::<ChatResponse>()
        .await
        .map_err(|e| format!("Parse error: {e}"))
}

/// Returns the WebSocket URL for the chat streaming endpoint.
pub fn ws_url() -> String {
    format!("ws://localhost:3000/ws/chat")
}
