//! Session repository - authentication session management.

use chrono::{DateTime, Duration, Utc};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::persistence::{converters, converters::Session, models::SessionRow, DbError};

/// Repository for session operations.
pub struct SessionRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> SessionRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Create a new session.
    pub async fn create(
        &self,
        player_id: Uuid,
        token_hash: &str,
        duration: Duration,
    ) -> Result<Session, DbError> {
        let session = Session {
            id: Uuid::new_v4(),
            player_id,
            token_hash: token_hash.to_string(),
            expires_at: Utc::now() + duration,
            created_at: Utc::now(),
        };

        sqlx::query(
            "INSERT INTO sessions (id, player_id, token_hash, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session.id.to_string())
        .bind(session.player_id.to_string())
        .bind(&session.token_hash)
        .bind(session.expires_at.to_rfc3339())
        .bind(session.created_at.to_rfc3339())
        .execute(self.pool)
        .await?;

        Ok(session)
    }

    /// Find a session by token hash.
    pub async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, DbError> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE token_hash = ?")
            .bind(token_hash)
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::session_from_row))
    }

    /// Find a session by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Session>, DbError> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::session_from_row))
    }

    /// Find all sessions for a player.
    pub async fn find_by_player(&self, player_id: Uuid) -> Result<Vec<Session>, DbError> {
        let rows =
            sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE player_id = ?")
                .bind(player_id.to_string())
                .fetch_all(self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(converters::session_from_row)
            .collect())
    }

    /// Delete a session by ID.
    pub async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a session by token hash.
    pub async fn delete_by_token_hash(&self, token_hash: &str) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(token_hash)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all sessions for a player (logout from all devices).
    pub async fn delete_for_player(&self, player_id: Uuid) -> Result<u64, DbError> {
        let result = sqlx::query("DELETE FROM sessions WHERE player_id = ?")
            .bind(player_id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all expired sessions.
    pub async fn delete_expired(&self) -> Result<u64, DbError> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < datetime('now')")
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Extend a session's expiration.
    pub async fn extend(&self, id: Uuid, new_expires_at: DateTime<Utc>) -> Result<bool, DbError> {
        let result = sqlx::query("UPDATE sessions SET expires_at = ? WHERE id = ?")
            .bind(new_expires_at.to_rfc3339())
            .bind(id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
