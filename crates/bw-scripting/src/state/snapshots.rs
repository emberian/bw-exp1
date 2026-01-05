//! Read-only state snapshots for scripts
//!
//! Snapshots are immutable views of game state that scripts receive when querying.
//! They can be converted to Rhai Dynamic maps for script consumption.

use uuid::Uuid;

use bw_core::models::{
    DangerLevel, Position, Ship, ShipStatus, Player, Sector, Location,
    LocationType, TrafficDensity, CombatStance, CargoItem, GameMode, InstalledUpgrade,
};
use crate::RhaiSerialize;

/// Read-only snapshot of a ship's state.
#[derive(Debug, Clone, RhaiSerialize)]
pub struct ShipSnapshot {
    #[rhai(as_string)]
    pub id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub ship_class: String,
    #[rhai(as_string)]
    pub sector_id: Uuid,
    pub position: PositionSnapshot,
    pub hull: f32,
    pub shields: f32,
    pub ammunition: f32,
    pub fuel: f32,
    pub morale: f32,
    pub experience: i32,
    pub status: String,
    pub is_player_ship: bool,
    pub faction_id: Option<Uuid>,
    pub can_attack: bool,
    pub can_move: bool,
    // Calculated combat stats
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub sensor_range: f32,
    // Combat control
    pub combat_stance: String,
    pub locked_target: Option<Uuid>,
    // Cargo
    pub cargo: Vec<CargoSnapshot>,
    pub cargo_capacity: u32,
    pub cargo_used: u32,
    // Upgrades
    pub upgrades: Vec<UpgradeSnapshot>,
}

/// Read-only snapshot of an installed upgrade.
#[derive(Debug, Clone, RhaiSerialize)]
pub struct UpgradeSnapshot {
    pub upgrade_id: String,
    pub slot: String,
}

impl UpgradeSnapshot {
    pub fn from_upgrade(upgrade: &InstalledUpgrade) -> Self {
        Self {
            upgrade_id: upgrade.upgrade_id.clone(),
            slot: upgrade.slot.clone(),
        }
    }
}

/// Read-only snapshot of a cargo item.
#[derive(Debug, Clone, RhaiSerialize)]
pub struct CargoSnapshot {
    #[rhai(rename = "type")]
    pub cargo_type: String,
    pub quantity: u32,
    #[rhai(rename = "price")]
    pub purchase_price: i64,
}

impl CargoSnapshot {
    pub fn from_cargo(item: &CargoItem) -> Self {
        Self {
            cargo_type: item.cargo_type.clone(),
            quantity: item.quantity,
            purchase_price: item.purchase_price,
        }
    }
}

impl ShipSnapshot {
    /// Create a snapshot from a Ship reference.
    pub fn from_ship(ship: &Ship) -> Self {
        let combat_stats = ship.combat_effectiveness();

        Self {
            id: ship.id,
            name: ship.name.clone(),
            owner_id: ship.owner_id,
            ship_class: format!("{:?}", ship.ship_class),
            sector_id: ship.sector_id,
            position: PositionSnapshot::from_position(&ship.position),
            hull: ship.hull_integrity,
            shields: ship.shield_strength,
            ammunition: ship.resources.ammunition,
            fuel: ship.resources.fuel,
            morale: ship.crew.morale,
            experience: ship.crew.experience,
            status: status_to_string(&ship.status),
            is_player_ship: ship.is_player_ship,
            faction_id: ship.faction_id,
            can_attack: ship.can_attack(),
            can_move: ship.can_move(),
            attack: combat_stats.attack,
            defense: combat_stats.defense,
            speed: combat_stats.speed,
            sensor_range: combat_stats.sensor_range,
            combat_stance: combat_stance_to_string(&ship.combat_stance),
            locked_target: ship.locked_target,
            cargo: ship.cargo.iter().map(CargoSnapshot::from_cargo).collect(),
            cargo_capacity: ship.cargo_capacity(),
            cargo_used: ship.cargo_used(),
            upgrades: ship.upgrades.iter().map(UpgradeSnapshot::from_upgrade).collect(),
        }
    }
}

/// Read-only snapshot of a position.
#[derive(Debug, Clone, Copy, RhaiSerialize)]
pub struct PositionSnapshot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl PositionSnapshot {
    pub fn from_position(pos: &Position) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }
    }

    pub fn distance_to(&self, other: &PositionSnapshot) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Read-only snapshot of a player's state.
#[derive(Debug, Clone, RhaiSerialize)]
pub struct PlayerSnapshot {
    #[rhai(as_string)]
    pub id: Uuid,
    pub username: String,
    pub reputation: i32,
    pub fame: i32,
    pub credits: i64,
    pub game_mode: String,
    pub owned_ships: Vec<Uuid>,
    #[rhai(as_string)]
    pub active_ship_id: Uuid,
    #[rhai(as_string)]
    pub sector_id: Uuid,
    #[rhai(as_string)]
    pub faction_id: Uuid,
    pub squadron_id: Option<Uuid>,
    pub is_online: bool,
    pub missions_completed: i32,
    pub missions_failed: i32,
    pub is_disgraced: bool,
}

impl PlayerSnapshot {
    /// Create a snapshot from a Player reference.
    pub fn from_player(player: &Player) -> Self {
        Self {
            id: player.id,
            username: player.username.clone(),
            reputation: player.resources.reputation,
            fame: player.resources.fame,
            credits: player.credits,
            game_mode: game_mode_to_string(&player.game_mode),
            owned_ships: player.owned_ships.clone(),
            active_ship_id: player.active_ship_id,
            sector_id: player.patrol_sector_id,
            faction_id: player.faction_id,
            squadron_id: player.squadron_id,
            is_online: player.is_online,
            missions_completed: player.missions_completed,
            missions_failed: player.missions_failed,
            is_disgraced: player.is_disgraced(),
        }
    }
}

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
            position: PositionSnapshot::from_position(&loc.position),
            faction_id: loc.faction_id,
            is_active: loc.is_active,
            is_dockable: loc.location_type.is_dockable(),
        }
    }
}

// Helper functions to convert enums to strings

fn status_to_string(status: &ShipStatus) -> String {
    match status {
        ShipStatus::Idle => "idle".to_string(),
        ShipStatus::InTransit { .. } => "in_transit".to_string(),
        ShipStatus::InCombat { .. } => "in_combat".to_string(),
        ShipStatus::Docked { .. } => "docked".to_string(),
        ShipStatus::Disabled => "disabled".to_string(),
        ShipStatus::Destroyed => "destroyed".to_string(),
    }
}

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

fn combat_stance_to_string(stance: &CombatStance) -> String {
    match stance {
        CombatStance::Aggressive => "aggressive".to_string(),
        CombatStance::Balanced => "balanced".to_string(),
        CombatStance::Defensive => "defensive".to_string(),
        CombatStance::Evasive => "evasive".to_string(),
    }
}

fn game_mode_to_string(mode: &GameMode) -> String {
    match mode {
        GameMode::Standard => "standard".to_string(),
        GameMode::Hardcore => "hardcore".to_string(),
    }
}
