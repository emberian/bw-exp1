//! SeaORM entity definitions for all database tables.
//!
//! Each entity module exports:
//! - `Entity`: The entity type for queries
//! - `Model`: The model struct (read from database)
//! - `ActiveModel`: For insert/update operations
//! - `Column`: Column enumeration for query building

pub mod combat_log;
pub mod faction;
pub mod location;
pub mod mission;
pub mod mission_choice;
pub mod player;
pub mod sector;
pub mod session;
pub mod ship;
pub mod squadron;

pub mod prelude {
    pub use super::combat_log::Entity as CombatLogEntity;
    pub use super::faction::Entity as FactionEntity;
    pub use super::location::Entity as LocationEntity;
    pub use super::mission::Entity as MissionEntity;
    pub use super::mission_choice::Entity as MissionChoiceEntity;
    pub use super::player::Entity as PlayerEntity;
    pub use super::sector::Entity as SectorEntity;
    pub use super::session::Entity as SessionEntity;
    pub use super::ship::Entity as ShipEntity;
    pub use super::squadron::Entity as SquadronEntity;
}
