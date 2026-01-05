//! Database persistence layer
//!
//! Provides SQLite-based persistence for all game entities.

pub mod converters;
pub mod entities;
pub mod error;
mod migrations;
pub mod models;
pub mod repositories;
pub mod tracked;
pub mod worker;

pub use error::DbError;
pub use repositories::*;
pub use tracked::TrackedDashMap;
pub use worker::{Persistence, spawn_persistence_worker};

use sea_orm::DatabaseConnection;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};

use bw_core::models::{Player, Ship};

/// Database connection pool and repository access.
pub struct Database {
    /// SQLx pool for legacy repositories
    pool: Pool<Sqlite>,
    /// SeaORM connection for new persistence layer
    sea_conn: DatabaseConnection,
}

impl Database {
    /// Create a new database connection from a URL.
    ///
    /// URL format: `sqlite:./path/to/database.db` or `sqlite::memory:` for in-memory.
    pub async fn new(database_url: &str) -> Result<Self, DbError> {
        // Create SQLx pool for legacy repositories
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Create SeaORM connection for new persistence layer
        let sea_conn = sea_orm::Database::connect(database_url).await?;

        // Enable WAL mode for better durability and concurrent reads
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous=NORMAL")
            .execute(&pool)
            .await?;

        let db = Self { pool, sea_conn };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Create a new in-memory database (useful for testing).
    pub async fn new_in_memory() -> Result<Self, DbError> {
        Self::new("sqlite::memory:").await
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<(), DbError> {
        migrations::run(&self.pool).await
    }

    /// Get the underlying SQLx connection pool.
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Get the SeaORM database connection.
    pub fn sea_connection(&self) -> &DatabaseConnection {
        &self.sea_conn
    }

    /// Spawn the persistence worker and return a handle.
    ///
    /// The worker runs in the background, batching and persisting entity changes.
    pub fn spawn_persistence(&self) -> Persistence {
        spawn_persistence_worker(self.sea_conn.clone())
    }

    /// Register a new player with their ship atomically.
    ///
    /// This ensures both player and ship are created together, or neither is.
    pub async fn register_player_atomically(
        &self,
        player: &Player,
        password_hash: &str,
        ship: &Ship,
    ) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;

        // Insert player
        let player_params = converters::player_to_insert_params(player, password_hash);
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
        .bind(&player_params.id)
        .bind(&player_params.username)
        .bind(&player_params.password_hash)
        .bind(player_params.reputation)
        .bind(player_params.fame)
        .bind(&player_params.faction_standings)
        .bind(&player_params.stats)
        .bind(&player_params.squadron_id)
        .bind(&player_params.squadron_rank)
        .bind(&player_params.active_ship_id)
        .bind(&player_params.faction_id)
        .bind(&player_params.patrol_sector_id)
        .bind(player_params.is_online)
        .bind(&player_params.last_seen)
        .bind(player_params.offline_attacks_remaining)
        .bind(player_params.missions_completed)
        .bind(player_params.missions_failed)
        .bind(&player_params.created_at)
        .execute(&mut *tx)
        .await?;

        // Insert ship
        let ship_params = converters::ship_to_insert_params(ship);
        sqlx::query(
            r#"
            INSERT INTO ships (
                id, owner_id, name, ship_class, sector_id,
                position_x, position_y, position_z,
                hull_integrity, shield_strength, ammunition, fuel,
                morale, experience, weapons, status, status_data,
                is_player_ship, faction_id, squadron_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&ship_params.id)
        .bind(&ship_params.owner_id)
        .bind(&ship_params.name)
        .bind(&ship_params.ship_class)
        .bind(&ship_params.sector_id)
        .bind(ship_params.position_x)
        .bind(ship_params.position_y)
        .bind(ship_params.position_z)
        .bind(ship_params.hull_integrity)
        .bind(ship_params.shield_strength)
        .bind(ship_params.ammunition)
        .bind(ship_params.fuel)
        .bind(ship_params.morale)
        .bind(ship_params.experience)
        .bind(&ship_params.weapons)
        .bind(&ship_params.status)
        .bind(&ship_params.status_data)
        .bind(ship_params.is_player_ship)
        .bind(&ship_params.faction_id)
        .bind(&ship_params.squadron_id)
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        Ok(())
    }

    /// Get the player repository.
    pub fn players(&self) -> PlayerRepository<'_> {
        PlayerRepository::new(&self.pool)
    }

    /// Get the ship repository.
    pub fn ships(&self) -> ShipRepository<'_> {
        ShipRepository::new(&self.pool)
    }

    /// Get the sector repository.
    pub fn sectors(&self) -> SectorRepository<'_> {
        SectorRepository::new(&self.pool)
    }

    /// Get the faction repository.
    pub fn factions(&self) -> FactionRepository<'_> {
        FactionRepository::new(&self.pool)
    }

    /// Get the squadron repository.
    pub fn squadrons(&self) -> SquadronRepository<'_> {
        SquadronRepository::new(&self.pool)
    }

    /// Get the mission repository.
    pub fn missions(&self) -> MissionRepository<'_> {
        MissionRepository::new(&self.pool)
    }

    /// Get the session repository.
    pub fn sessions(&self) -> SessionRepository<'_> {
        SessionRepository::new(&self.pool)
    }

    /// Get the combat log repository.
    pub fn combat_logs(&self) -> CombatLogRepository<'_> {
        CombatLogRepository::new(&self.pool)
    }
}
