//! Database persistence layer
//!
//! Provides SQLite-based persistence for all game entities.

pub mod converters;
pub mod error;
mod migrations;
pub mod models;
pub mod repositories;

pub use error::DbError;
pub use repositories::*;

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

/// Database connection pool and repository access.
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Create a new database connection from a URL.
    ///
    /// URL format: `sqlite:./path/to/database.db` or `sqlite::memory:` for in-memory.
    pub async fn new(database_url: &str) -> Result<Self, DbError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let db = Self { pool };
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

    /// Get the underlying connection pool.
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
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
