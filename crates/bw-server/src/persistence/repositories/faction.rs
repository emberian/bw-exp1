//! Faction repository - read-only access to factions.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use bw_core::models::Faction;

use crate::persistence::{converters, models::FactionRow, DbError};

/// Repository for faction operations.
pub struct FactionRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> FactionRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Find a faction by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Faction>, DbError> {
        let row = sqlx::query_as::<_, FactionRow>("SELECT * FROM factions WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::faction_from_row))
    }

    /// Find a faction by name (case-insensitive).
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Faction>, DbError> {
        let row = sqlx::query_as::<_, FactionRow>(
            "SELECT * FROM factions WHERE name = ? COLLATE NOCASE",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(converters::faction_from_row))
    }

    /// Find a faction by tag (case-insensitive).
    pub async fn find_by_tag(&self, tag: &str) -> Result<Option<Faction>, DbError> {
        let row =
            sqlx::query_as::<_, FactionRow>("SELECT * FROM factions WHERE tag = ? COLLATE NOCASE")
                .bind(tag)
                .fetch_optional(self.pool)
                .await?;

        Ok(row.map(converters::faction_from_row))
    }

    /// Get all factions.
    pub async fn find_all(&self) -> Result<Vec<Faction>, DbError> {
        let rows = sqlx::query_as::<_, FactionRow>("SELECT * FROM factions ORDER BY name")
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(converters::faction_from_row).collect())
    }

    /// Get all playable factions.
    pub async fn find_playable(&self) -> Result<Vec<Faction>, DbError> {
        let rows = sqlx::query_as::<_, FactionRow>(
            "SELECT * FROM factions WHERE is_playable = 1 ORDER BY name",
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(converters::faction_from_row).collect())
    }

    /// Get all hostile factions.
    pub async fn find_hostile(&self) -> Result<Vec<Faction>, DbError> {
        let rows = sqlx::query_as::<_, FactionRow>(
            "SELECT * FROM factions WHERE is_hostile = 1 ORDER BY name",
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(converters::faction_from_row).collect())
    }
}
