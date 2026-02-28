use rig::client::Nothing;
use rig::completion::Chat;
use rig::message::Message as RigMessage;
use rig::prelude::CompletionClient;
use rig::providers::ollama;
use tracing::error;

use crate::errors::AppError;
use crate::models::{Message, MessageRole};

const DEFAULT_MODEL: &str = "llama3.2";
const PREAMBLE: &str = "You are a helpful AI assistant running locally via Ollama. \
                        Be concise, accurate, and friendly. \
                        If you don't know something, say so.";

/// Builds a rig [`RigMessage`] history list from stored [`Message`] records,
/// mirroring the Kotlin `OllamaAgentService` history replay pattern.
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

/// Service that uses the rig [`ollama::Client`] to run a single chat turn.
/// A fresh agent is built per request so the history is replayed from the DB each time,
/// matching the Koog `OllamaAgentService` design.
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
                let msg = e.to_string();
                if msg.contains("Connection refused") || msg.contains("connect") {
                    AppError::OllamaUnavailable { host: self.base_url.clone() }
                } else if msg.contains("model") {
                    AppError::ModelNotFound { model_name: self.model.clone() }
                } else {
                    AppError::InferenceError { message: msg }
                }
            })?;

        Ok(Message::new(
            conversation_id.to_string(),
            MessageRole::Assistant,
            content,
        ))
    }
}
