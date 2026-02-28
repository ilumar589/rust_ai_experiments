use serde::{Deserialize, Serialize};

/// Matches the backend `Conversation` model.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Matches the backend `Message` model.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Request body for the chat API and WebSocket.
#[derive(Clone, Debug, Serialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Response from the REST chat API.
#[derive(Clone, Debug, Deserialize)]
pub struct ChatResponse {
    pub conversation_id: String,
    pub message: Message,
}

/// WebSocket request sent by the client.
#[derive(Clone, Debug, Serialize)]
pub struct WsChatRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// WebSocket event received from the server.
/// Matches the backend `WsEvent` enum (internally tagged).
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    #[serde(rename = "stream_start")]
    StreamStart { conversation_id: String },
    #[serde(rename = "stream_chunk")]
    StreamChunk { content: String },
    #[serde(rename = "stream_end")]
    StreamEnd {
        full_content: String,
        #[serde(default)]
        message_id: Option<String>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}
