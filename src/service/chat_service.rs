use tracing::error;
use uuid::Uuid;

use crate::agent::OllamaAgentService;
use crate::db::conversation_repository::ConversationRepository;
use crate::db::message_repository::MessageRepository;
use crate::errors::AppError;
use crate::models::{ChatContext, ChatRequest, ChatResponse, Conversation, Message, MessageRole};

const MAX_MESSAGE_LENGTH: usize = 8000;

#[derive(Clone)]
pub struct ChatService {
    conversation_repo: ConversationRepository,
    message_repo: MessageRepository,
    agent: OllamaAgentService,
}

impl ChatService {
    pub fn new(
        conversation_repo: ConversationRepository,
        message_repo: MessageRepository,
        agent: OllamaAgentService,
    ) -> Self {
        Self { conversation_repo, message_repo, agent }
    }

    /// Expose the agent for direct streaming calls from WebSocket handlers.
    pub fn agent(&self) -> &OllamaAgentService {
        &self.agent
    }

    pub async fn get_conversations(&self) -> Result<Vec<Conversation>, AppError> {
        self.conversation_repo.find_all().await
    }

    pub async fn get_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Message>, AppError> {
        self.conversation_repo
            .find_by_id(conversation_id)
            .await?
            .ok_or_else(|| AppError::ConversationNotFound {
                id: conversation_id.to_string(),
            })?;
        self.message_repo.find_by_conversation_id(conversation_id).await
    }

    /// Non-streaming chat (POST /api/chat fallback).
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        let ctx = self.prepare_chat(request).await?;

        let assistant_message = self
            .agent
            .chat(&ctx.conversation_id, &ctx.history, &ctx.user_message)
            .await?;

        self.message_repo.save(&assistant_message).await?;
        if let Err(e) = self.conversation_repo.update_timestamp(&ctx.conversation_id).await {
            error!("Failed to update conversation timestamp: {e}");
        }

        Ok(ChatResponse {
            conversation_id: ctx.conversation_id,
            message: assistant_message,
        })
    }

    /// Validate the request, resolve/create the conversation, persist the user
    /// message, and return a [`ChatContext`] ready for the agent to process.
    ///
    /// Used by both the REST handler and the WebSocket streaming handler.
    pub async fn prepare_chat(&self, request: ChatRequest) -> Result<ChatContext, AppError> {
        // ── Validation ────────────────────────────────────────────────────────
        if request.message.trim().is_empty() {
            return Err(AppError::EmptyField { field_name: "message".to_string() });
        }
        if request.message.len() > MAX_MESSAGE_LENGTH {
            return Err(AppError::FieldTooLong {
                field_name: "message".to_string(),
                max_length: MAX_MESSAGE_LENGTH,
                actual_length: request.message.len(),
            });
        }

        // ── Resolve or create conversation ────────────────────────────────────
        let conversation_id = request
            .conversation_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        match self.conversation_repo.find_by_id(&conversation_id).await? {
            Some(_) => {}
            None => {
                let title = {
                    let t = request.message.trim();
                    if t.chars().count() > 60 {
                        format!("{}…", t.chars().take(60).collect::<String>())
                    } else {
                        t.to_string()
                    }
                };
                let conv = Conversation::new(conversation_id.clone(), title);
                self.conversation_repo.save(&conv).await?;
            }
        };

        // ── Persist user message ──────────────────────────────────────────────
        let user_message = Message::new(
            conversation_id.clone(),
            MessageRole::User,
            request.message.clone(),
        );
        self.message_repo.save(&user_message).await?;

        // ── Fetch history (excludes the just-saved user message) ──────────────
        let all_messages = self
            .message_repo
            .find_by_conversation_id(&conversation_id)
            .await?;
        let history: Vec<Message> = all_messages
            .into_iter()
            .filter(|m| m.id != user_message.id)
            .collect();

        Ok(ChatContext {
            conversation_id,
            history,
            user_message: request.message,
        })
    }

    /// Persist a complete assistant response and update the conversation timestamp.
    pub async fn save_assistant_message(
        &self,
        conversation_id: &str,
        content: &str,
    ) -> Result<Message, AppError> {
        let msg = Message::new(
            conversation_id.to_string(),
            MessageRole::Assistant,
            content.to_string(),
        );
        self.message_repo.save(&msg).await?;
        if let Err(e) = self.conversation_repo.update_timestamp(conversation_id).await {
            error!("Failed to update conversation timestamp: {e}");
        }
        Ok(msg)
    }
}
