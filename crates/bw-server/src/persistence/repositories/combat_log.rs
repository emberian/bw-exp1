//! Combat log repository - append-only combat history.

use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::persistence::{models::CombatLogRow, DbError};

/// A combat log entry.
#[derive(Debug, Clone)]
pub struct CombatLog {
    pub id: Uuid,
    pub sector_id: Option<Uuid>,
    pub attackers: serde_json::Value,
    pub defenders: serde_json::Value,
    pub winner: Option<String>,
    pub outcome_data: serde_json::Value,
    pub events: Vec<serde_json::Value>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Repository for combat log operations.
pub struct CombatLogRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> CombatLogRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Insert a new combat log.
    pub async fn insert(&self, log: &CombatLog) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO combat_logs (
                id, sector_id, attackers, defenders, winner,
                outcome_data, events, started_at, ended_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(log.id.to_string())
        .bind(log.sector_id.map(|id| id.to_string()))
        .bind(log.attackers.to_string())
        .bind(log.defenders.to_string())
        .bind(&log.winner)
        .bind(log.outcome_data.to_string())
        .bind(serde_json::to_string(&log.events).unwrap_or_else(|_| "[]".to_string()))
        .bind(log.started_at.to_rfc3339())
        .bind(log.ended_at.map(|dt| dt.to_rfc3339()))
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Find a combat log by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<CombatLog>, DbError> {
        let row = sqlx::query_as::<_, CombatLogRow>("SELECT * FROM combat_logs WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(combat_log_from_row))
    }

    /// Find combat logs in a sector.
    pub async fn find_by_sector(&self, sector_id: Uuid) -> Result<Vec<CombatLog>, DbError> {
        let rows = sqlx::query_as::<_, CombatLogRow>(
            "SELECT * FROM combat_logs WHERE sector_id = ? ORDER BY started_at DESC",
        )
        .bind(sector_id.to_string())
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(combat_log_from_row).collect())
    }

    /// Find recent combat logs (last N).
    pub async fn find_recent(&self, limit: u32) -> Result<Vec<CombatLog>, DbError> {
        let rows = sqlx::query_as::<_, CombatLogRow>(
            "SELECT * FROM combat_logs ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(combat_log_from_row).collect())
    }

    /// Update combat log with outcome.
    pub async fn update_outcome(
        &self,
        id: Uuid,
        winner: Option<&str>,
        outcome_data: &serde_json::Value,
        events: &[serde_json::Value],
        ended_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE combat_logs SET
                winner = ?, outcome_data = ?, events = ?, ended_at = ?
            WHERE id = ?
            "#,
        )
        .bind(winner)
        .bind(outcome_data.to_string())
        .bind(serde_json::to_string(events).unwrap_or_else(|_| "[]".to_string()))
        .bind(ended_at.to_rfc3339())
        .bind(id.to_string())
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete old combat logs (older than N days).
    pub async fn delete_older_than_days(&self, days: u32) -> Result<u64, DbError> {
        let result = sqlx::query(
            "DELETE FROM combat_logs WHERE started_at < datetime('now', ? || ' days')",
        )
        .bind(format!("-{}", days))
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

fn combat_log_from_row(row: CombatLogRow) -> CombatLog {
    CombatLog {
        id: uuid::Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil()),
        sector_id: row
            .sector_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok()),
        attackers: serde_json::from_str(&row.attackers).unwrap_or_default(),
        defenders: serde_json::from_str(&row.defenders).unwrap_or_default(),
        winner: row.winner,
        outcome_data: serde_json::from_str(&row.outcome_data).unwrap_or_default(),
        events: serde_json::from_str(&row.events).unwrap_or_default(),
        started_at: chrono::DateTime::parse_from_rfc3339(&row.started_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        ended_at: row.ended_at.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }),
    }
}
