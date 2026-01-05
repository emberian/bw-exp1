//! Player repository - CRUD operations for players.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use bw_core::models::Player;

use crate::persistence::{converters, models::PlayerRow, DbError};

/// Repository for player operations.
pub struct PlayerRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> PlayerRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Find a player by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Player>, DbError> {
        let row = sqlx::query_as::<_, PlayerRow>("SELECT * FROM players WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::player_from_row))
    }

    /// Find a player by username (case-insensitive).
    pub async fn find_by_username(&self, username: &str) -> Result<Option<Player>, DbError> {
        let row = sqlx::query_as::<_, PlayerRow>(
            "SELECT * FROM players WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(converters::player_from_row))
    }

    /// Find a player by username and return with password hash for authentication.
    pub async fn find_by_username_with_password(
        &self,
        username: &str,
    ) -> Result<Option<(Player, String)>, DbError> {
        let row = sqlx::query_as::<_, PlayerRow>(
            "SELECT * FROM players WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|r| {
            let password_hash = r.password_hash.clone();
            (converters::player_from_row(r), password_hash)
        }))
    }

    /// Check if a username already exists.
    pub async fn username_exists(&self, username: &str) -> Result<bool, DbError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM players WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_one(self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Insert a new player.
    pub async fn insert(&self, player: &Player, password_hash: &str) -> Result<(), DbError> {
        let params = converters::player_to_insert_params(player, password_hash);

        sqlx::query(
            r#"
            INSERT INTO players (
                id, username, password_hash, reputation, fame,
                faction_standings, stats, squadron_id, squadron_rank,
                active_ship_id, faction_id, patrol_sector_id,
                is_online, last_seen, offline_attacks_remaining,
                missions_completed, missions_failed, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&params.id)
        .bind(&params.username)
        .bind(&params.password_hash)
        .bind(params.reputation)
        .bind(params.fame)
        .bind(&params.faction_standings)
        .bind(&params.stats)
        .bind(&params.squadron_id)
        .bind(&params.squadron_rank)
        .bind(&params.active_ship_id)
        .bind(&params.faction_id)
        .bind(&params.patrol_sector_id)
        .bind(params.is_online)
        .bind(&params.last_seen)
        .bind(params.offline_attacks_remaining)
        .bind(params.missions_completed)
        .bind(params.missions_failed)
        .bind(&params.created_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update a player.
    pub async fn update(&self, player: &Player) -> Result<(), DbError> {
        let params = converters::player_to_insert_params(player, ""); // Password not updated here

        sqlx::query(
            r#"
            UPDATE players SET
                reputation = ?, fame = ?, faction_standings = ?,
                stats = ?, squadron_id = ?, squadron_rank = ?,
                active_ship_id = ?, faction_id = ?, patrol_sector_id = ?,
                is_online = ?, last_seen = ?, offline_attacks_remaining = ?,
                missions_completed = ?, missions_failed = ?
            WHERE id = ?
            "#,
        )
        .bind(params.reputation)
        .bind(params.fame)
        .bind(&params.faction_standings)
        .bind(&params.stats)
        .bind(&params.squadron_id)
        .bind(&params.squadron_rank)
        .bind(&params.active_ship_id)
        .bind(&params.faction_id)
        .bind(&params.patrol_sector_id)
        .bind(params.is_online)
        .bind(&params.last_seen)
        .bind(params.offline_attacks_remaining)
        .bind(params.missions_completed)
        .bind(params.missions_failed)
        .bind(&params.id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update a player's password.
    pub async fn update_password(&self, player_id: Uuid, new_hash: &str) -> Result<(), DbError> {
        sqlx::query("UPDATE players SET password_hash = ? WHERE id = ?")
            .bind(new_hash)
            .bind(player_id.to_string())
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Set a player's online status.
    pub async fn set_online(&self, player_id: Uuid, online: bool) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE players SET is_online = ?, last_seen = ? WHERE id = ?")
            .bind(if online { 1 } else { 0 })
            .bind(&now)
            .bind(player_id.to_string())
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Get all online players.
    pub async fn find_online(&self) -> Result<Vec<Player>, DbError> {
        let rows = sqlx::query_as::<_, PlayerRow>("SELECT * FROM players WHERE is_online = 1")
            .fetch_all(self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(converters::player_from_row)
            .collect())
    }

    /// Get all players in a squadron.
    pub async fn find_by_squadron(&self, squadron_id: Uuid) -> Result<Vec<Player>, DbError> {
        let rows = sqlx::query_as::<_, PlayerRow>("SELECT * FROM players WHERE squadron_id = ?")
            .bind(squadron_id.to_string())
            .fetch_all(self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(converters::player_from_row)
            .collect())
    }

    /// Delete a player.
    pub async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM players WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
