//! Database error types

use thiserror::Error;

/// Database operation errors.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database connection failed: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("Record not found: {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("Duplicate record: {entity} with {field} = {value}")]
    Duplicate {
        entity: &'static str,
        field: &'static str,
        value: String,
    },

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("UUID parse error: {0}")]
    Uuid(#[from] uuid::Error),
}

impl DbError {
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }

    pub fn duplicate(entity: &'static str, field: &'static str, value: impl Into<String>) -> Self {
        Self::Duplicate {
            entity,
            field,
            value: value.into(),
        }
    }
}
