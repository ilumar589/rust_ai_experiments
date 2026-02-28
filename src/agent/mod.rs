use rig::agent::MultiTurnStreamItem;
use rig::client::Nothing;
use rig::completion::Chat;
use rig::message::Message as RigMessage;
use rig::prelude::CompletionClient;
use rig::providers::ollama;
use rig::streaming::{StreamedAssistantContent, StreamingChat};
use futures_util::StreamExt;
use tracing::error;

use crate::errors::AppError;
use crate::models::{Message, MessageRole};

const DEFAULT_MODEL: &str = "llama3.2";
const PREAMBLE: &str = "You are a helpful AI assistant running locally via Ollama. \
                        Be concise, accurate, and friendly. \
                        If you don't know something, say so.";

/// Builds a rig [`RigMessage`] history list from stored [`Message`] records.
fn to_rig_history(messages: &[Message]) -> Vec<RigMessage> {
    messages
        .iter()
        .filter_map(|m| match m.role {
            MessageRole::User => Some(RigMessage::user(&m.content)),
            MessageRole::Assistant => Some(RigMessage::assistant(&m.content)),
            MessageRole::System => None, // system prompt is set via preamble
        })
        .collect()
}

/// Maps a rig error string to an [`AppError`].
fn map_rig_error(e: &str, base_url: &str, model: &str) -> AppError {
    if e.contains("Connection refused") || e.contains("connect") {
        AppError::OllamaUnavailable { host: base_url.to_string() }
    } else if e.contains("model") {
        AppError::ModelNotFound { model_name: model.to_string() }
    } else {
        AppError::InferenceError { message: e.to_string() }
    }
}

/// Service that uses the rig [`ollama::Client`] to run chat turns.
/// A fresh agent is built per request so the history is replayed from the DB each time.
#[derive(Clone)]
pub struct OllamaAgentService {
    client: ollama::Client,
    base_url: String,
    model: String,
}

impl OllamaAgentService {
    pub fn new(base_url: &str) -> Self {
        let client = ollama::Client::builder()
            .api_key(Nothing)
            .base_url(base_url)
            .build()
            .expect("Failed to build Ollama client");
        Self {
            client,
            base_url: base_url.to_string(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Sends a chat turn to the local Ollama LLM, replaying `history` as context.
    /// Returns the complete response (non-streaming).
    pub async fn chat(
        &self,
        conversation_id: &str,
        history: &[Message],
        user_message: &str,
    ) -> Result<Message, AppError> {
        let agent = self
            .client
            .agent(&self.model)
            .preamble(PREAMBLE)
            .build();

        let rig_history = to_rig_history(history);

        let content = agent
            .chat(user_message, rig_history)
            .await
            .map_err(|e| {
                error!("Ollama inference failed for conversation {conversation_id}: {e}");
                map_rig_error(&e.to_string(), &self.base_url, &self.model)
            })?;

        Ok(Message::new(
            conversation_id.to_string(),
            MessageRole::Assistant,
            content,
        ))
    }

    /// Streams a chat response from Ollama token-by-token using rig's native
    /// [`StreamingChat`] trait.
    ///
    /// Each content chunk is sent through `tx`. The caller is responsible for
    /// accumulating the full response and persisting it.
    pub async fn stream_chat(
        &self,
        conversation_id: &str,
        history: &[Message],
        user_message: &str,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<(), AppError> {
        let agent = self
            .client
            .agent(&self.model)
            .preamble(PREAMBLE)
            .build();

        let rig_history = to_rig_history(history);

        let mut stream = agent
            .stream_chat(user_message, rig_history)
            .await;

        while let Some(item) = stream.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Text(text),
                )) => {
                    // Send the text chunk to the WebSocket handler
                    if tx.send(text.text).await.is_err() {
                        // Receiver dropped — client disconnected
                        return Ok(());
                    }
                }
                Ok(_) => {
                    // Ignore tool calls, user items, final responses, etc.
                }
                Err(e) => {
                    error!("Streaming error for conversation {conversation_id}: {e}");
                    return Err(AppError::InferenceError {
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}
