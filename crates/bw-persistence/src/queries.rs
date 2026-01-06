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

    // ========================================================================
    // Seed data methods
    // ========================================================================

    /// Count factions in the database.
    pub async fn count_factions(&self) -> Result<u64, DbError> {
        Ok(faction::Entity::find().count(self.conn).await?)
    }

    /// Count sectors in the database.
    pub async fn count_sectors(&self) -> Result<u64, DbError> {
        Ok(sector::Entity::find().count(self.conn).await?)
    }

    /// Insert a faction.
    pub async fn insert_faction(&self, f: &Faction) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let model = faction::ActiveModel {
            id: Set(f.id.to_string()),
            name: Set(f.name.clone()),
            tag: Set(f.tag.clone()),
            faction_type: Set(format!("{:?}", f.faction_type)),
            description: Set(Some(f.description.clone())),
            philosophy: Set(Some(f.philosophy.clone())),
            aesthetic: Set(Some(f.aesthetic.clone())),
            is_playable: Set(if f.is_playable { 1 } else { 0 }),
            is_hostile: Set(if f.is_hostile { 1 } else { 0 }),
            default_standings: Set("{}".to_string()),
            created_at: Set(now),
        };
        model.insert(self.conn).await?;
        Ok(())
    }

    /// Insert a sector.
    pub async fn insert_sector(&self, s: &Sector) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let model = sector::ActiveModel {
            id: Set(s.id.to_string()),
            name: Set(s.name.clone()),
            description: Set(Some(s.description.clone())),
            bounds_min_x: Set(s.bounds.min.x),
            bounds_min_y: Set(s.bounds.min.y),
            bounds_min_z: Set(s.bounds.min.z),
            bounds_max_x: Set(s.bounds.max.x),
            bounds_max_y: Set(s.bounds.max.y),
            bounds_max_z: Set(s.bounds.max.z),
            danger_level: Set(format!("{:?}", s.danger_level).to_lowercase()),
            traffic_density: Set(format!("{:?}", s.traffic_density).to_lowercase()),
            fuel_cost_modifier: Set(s.fuel_cost_modifier),
            is_core_sector: Set(if s.is_core_sector { 1 } else { 0 }),
            controlling_faction_id: Set(s.controlling_faction.map(|id| id.to_string())),
            controlling_squadron_id: Set(s.controlling_squadron.map(|id| id.to_string())),
            adjacent_sectors: Set(serde_json::to_string(&s.adjacent_sectors).unwrap_or_default()),
            created_at: Set(now),
        };
        model.insert(self.conn).await?;
        Ok(())
    }

    /// Insert a location.
    pub async fn insert_location(&self, sector_id: Uuid, loc: &bw_core::models::Location) -> Result<(), DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let services: Vec<String> = loc.services.iter().map(|s| format!("{:?}", s)).collect();
        let model = location::ActiveModel {
            id: Set(loc.id.to_string()),
            sector_id: Set(sector_id.to_string()),
            name: Set(loc.name.clone()),
            description: Set(Some(loc.description.clone())),
            location_type: Set(format!("{:?}", loc.location_type)),
            position_x: Set(loc.position.x),
            position_y: Set(loc.position.y),
            position_z: Set(loc.position.z),
            faction_id: Set(loc.faction_id.map(|id| id.to_string())),
            services: Set(serde_json::to_string(&services).unwrap_or_default()),
            is_active: Set(1),
            created_at: Set(now),
        };
        model.insert(self.conn).await?;
        Ok(())
    }
}
