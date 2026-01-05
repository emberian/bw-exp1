//! Squadron repository - CRUD operations for squadrons.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use bw_core::models::Squadron;

use crate::persistence::{converters, models::SquadronRow, DbError};

/// Repository for squadron operations.
pub struct SquadronRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> SquadronRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Find a squadron by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Squadron>, DbError> {
        let row = sqlx::query_as::<_, SquadronRow>("SELECT * FROM squadrons WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::squadron_from_row))
    }

    /// Find a squadron by name (case-insensitive).
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Squadron>, DbError> {
        let row = sqlx::query_as::<_, SquadronRow>(
            "SELECT * FROM squadrons WHERE name = ? COLLATE NOCASE",
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(converters::squadron_from_row))
    }

    /// Find a squadron by tag (case-insensitive).
    pub async fn find_by_tag(&self, tag: &str) -> Result<Option<Squadron>, DbError> {
        let row = sqlx::query_as::<_, SquadronRow>(
            "SELECT * FROM squadrons WHERE tag = ? COLLATE NOCASE",
        )
        .bind(tag)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(converters::squadron_from_row))
    }

    /// Check if a squadron name already exists.
    pub async fn name_exists(&self, name: &str) -> Result<bool, DbError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM squadrons WHERE name = ? COLLATE NOCASE",
        )
        .bind(name)
        .fetch_one(self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Check if a squadron tag already exists.
    pub async fn tag_exists(&self, tag: &str) -> Result<bool, DbError> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM squadrons WHERE tag = ? COLLATE NOCASE")
                .bind(tag)
                .fetch_one(self.pool)
                .await?;

        Ok(count.0 > 0)
    }

    /// Get all squadrons.
    pub async fn find_all(&self) -> Result<Vec<Squadron>, DbError> {
        let rows = sqlx::query_as::<_, SquadronRow>("SELECT * FROM squadrons ORDER BY name")
            .fetch_all(self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(converters::squadron_from_row)
            .collect())
    }

    /// Insert a new squadron.
    pub async fn insert(&self, squadron: &Squadron) -> Result<(), DbError> {
        let params = converters::squadron_to_insert_params(squadron);

        sqlx::query(
            r#"
            INSERT INTO squadrons (
                id, name, tag, motto, description, leader_id,
                officers, members, patrol_sectors, owned_stations, owned_ships,
                treasury, reputation_bonus, fame_bonus,
                allied_squadrons, hostile_squadrons,
                wargames_enabled, privateering_enabled, settings, stats, founded_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&params.id)
        .bind(&params.name)
        .bind(&params.tag)
        .bind(&params.motto)
        .bind(&params.description)
        .bind(&params.leader_id)
        .bind(&params.officers)
        .bind(&params.members)
        .bind(&params.patrol_sectors)
        .bind(&params.owned_stations)
        .bind(&params.owned_ships)
        .bind(params.treasury)
        .bind(params.reputation_bonus)
        .bind(params.fame_bonus)
        .bind(&params.allied_squadrons)
        .bind(&params.hostile_squadrons)
        .bind(params.wargames_enabled)
        .bind(params.privateering_enabled)
        .bind(&params.settings)
        .bind(&params.stats)
        .bind(&params.founded_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update a squadron.
    pub async fn update(&self, squadron: &Squadron) -> Result<(), DbError> {
        let params = converters::squadron_to_insert_params(squadron);

        sqlx::query(
            r#"
            UPDATE squadrons SET
                name = ?, tag = ?, motto = ?, description = ?, leader_id = ?,
                officers = ?, members = ?, patrol_sectors = ?, owned_stations = ?, owned_ships = ?,
                treasury = ?, reputation_bonus = ?, fame_bonus = ?,
                allied_squadrons = ?, hostile_squadrons = ?,
                wargames_enabled = ?, privateering_enabled = ?, settings = ?, stats = ?
            WHERE id = ?
            "#,
        )
        .bind(&params.name)
        .bind(&params.tag)
        .bind(&params.motto)
        .bind(&params.description)
        .bind(&params.leader_id)
        .bind(&params.officers)
        .bind(&params.members)
        .bind(&params.patrol_sectors)
        .bind(&params.owned_stations)
        .bind(&params.owned_ships)
        .bind(params.treasury)
        .bind(params.reputation_bonus)
        .bind(params.fame_bonus)
        .bind(&params.allied_squadrons)
        .bind(&params.hostile_squadrons)
        .bind(params.wargames_enabled)
        .bind(params.privateering_enabled)
        .bind(&params.settings)
        .bind(&params.stats)
        .bind(&params.id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete a squadron.
    pub async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM squadrons WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
