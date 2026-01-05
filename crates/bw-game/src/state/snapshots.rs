//! Read-only state snapshots for scripts
//!
//! Re-exports unified types from bw-shared and provides conversion helpers.
//! Scripts receive these immutable views when querying game state.

use uuid::Uuid;

use bw_core::models::{
    DangerLevel, Position, Ship as CoreShip, Player as CorePlayer, Sector, Location,
    LocationType, TrafficDensity,
};
use crate::RhaiSerialize;

// =============================================================================
// Re-export unified types from bw-shared
// =============================================================================

/// Unified ship snapshot - re-exported from bw-shared.
pub use bw_shared::dto::Ship as ShipSnapshot;

/// Unified player snapshot - re-exported from bw-shared.
pub use bw_shared::dto::Player as PlayerSnapshot;

/// Position snapshot - re-exported from bw-shared.
pub use bw_shared::dto::PositionDto as PositionSnapshot;

/// Cargo item snapshot - re-exported from bw-shared.
pub use bw_shared::dto::CargoItem as CargoSnapshot;

/// Installed upgrade snapshot - re-exported from bw-shared.
pub use bw_shared::dto::InstalledUpgrade as UpgradeSnapshot;

// =============================================================================
// Conversion helpers
// =============================================================================

/// Create a ShipSnapshot from a core Ship model.
///
/// The `faction_tag` must be resolved externally since it requires
/// looking up the faction by ID.
pub fn ship_snapshot_from_ship(ship: &CoreShip, faction_tag: Option<String>) -> ShipSnapshot {
    bw_shared::dto::Ship::from_core(ship, faction_tag)
}

/// Create a PlayerSnapshot from a core Player model.
///
/// Tags and admin status must be resolved externally.
pub fn player_snapshot_from_player(
    player: &CorePlayer,
    faction_tag: String,
    squadron_tag: Option<String>,
    is_admin: bool,
) -> PlayerSnapshot {
    bw_shared::dto::Player::from_core(player, faction_tag, squadron_tag, is_admin)
}

/// Create a PositionSnapshot from a Position.
pub fn position_from_core(pos: &Position) -> PositionSnapshot {
    PositionSnapshot {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

/// Calculate distance between two positions.
pub fn position_distance(a: &PositionSnapshot, b: &PositionSnapshot) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

// =============================================================================
// Sector and Location snapshots (kept local for now)
// =============================================================================

/// Read-only snapshot of a sector's state.
#[derive(Debug, Clone, RhaiSerialize)]
pub struct SectorSnapshot {
    #[rhai(as_string)]
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
    pub traffic_density: String,
    pub is_core_sector: bool,
    pub controlling_faction: Option<Uuid>,
    pub controlling_squadron: Option<Uuid>,
    pub locations: Vec<LocationSnapshot>,
}

impl SectorSnapshot {
    /// Create a snapshot from a Sector reference.
    pub fn from_sector(sector: &Sector) -> Self {
        Self {
            id: sector.id,
            name: sector.name.clone(),
            danger_level: danger_level_to_string(&sector.danger_level),
            traffic_density: traffic_density_to_string(&sector.traffic_density),
            is_core_sector: sector.is_core_sector,
            controlling_faction: sector.controlling_faction,
            controlling_squadron: sector.controlling_squadron,
            locations: sector.locations.iter().map(LocationSnapshot::from_location).collect(),
        }
    }
}

/// Read-only snapshot of a location within a sector.
#[derive(Debug, Clone, RhaiSerialize)]
pub struct LocationSnapshot {
    #[rhai(as_string)]
    pub id: Uuid,
    pub name: String,
    pub location_type: String,
    pub position: PositionSnapshot,
    pub faction_id: Option<Uuid>,
    pub is_active: bool,
    pub is_dockable: bool,
}

impl LocationSnapshot {
    pub fn from_location(loc: &Location) -> Self {
        Self {
            id: loc.id,
            name: loc.name.clone(),
            location_type: location_type_to_string(&loc.location_type),
            position: position_from_core(&loc.position),
            faction_id: loc.faction_id,
            is_active: loc.is_active,
            is_dockable: loc.location_type.is_dockable(),
        }
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn danger_level_to_string(level: &DangerLevel) -> String {
    match level {
        DangerLevel::Safe => "safe".to_string(),
        DangerLevel::Moderate => "moderate".to_string(),
        DangerLevel::Dangerous => "dangerous".to_string(),
        DangerLevel::Hostile => "hostile".to_string(),
    }
}

fn traffic_density_to_string(density: &TrafficDensity) -> String {
    match density {
        TrafficDensity::Sparse => "sparse".to_string(),
        TrafficDensity::Light => "light".to_string(),
        TrafficDensity::Moderate => "moderate".to_string(),
        TrafficDensity::Heavy => "heavy".to_string(),
        TrafficDensity::Congested => "congested".to_string(),
    }
}

fn location_type_to_string(loc_type: &LocationType) -> String {
    match loc_type {
        LocationType::NavalStation => "naval_station".to_string(),
        LocationType::CivilianStation => "civilian_station".to_string(),
        LocationType::MiningFacility => "mining_facility".to_string(),
        LocationType::ProcessingPlant => "processing_plant".to_string(),
        LocationType::OrbitalFactory => "orbital_factory".to_string(),
        LocationType::Jumpgate => "jumpgate".to_string(),
        LocationType::AsteroidField => "asteroid_field".to_string(),
        LocationType::DebrisField => "debris_field".to_string(),
        LocationType::Anomaly => "anomaly".to_string(),
        LocationType::Graveyard => "graveyard".to_string(),
        LocationType::FreePort => "free_port".to_string(),
        LocationType::Archive => "archive".to_string(),
        LocationType::Shipyard => "shipyard".to_string(),
    }
}
