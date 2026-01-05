//! Sector model - Regions of space
//!
//! Sectors are the primary game regions. Players are assigned patrol sectors
//! and can move between them (at fuel cost). Each sector contains locations,
//! ships, and active missions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Position;

/// A sector of space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sector {
    /// Unique identifier
    pub id: Uuid,

    /// Sector name
    pub name: String,

    /// Description for flavor
    pub description: String,

    /// Spatial bounds
    pub bounds: SectorBounds,

    /// Adjacent sectors (can travel to)
    pub adjacent_sectors: Vec<Uuid>,

    /// Locations within the sector (stations, etc.)
    pub locations: Vec<Location>,

    /// Controlling faction (if any)
    pub controlling_faction: Option<Uuid>,

    /// Controlling squadron (if any)
    pub controlling_squadron: Option<Uuid>,

    /// Danger level affects random event types
    pub danger_level: DangerLevel,

    /// Fuel cost modifier (higher near planets/stars per design doc)
    pub fuel_cost_modifier: f32,

    /// Traffic density affects civilian encounters
    pub traffic_density: TrafficDensity,

    /// Whether this is a "core" sector (safe) or "margin" (frontier)
    pub is_core_sector: bool,
}

impl Sector {
    /// Create a new sector.
    pub fn new(name: String, danger_level: DangerLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            bounds: SectorBounds::default(),
            adjacent_sectors: Vec::new(),
            locations: Vec::new(),
            controlling_faction: None,
            controlling_squadron: None,
            danger_level,
            fuel_cost_modifier: 1.0,
            traffic_density: TrafficDensity::Moderate,
            is_core_sector: false,
        }
    }

    /// Get fuel cost to move within this sector.
    pub fn in_sector_fuel_cost(&self, distance: f64) -> f32 {
        // Base cost is very low for in-sector movement
        (distance as f32 * 0.1 * self.fuel_cost_modifier).max(0.1)
    }

    /// Get fuel cost to move to an adjacent sector.
    pub fn inter_sector_fuel_cost(&self) -> f32 {
        // Per design doc: fixed amount per sector, double near planets/stars
        5.0 * self.fuel_cost_modifier
    }

    /// Check if a position is within sector bounds.
    pub fn contains(&self, pos: &Position) -> bool {
        pos.x >= self.bounds.min.x
            && pos.x <= self.bounds.max.x
            && pos.y >= self.bounds.min.y
            && pos.y <= self.bounds.max.y
            && pos.z >= self.bounds.min.z
            && pos.z <= self.bounds.max.z
    }

    /// Get a random position within the sector.
    pub fn random_position(&self) -> Position {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Position {
            x: rng.gen_range(self.bounds.min.x..self.bounds.max.x),
            y: rng.gen_range(self.bounds.min.y..self.bounds.max.y),
            z: rng.gen_range(self.bounds.min.z..self.bounds.max.z),
        }
    }
}

/// Spatial bounds of a sector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorBounds {
    pub min: Position,
    pub max: Position,
}

impl Default for SectorBounds {
    fn default() -> Self {
        // Default 1000x1000x200 unit sector
        Self {
            min: Position::new(-500.0, -500.0, -100.0),
            max: Position::new(500.0, 500.0, 100.0),
        }
    }
}

/// A location within a sector (station, point of interest, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Unique identifier
    pub id: Uuid,

    /// Location name
    pub name: String,

    /// Description
    pub description: String,

    /// Type of location
    pub location_type: LocationType,

    /// Position in sector
    pub position: Position,

    /// Faction that controls this location
    pub faction_id: Option<Uuid>,

    /// Services available here
    pub services: Vec<StationService>,

    /// Whether this location is currently active/functional
    pub is_active: bool,
}

impl Location {
    pub fn new(name: String, location_type: LocationType, position: Position) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            location_type,
            position,
            faction_id: None,
            services: location_type.default_services(),
            is_active: true,
        }
    }

    /// Check if this location provides a specific service.
    pub fn has_service(&self, service: StationService) -> bool {
        self.services.contains(&service)
    }
}

/// Type of location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationType {
    // Stations
    /// Military station - free services for faction members
    NavalStation,
    /// Civilian station - paid services
    CivilianStation,
    /// Mining operation
    MiningFacility,
    /// Refinery
    ProcessingPlant,
    /// Factory
    OrbitalFactory,

    // Travel
    /// Instant travel to connected gates
    Jumpgate,

    // Points of interest
    /// Resource-rich area
    AsteroidField,
    /// Wreckage, potential salvage
    DebrisField,
    /// Unknown energy signature (mission hooks)
    Anomaly,

    // Special (from lore)
    /// Fleet graveyard - derelict ships, dangerous
    Graveyard,
    /// Lawless station - no faction control
    FreePort,
    /// Remnant archive
    Archive,
    /// Forgeborn shipyard
    Shipyard,
}

impl LocationType {
    /// Get default services for this location type.
    pub fn default_services(&self) -> Vec<StationService> {
        match self {
            Self::NavalStation => vec![
                StationService::Refuel,
                StationService::Rearm,
                StationService::Repair,
                StationService::MissionBoard,
                StationService::ShoreLeave,
            ],
            Self::CivilianStation => vec![
                StationService::Refuel,
                StationService::Trade,
                StationService::Repair,
            ],
            Self::MiningFacility => vec![
                StationService::Refuel,
                StationService::Trade,
            ],
            Self::ProcessingPlant => vec![
                StationService::Trade,
            ],
            Self::OrbitalFactory => vec![
                StationService::Trade,
                StationService::ShipUpgrade,
            ],
            Self::Jumpgate => vec![
                StationService::JumpgateAccess,
            ],
            Self::FreePort => vec![
                StationService::Refuel,
                StationService::Rearm,
                StationService::Repair,
                StationService::Trade,
                StationService::BlackMarket,
            ],
            Self::Shipyard => vec![
                StationService::Repair,
                StationService::ShipUpgrade,
                StationService::ShipPurchase,
            ],
            Self::Archive => vec![
                StationService::Trade,
                StationService::Information,
            ],
            _ => vec![],
        }
    }

    /// Whether ships can dock here.
    pub fn is_dockable(&self) -> bool {
        matches!(
            self,
            Self::NavalStation
                | Self::CivilianStation
                | Self::MiningFacility
                | Self::ProcessingPlant
                | Self::OrbitalFactory
                | Self::FreePort
                | Self::Shipyard
                | Self::Archive
        )
    }

    /// Whether this is a station (provides services).
    pub fn is_station(&self) -> bool {
        matches!(
            self,
            Self::NavalStation
                | Self::CivilianStation
                | Self::MiningFacility
                | Self::ProcessingPlant
                | Self::OrbitalFactory
                | Self::FreePort
                | Self::Shipyard
                | Self::Archive
        )
    }
}

/// Services available at a location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StationService {
    /// Restore fuel to 100%
    Refuel,
    /// Restore ammunition to 100%
    Rearm,
    /// Repair hull and shields
    Repair,
    /// Buy/sell cargo
    Trade,
    /// Black market goods (illegal)
    BlackMarket,
    /// Crew rest (morale recovery per design doc)
    ShoreLeave,
    /// Install new equipment
    ShipUpgrade,
    /// Buy new ships
    ShipPurchase,
    /// View available missions
    MissionBoard,
    /// Use jumpgate
    JumpgateAccess,
    /// Access archives/databases
    Information,
}

/// Danger level of a sector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DangerLevel {
    /// Core systems - minimal threats
    Safe,
    /// Standard patrol sectors
    Moderate,
    /// Frontier - pirates, criminals
    Dangerous,
    /// Enemy territory - Sera, drones
    Hostile,
}

impl DangerLevel {
    /// Probability of hostile encounter per tick.
    pub fn encounter_chance(&self) -> f32 {
        match self {
            Self::Safe => 0.001,
            Self::Moderate => 0.005,
            Self::Dangerous => 0.02,
            Self::Hostile => 0.05,
        }
    }

    /// Multiplier for mission rewards in this danger level.
    pub fn reward_multiplier(&self) -> f32 {
        match self {
            Self::Safe => 0.5,
            Self::Moderate => 1.0,
            Self::Dangerous => 1.5,
            Self::Hostile => 2.5,
        }
    }
}

/// Traffic density affects civilian encounters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficDensity {
    Sparse,
    Light,
    Moderate,
    Heavy,
    Congested,
}

impl TrafficDensity {
    /// Average civilian ships in sector.
    pub fn civilian_ship_count(&self) -> u32 {
        match self {
            Self::Sparse => 2,
            Self::Light => 5,
            Self::Moderate => 10,
            Self::Heavy => 20,
            Self::Congested => 40,
        }
    }
}
