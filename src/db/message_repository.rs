use sqlx::PgPool;
use tracing::error;

use crate::errors::AppError;
use crate::models::{Message, MessageRole};

#[derive(Clone)]
pub struct MessageRepository {
    pool: PgPool,
}

impl MessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_conversation_id(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Message>, AppError> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, created_at
             FROM messages
             WHERE conversation_id = $1
             ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch messages for conversation {conversation_id}: {e}");
            AppError::db_query(
                format!("Failed to fetch messages for conversation {conversation_id}"),
                e,
            )
        })?;

        rows.into_iter()
            .map(|row: sqlx::postgres::PgRow| {
                use sqlx::Row;
                let role_str: String = row.try_get("role")
                    .map_err(|e| AppError::db_query("Failed to read role", e))?;
                let role = MessageRole::try_from(role_str)
                    .map_err(|e| AppError::Unexpected(format!("Unknown message role: {e}")))?;
                Ok(Message {
                    id: row.try_get("id")
                        .map_err(|e| AppError::db_query("Failed to read id", e))?,
                    conversation_id: row.try_get("conversation_id")
                        .map_err(|e| AppError::db_query("Failed to read conversation_id", e))?,
                    role,
                    content: row.try_get("content")
                        .map_err(|e| AppError::db_query("Failed to read content", e))?,
                    created_at: row.try_get("created_at")
                        .map_err(|e| AppError::db_query("Failed to read created_at", e))?,
                })
            })
            .collect()
    }

    pub async fn save(&self, message: &Message) -> Result<Message, AppError> {
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, role, content, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(message.role.as_str())
        .bind(&message.content)
        .bind(message.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to save message {}: {e}", message.id);
            AppError::db_query("Failed to save message", e)
        })?;
        Ok(message.clone())
    }
}
