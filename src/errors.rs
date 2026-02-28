use thiserror::Error;

/// Top-level application error — mirrors the Kotlin `AppError` sealed interface.
/// All variants carry a human-readable message for display/logging.
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum AppError {
    // ── Database errors ──────────────────────────────────────────────────────
    #[error("Database connection failed: {0}")]
    DatabaseConnectionFailed(#[source] sqlx::Error),

    #[error("Database query failed: {message}")]
    DatabaseQueryFailed {
        message: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("Record not found: {entity_type} with id '{id}'")]
    RecordNotFound { entity_type: String, id: String },

    // ── AI Agent errors ──────────────────────────────────────────────────────
    #[error("Ollama service unavailable at {host}")]
    OllamaUnavailable { host: String },

    #[error("Model '{model_name}' not found in Ollama")]
    ModelNotFound { model_name: String },

    #[error("Inference error: {message}")]
    InferenceError { message: String },

    // ── Validation errors ────────────────────────────────────────────────────
    #[error("Field '{field_name}' cannot be empty")]
    EmptyField { field_name: String },

    #[error("Field '{field_name}' exceeds max length of {max_length} (actual: {actual_length})")]
    FieldTooLong { field_name: String, max_length: usize, actual_length: usize },

    // ── Conversation errors ──────────────────────────────────────────────────
    #[error("Conversation '{id}' not found")]
    ConversationNotFound { id: String },

    // ── System errors ────────────────────────────────────────────────────────
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

impl AppError {
    pub fn db_query(message: impl Into<String>, source: sqlx::Error) -> Self {
        AppError::DatabaseQueryFailed { message: message.into(), source }
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, AppError::ConversationNotFound { .. } | AppError::RecordNotFound { .. })
    }

    pub fn is_validation(&self) -> bool {
        matches!(self, AppError::EmptyField { .. } | AppError::FieldTooLong { .. })
    }

    pub fn is_agent_unavailable(&self) -> bool {
        matches!(self, AppError::OllamaUnavailable { .. })
    }
}
