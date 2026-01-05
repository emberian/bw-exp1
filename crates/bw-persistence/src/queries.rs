//! SeaORM-based database queries.
//!
//! Provides typed query methods for all entity types.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, ActiveModelTrait, Set, PaginatorTrait};
use uuid::Uuid;

use bw_core::models::{Faction, Player, Sector, Ship};

use crate::converters::{self, Session};
use crate::entities::{faction, location, player, sector, session, ship};
use crate::DbError;

/// Query methods for the database.
pub struct Queries<'a> {
    conn: &'a DatabaseConnection,
}

impl<'a> Queries<'a> {
    pub fn new(conn: &'a DatabaseConnection) -> Self {
        Self { conn }
    }

    // ========================================================================
    // Player queries
    // ========================================================================

    /// Find a player by ID.
    pub async fn find_player_by_id(&self, id: Uuid) -> Result<Option<Player>, DbError> {
        let model = player::Entity::find_by_id(id.to_string())
            .one(self.conn)
            .await?;
        Ok(model.map(converters::player_from_model))
    }

    /// Find a player by username (case-insensitive).
    pub async fn find_player_by_username(&self, username: &str) -> Result<Option<Player>, DbError> {
        // SQLite COLLATE NOCASE via raw comparison
        let model = player::Entity::find()
            .filter(player::Column::Username.eq(username))
            .one(self.conn)
            .await?;
        Ok(model.map(converters::player_from_model))
    }

    /// Find a player by username and return with password hash.
    pub async fn find_player_with_password(&self, username: &str) -> Result<Option<(Player, String)>, DbError> {
        let model = player::Entity::find()
            .filter(player::Column::Username.eq(username))
            .one(self.conn)
            .await?;
        Ok(model.map(|m| {
            let password_hash = m.password_hash.clone();
            (converters::player_from_model(m), password_hash)
        }))
    }

    /// Check if a username exists.
    pub async fn username_exists(&self, username: &str) -> Result<bool, DbError> {
        let count = player::Entity::find()
            .filter(player::Column::Username.eq(username))
            .count(self.conn)
            .await?;
        Ok(count > 0)
    }

    // ========================================================================
    // Ship queries
    // ========================================================================

    /// Find a ship by ID.
    pub async fn find_ship_by_id(&self, id: Uuid) -> Result<Option<Ship>, DbError> {
        let model = ship::Entity::find_by_id(id.to_string())
            .one(self.conn)
            .await?;
        Ok(model.map(converters::ship_from_model))
    }

    // ========================================================================
    // Faction queries
    // ========================================================================

    /// Find all factions.
    pub async fn find_all_factions(&self) -> Result<Vec<Faction>, DbError> {
        let models = faction::Entity::find()
            .all(self.conn)
            .await?;
        Ok(models.into_iter().map(converters::faction_from_model).collect())
    }

    // ========================================================================
    // Sector queries
    // ========================================================================

    /// Find all sectors with their locations.
    pub async fn find_all_sectors_with_locations(&self) -> Result<Vec<Sector>, DbError> {
        let sector_models = sector::Entity::find()
            .all(self.conn)
            .await?;

        let mut sectors = Vec::new();
        for sector_model in sector_models {
            let sector_id = sector_model.id.clone();
            let mut sector = converters::sector_from_model(sector_model);

            // Load locations for this sector
            let location_models = location::Entity::find()
                .filter(location::Column::SectorId.eq(&sector_id))
                .all(self.conn)
                .await?;

            sector.locations = location_models
                .into_iter()
                .map(converters::location_from_model)
                .collect();

            sectors.push(sector);
        }

        Ok(sectors)
    }

    // ========================================================================
    // Session queries
    // ========================================================================

    /// Find a session by token hash.
    pub async fn find_session_by_token(&self, token_hash: &str) -> Result<Option<Session>, DbError> {
        let model = session::Entity::find()
            .filter(session::Column::TokenHash.eq(token_hash))
            .one(self.conn)
            .await?;
        Ok(model.map(converters::session_from_model))
    }

    /// Create a new session.
    pub async fn create_session(&self, sess: &Session) -> Result<(), DbError> {
        let active_model = session::ActiveModel {
            id: Set(sess.id.to_string()),
            player_id: Set(sess.player_id.to_string()),
            token_hash: Set(sess.token_hash.clone()),
            expires_at: Set(sess.expires_at.to_rfc3339()),
            created_at: Set(sess.created_at.to_rfc3339()),
        };
        active_model.insert(self.conn).await?;
        Ok(())
    }

    /// Delete a session by ID.
    pub async fn delete_session(&self, id: Uuid) -> Result<bool, DbError> {
        let result = session::Entity::delete_by_id(id.to_string())
            .exec(self.conn)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Delete expired sessions.
    pub async fn delete_expired_sessions(&self) -> Result<u64, DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = session::Entity::delete_many()
            .filter(session::Column::ExpiresAt.lt(&now))
            .exec(self.conn)
            .await?;
        Ok(result.rows_affected)
    }

    // ========================================================================
    // Registration (atomic player + ship insert)
    // ========================================================================

    /// Register a new player with their ship atomically.
    pub async fn register_player_atomically(
        &self,
        new_player: &Player,
        password_hash: &str,
        new_ship: &Ship,
    ) -> Result<(), DbError> {
        use sea_orm::TransactionTrait;

        let txn = self.conn.begin().await?;

        // Insert player
        let player_model = converters::player_to_active_model(new_player, password_hash);
        player_model.insert(&txn).await?;

        // Insert ship
        let ship_model = converters::ship_to_active_model(new_ship);
        ship_model.insert(&txn).await?;

        txn.commit().await?;
        Ok(())
    }
}
