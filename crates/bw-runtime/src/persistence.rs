//! Persistence layer re-exports and application-specific database operations
//!
//! Core persistence functionality is in bw-persistence.
//! This module re-exports it and adds application-specific helpers.

// Re-export everything from bw-persistence
pub use bw_persistence::*;

use uuid::Uuid;

/// Extension trait for Database with application-specific helpers.
pub trait DatabaseExt {
    /// Seed default admin user if it doesn't exist.
    /// Returns true if a new user was created.
    fn seed_default_admin(&self) -> impl std::future::Future<Output = Result<bool, DbError>> + Send;
}

impl DatabaseExt for Database {
    async fn seed_default_admin(&self) -> Result<bool, DbError> {
        use bw_core::models::ShipClass;

        // Check if admin user exists
        if self.username_exists("admin").await? {
            return Ok(false);
        }

        // Use known seed UUIDs (from migrations)
        let faction_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;
        let sector_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")?;

        // Create player and ship using constructors
        let player = bw_core::models::Player::new("admin".to_string(), Uuid::nil(), sector_id, faction_id);
        let player_id = player.id;

        let mut ship = bw_core::models::Ship::new_player_ship(
            "Admin's Ship".to_string(),
            player_id,
            ShipClass::PatrolCorvette,
            sector_id,
            faction_id,
        );
        let ship_id = ship.id;

        // Update player with correct ship ID
        let mut player = player;
        player.active_ship_id = ship_id;
        player.owned_ships = vec![ship_id];
        player.credits = 10000; // Give admin some starting credits

        // Set ship owner
        ship.owner_id = Some(player_id);

        // Hash password "hunter2"
        let password_hash = bw_auth::hash_password("hunter2")
            .map_err(|e| DbError::InvalidData(format!("Failed to hash password: {}", e)))?;

        self.register_player(&player, &password_hash, &ship).await?;
        Ok(true)
    }
}
