use chrono::Utc;
use sqlx::PgPool;
use tracing::error;

use crate::errors::AppError;
use crate::models::Conversation;

#[derive(Clone)]
pub struct ConversationRepository {
    pool: PgPool,
}

impl ConversationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_all(&self) -> Result<Vec<Conversation>, AppError> {
        sqlx::query_as::<_, Conversation>(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch all conversations: {e}");
            AppError::db_query("Failed to fetch conversations", e)
        })
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Conversation>, AppError> {
        sqlx::query_as::<_, Conversation>(
            "SELECT id, title, created_at, updated_at FROM conversations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to find conversation {id}: {e}");
            AppError::db_query(format!("Failed to find conversation {id}"), e)
        })
    }

    pub async fn save(&self, conversation: &Conversation) -> Result<Conversation, AppError> {
        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&conversation.id)
        .bind(&conversation.title)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to save conversation {}: {e}", conversation.id);
            AppError::db_query("Failed to save conversation", e)
        })?;
        Ok(conversation.clone())
    }

    pub async fn update_timestamp(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE conversations SET updated_at = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to update conversation timestamp {id}: {e}");
                AppError::db_query("Failed to update conversation", e)
            })?;
        Ok(())
    }
}
