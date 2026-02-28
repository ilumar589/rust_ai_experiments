use tracing::error;
use uuid::Uuid;

use crate::agent::OllamaAgentService;
use crate::db::conversation_repository::ConversationRepository;
use crate::db::message_repository::MessageRepository;
use crate::errors::AppError;
use crate::models::{ChatRequest, ChatResponse, Conversation, Message, MessageRole};

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

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
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

        let conversation = match self.conversation_repo.find_by_id(&conversation_id).await? {
            Some(c) => c,
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
                self.conversation_repo.save(&conv).await?
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

        // ── Call the Ollama agent via rig ─────────────────────────────────────
        let assistant_message = self
            .agent
            .chat(&conversation_id, &history, &request.message)
            .await?;

        // ── Persist assistant reply & bump conversation timestamp ─────────────
        self.message_repo.save(&assistant_message).await?;
        if let Err(e) = self.conversation_repo.update_timestamp(&conversation_id).await {
            error!("Failed to update conversation timestamp: {e}");
        }

        Ok(ChatResponse {
            conversation_id: conversation.id,
            message: assistant_message,
        })
    }
}
