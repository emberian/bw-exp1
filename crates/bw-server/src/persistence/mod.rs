//! Database persistence layer
//!
//! Provides SQLite-based persistence using SeaORM.

pub mod converters;
pub mod entities;
pub mod error;
mod migrations;
pub mod queries;
pub mod tracked;
pub mod worker;

pub use converters::Session;
pub use error::DbError;
pub use queries::Queries;
pub use tracked::TrackedDashMap;
pub use worker::{Persistence, spawn_persistence_worker};

use sea_orm::DatabaseConnection;

use bw_core::models::{Faction, Player, Sector, Ship};
use uuid::Uuid;

/// Database connection and query access.
pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    /// Create a new database connection from a URL.
    ///
    /// URL format: `sqlite:./path/to/database.db` or `sqlite::memory:` for in-memory.
    pub async fn new(database_url: &str) -> Result<Self, DbError> {
        let conn = sea_orm::Database::connect(database_url).await?;

        // Enable WAL mode for better durability and concurrent reads
        use sea_orm::ConnectionTrait;
        conn.execute_unprepared("PRAGMA journal_mode=WAL").await?;
        conn.execute_unprepared("PRAGMA synchronous=NORMAL").await?;

        let db = Self { conn };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Create a new in-memory database (useful for testing).
    pub async fn new_in_memory() -> Result<Self, DbError> {
        Self::new("sqlite::memory:").await
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<(), DbError> {
        migrations::run_sea(&self.conn).await
    }

    /// Get the SeaORM database connection.
    pub fn connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Spawn the persistence worker and return a handle.
    pub fn spawn_persistence(&self) -> Persistence {
        spawn_persistence_worker(self.conn.clone())
    }

    /// Get query methods.
    pub fn queries(&self) -> Queries<'_> {
        Queries::new(&self.conn)
    }

    // ========================================================================
    // Convenience methods that delegate to Queries
    // ========================================================================

    /// Find a player by ID.
    pub async fn find_player(&self, id: Uuid) -> Result<Option<Player>, DbError> {
        self.queries().find_player_by_id(id).await
    }

    /// Find a player by username with password hash.
    pub async fn find_player_with_password(&self, username: &str) -> Result<Option<(Player, String)>, DbError> {
        self.queries().find_player_with_password(username).await
    }

    /// Check if username exists.
    pub async fn username_exists(&self, username: &str) -> Result<bool, DbError> {
        self.queries().username_exists(username).await
    }

    /// Find a ship by ID.
    pub async fn find_ship(&self, id: Uuid) -> Result<Option<Ship>, DbError> {
        self.queries().find_ship_by_id(id).await
    }

    /// Find all factions.
    pub async fn find_all_factions(&self) -> Result<Vec<Faction>, DbError> {
        self.queries().find_all_factions().await
    }

    /// Find all sectors with locations.
    pub async fn find_all_sectors(&self) -> Result<Vec<Sector>, DbError> {
        self.queries().find_all_sectors_with_locations().await
    }

    /// Find session by token hash.
    pub async fn find_session(&self, token_hash: &str) -> Result<Option<Session>, DbError> {
        self.queries().find_session_by_token(token_hash).await
    }

    /// Create a session.
    pub async fn create_session(&self, session: &Session) -> Result<(), DbError> {
        self.queries().create_session(session).await
    }

    /// Delete a session.
    pub async fn delete_session(&self, id: Uuid) -> Result<bool, DbError> {
        self.queries().delete_session(id).await
    }

    /// Delete expired sessions.
    pub async fn delete_expired_sessions(&self) -> Result<u64, DbError> {
        self.queries().delete_expired_sessions().await
    }

    /// Register a new player with ship atomically.
    pub async fn register_player(&self, player: &Player, password_hash: &str, ship: &Ship) -> Result<(), DbError> {
        self.queries().register_player_atomically(player, password_hash, ship).await
    }
}
