//! Converters between domain models and SeaORM entities.
//!
//! Handles UUID parsing, JSON serialization/deserialization, and timestamp conversion.

use chrono::{DateTime, Utc};
use sea_orm::Set;
use uuid::Uuid;

use bw_core::models::*;

use super::entities::{faction, location, player, sector, session, ship};

// ============================================================================
// Helper functions
// ============================================================================

fn parse_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_else(|_| Uuid::nil())
}

fn parse_uuid_opt(s: &Option<String>) -> Option<Uuid> {
    s.as_ref().and_then(|s| Uuid::parse_str(s).ok())
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_datetime_opt(s: &Option<String>) -> Option<DateTime<Utc>> {
    s.as_ref().and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

fn parse_uuid_vec(json: &str) -> Vec<Uuid> {
    serde_json::from_str::<Vec<String>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect()
}

// ============================================================================
// Player conversions
// ============================================================================

pub fn player_from_model(model: player::Model) -> Player {
    let faction_standings: Vec<FactionStanding> =
        serde_json::from_str(&model.faction_standings).unwrap_or_default();
    let stats: PlayerStats = serde_json::from_str(&model.stats).unwrap_or_default();

    Player {
        id: parse_uuid(&model.id),
        username: model.username,
        created_at: parse_datetime(&model.created_at),
        resources: PlayerResources {
            reputation: model.reputation,
            fame: model.fame,
        },
        active_ship_id: parse_uuid_opt(&model.active_ship_id).unwrap_or_else(Uuid::nil),
        patrol_sector_id: parse_uuid_opt(&model.patrol_sector_id).unwrap_or_else(Uuid::nil),
        faction_id: parse_uuid(&model.faction_id),
        squadron_id: parse_uuid_opt(&model.squadron_id),
        squadron_rank: model.squadron_rank.as_ref().and_then(|r| parse_squadron_rank(r)),
        faction_standings,
        is_online: model.is_online != 0,
        last_seen: parse_datetime_opt(&model.last_seen).unwrap_or_else(Utc::now),
        offline_attacks_remaining: model.offline_attacks_remaining,
        missions_completed: model.missions_completed,
        missions_failed: model.missions_failed,
        stats,
    }
}

pub fn player_to_active_model(p: &Player, password_hash: &str) -> player::ActiveModel {
    player::ActiveModel {
        id: Set(p.id.to_string()),
        username: Set(p.username.clone()),
        password_hash: Set(password_hash.to_string()),
        reputation: Set(p.resources.reputation),
        fame: Set(p.resources.fame),
        faction_standings: Set(serde_json::to_string(&p.faction_standings).unwrap_or_default()),
        stats: Set(serde_json::to_string(&p.stats).unwrap_or_default()),
        squadron_id: Set(p.squadron_id.map(|id| id.to_string())),
        squadron_rank: Set(p.squadron_rank.map(|r| format!("{:?}", r))),
        active_ship_id: Set(Some(p.active_ship_id.to_string())),
        faction_id: Set(p.faction_id.to_string()),
        patrol_sector_id: Set(Some(p.patrol_sector_id.to_string())),
        is_online: Set(if p.is_online { 1 } else { 0 }),
        last_seen: Set(Some(p.last_seen.to_rfc3339())),
        offline_attacks_remaining: Set(p.offline_attacks_remaining),
        missions_completed: Set(p.missions_completed),
        missions_failed: Set(p.missions_failed),
        created_at: Set(p.created_at.to_rfc3339()),
        updated_at: Set(Utc::now().to_rfc3339()),
    }
}

fn parse_squadron_rank(s: &str) -> Option<SquadronRank> {
    match s.to_lowercase().as_str() {
        "member" => Some(SquadronRank::Member),
        "officer" => Some(SquadronRank::Officer),
        "leader" => Some(SquadronRank::Leader),
        _ => None,
    }
}

// ============================================================================
// Ship conversions
// ============================================================================

pub fn ship_from_model(model: ship::Model) -> Ship {
    let weapons: Vec<WeaponMount> = serde_json::from_str(&model.weapons).unwrap_or_default();
    let status = parse_ship_status(&model.status, &model.status_data);

    Ship {
        id: parse_uuid(&model.id),
        name: model.name,
        owner_id: parse_uuid_opt(&model.owner_id),
        ship_class: parse_ship_class(&model.ship_class),
        sector_id: parse_uuid_opt(&model.sector_id).unwrap_or_else(Uuid::nil),
        position: Position::new(model.position_x, model.position_y, model.position_z),
        resources: ShipResources {
            ammunition: model.ammunition,
            fuel: model.fuel,
        },
        crew: CrewResources {
            morale: model.morale,
            experience: model.experience,
        },
        hull_integrity: model.hull_integrity,
        shield_strength: model.shield_strength,
        weapons,
        status,
        is_player_ship: model.is_player_ship != 0,
        faction_id: parse_uuid_opt(&model.faction_id),
        squadron_id: parse_uuid_opt(&model.squadron_id),
    }
}

pub fn ship_to_active_model(s: &Ship) -> ship::ActiveModel {
    let (status, status_data) = serialize_ship_status(&s.status);

    ship::ActiveModel {
        id: Set(s.id.to_string()),
        owner_id: Set(s.owner_id.map(|id| id.to_string())),
        name: Set(s.name.clone()),
        ship_class: Set(format!("{:?}", s.ship_class)),
        sector_id: Set(Some(s.sector_id.to_string())),
        position_x: Set(s.position.x),
        position_y: Set(s.position.y),
        position_z: Set(s.position.z),
        hull_integrity: Set(s.hull_integrity),
        shield_strength: Set(s.shield_strength),
        ammunition: Set(s.resources.ammunition),
        fuel: Set(s.resources.fuel),
        morale: Set(s.crew.morale),
        experience: Set(s.crew.experience),
        weapons: Set(serde_json::to_string(&s.weapons).unwrap_or_else(|_| "[]".to_string())),
        status: Set(status),
        status_data: Set(status_data),
        is_player_ship: Set(if s.is_player_ship { 1 } else { 0 }),
        faction_id: Set(s.faction_id.map(|id| id.to_string())),
        squadron_id: Set(s.squadron_id.map(|id| id.to_string())),
        created_at: Set(Utc::now().to_rfc3339()),
        updated_at: Set(Utc::now().to_rfc3339()),
    }
}

fn parse_ship_class(s: &str) -> ShipClass {
    match s {
        "PatrolCorvette" => ShipClass::PatrolCorvette,
        "Frigate" => ShipClass::Frigate,
        "Destroyer" => ShipClass::Destroyer,
        "Cruiser" => ShipClass::Cruiser,
        "Carrier" => ShipClass::Carrier,
        "Freighter" => ShipClass::Freighter,
        "Transport" => ShipClass::Transport,
        "MiningVessel" => ShipClass::MiningVessel,
        "PirateRaider" => ShipClass::PirateRaider,
        "PirateFrigate" => ShipClass::PirateFrigate,
        "TerroristBomber" => ShipClass::TerroristBomber,
        "SeraSwarm" => ShipClass::SeraSwarm,
        "SeraHunter" => ShipClass::SeraHunter,
        "DroneHarvester" => ShipClass::DroneHarvester,
        "DroneSwarm" => ShipClass::DroneSwarm,
        _ => ShipClass::PatrolCorvette,
    }
}

fn parse_ship_status(status: &str, status_data: &str) -> ShipStatus {
    match status {
        "idle" => ShipStatus::Idle,
        "in_transit" => {
            let data: serde_json::Value = serde_json::from_str(status_data).unwrap_or_default();
            let destination = Position::new(
                data["destination_x"].as_f64().unwrap_or(0.0),
                data["destination_y"].as_f64().unwrap_or(0.0),
                data["destination_z"].as_f64().unwrap_or(0.0),
            );
            let target_id = data["target_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok());
            ShipStatus::InTransit {
                destination,
                target_id,
            }
        }
        "in_combat" => {
            let data: serde_json::Value = serde_json::from_str(status_data).unwrap_or_default();
            let engagement_id = data["engagement_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::nil);
            ShipStatus::InCombat { engagement_id }
        }
        "docked" => {
            let data: serde_json::Value = serde_json::from_str(status_data).unwrap_or_default();
            let station_id = data["station_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::nil);
            ShipStatus::Docked { station_id }
        }
        "disabled" => ShipStatus::Disabled,
        "destroyed" => ShipStatus::Destroyed,
        _ => ShipStatus::Idle,
    }
}

fn serialize_ship_status(status: &ShipStatus) -> (String, String) {
    match status {
        ShipStatus::Idle => ("idle".to_string(), "{}".to_string()),
        ShipStatus::InTransit {
            destination,
            target_id,
        } => {
            let data = serde_json::json!({
                "destination_x": destination.x,
                "destination_y": destination.y,
                "destination_z": destination.z,
                "target_id": target_id.map(|id| id.to_string()),
            });
            ("in_transit".to_string(), data.to_string())
        }
        ShipStatus::InCombat { engagement_id } => {
            let data = serde_json::json!({
                "engagement_id": engagement_id.to_string(),
            });
            ("in_combat".to_string(), data.to_string())
        }
        ShipStatus::Docked { station_id } => {
            let data = serde_json::json!({
                "station_id": station_id.to_string(),
            });
            ("docked".to_string(), data.to_string())
        }
        ShipStatus::Disabled => ("disabled".to_string(), "{}".to_string()),
        ShipStatus::Destroyed => ("destroyed".to_string(), "{}".to_string()),
    }
}

// ============================================================================
// Faction conversions
// ============================================================================

pub fn faction_from_model(model: faction::Model) -> Faction {
    Faction {
        id: parse_uuid(&model.id),
        name: model.name,
        tag: model.tag,
        faction_type: parse_faction_type(&model.faction_type),
        description: model.description.unwrap_or_default(),
        philosophy: model.philosophy.unwrap_or_default(),
        aesthetic: model.aesthetic.unwrap_or_default(),
        default_standings: Vec::new(),
        is_playable: model.is_playable != 0,
        is_hostile: model.is_hostile != 0,
    }
}

fn parse_faction_type(s: &str) -> FactionType {
    match s {
        "ContinuityCompact" => FactionType::ContinuityCompact,
        "ArgentFlotilla" => FactionType::ArgentFlotilla,
        "Forgeborn" => FactionType::Forgeborn,
        "Illuminate" => FactionType::Illuminate,
        "Remnant" => FactionType::Remnant,
        "HollowCircuit" => FactionType::HollowCircuit,
        "Sera" => FactionType::Sera,
        "DroneIntelligence" => FactionType::DroneIntelligence,
        "Pirates" => FactionType::Pirates,
        "Terrorists" => FactionType::Terrorists,
        "Squadron" => FactionType::Squadron,
        _ => FactionType::Independent,
    }
}

// ============================================================================
// Sector conversions
// ============================================================================

pub fn sector_from_model(model: sector::Model) -> Sector {
    Sector {
        id: parse_uuid(&model.id),
        name: model.name,
        description: model.description.unwrap_or_default(),
        bounds: SectorBounds {
            min: Position::new(model.bounds_min_x, model.bounds_min_y, model.bounds_min_z),
            max: Position::new(model.bounds_max_x, model.bounds_max_y, model.bounds_max_z),
        },
        adjacent_sectors: parse_uuid_vec(&model.adjacent_sectors),
        locations: Vec::new(), // Loaded separately
        controlling_faction: parse_uuid_opt(&model.controlling_faction_id),
        controlling_squadron: parse_uuid_opt(&model.controlling_squadron_id),
        danger_level: parse_danger_level(&model.danger_level),
        fuel_cost_modifier: model.fuel_cost_modifier,
        traffic_density: parse_traffic_density(&model.traffic_density),
        is_core_sector: model.is_core_sector != 0,
    }
}

pub fn location_from_model(model: location::Model) -> Location {
    let services: Vec<String> = serde_json::from_str(&model.services).unwrap_or_default();
    let services: Vec<StationService> = services.iter().filter_map(|s| parse_service(s)).collect();

    Location {
        id: parse_uuid(&model.id),
        name: model.name,
        description: model.description.unwrap_or_default(),
        location_type: parse_location_type(&model.location_type),
        position: Position::new(model.position_x, model.position_y, model.position_z),
        faction_id: parse_uuid_opt(&model.faction_id),
        services,
        is_active: model.is_active != 0,
    }
}

fn parse_danger_level(s: &str) -> DangerLevel {
    match s.to_lowercase().as_str() {
        "safe" => DangerLevel::Safe,
        "moderate" => DangerLevel::Moderate,
        "dangerous" => DangerLevel::Dangerous,
        "hostile" => DangerLevel::Hostile,
        _ => DangerLevel::Moderate,
    }
}

fn parse_traffic_density(s: &str) -> TrafficDensity {
    match s.to_lowercase().as_str() {
        "sparse" => TrafficDensity::Sparse,
        "light" => TrafficDensity::Light,
        "moderate" => TrafficDensity::Moderate,
        "heavy" => TrafficDensity::Heavy,
        "congested" => TrafficDensity::Congested,
        _ => TrafficDensity::Moderate,
    }
}

fn parse_location_type(s: &str) -> LocationType {
    match s {
        "NavalStation" => LocationType::NavalStation,
        "CivilianStation" => LocationType::CivilianStation,
        "MiningFacility" => LocationType::MiningFacility,
        "ProcessingPlant" => LocationType::ProcessingPlant,
        "OrbitalFactory" => LocationType::OrbitalFactory,
        "Jumpgate" => LocationType::Jumpgate,
        "AsteroidField" => LocationType::AsteroidField,
        "DebrisField" => LocationType::DebrisField,
        "Anomaly" => LocationType::Anomaly,
        "Graveyard" => LocationType::Graveyard,
        "FreePort" => LocationType::FreePort,
        "Archive" => LocationType::Archive,
        "Shipyard" => LocationType::Shipyard,
        _ => LocationType::CivilianStation,
    }
}

fn parse_service(s: &str) -> Option<StationService> {
    match s.to_lowercase().as_str() {
        "refuel" => Some(StationService::Refuel),
        "rearm" => Some(StationService::Rearm),
        "repair" => Some(StationService::Repair),
        "trade" => Some(StationService::Trade),
        "blackmarket" | "black_market" => Some(StationService::BlackMarket),
        "shoreleave" | "shore_leave" => Some(StationService::ShoreLeave),
        "shipupgrade" | "ship_upgrade" => Some(StationService::ShipUpgrade),
        "shippurchase" | "ship_purchase" => Some(StationService::ShipPurchase),
        "missionboard" | "mission_board" | "missions" => Some(StationService::MissionBoard),
        "jumpgateaccess" | "jumpgate" => Some(StationService::JumpgateAccess),
        "information" => Some(StationService::Information),
        _ => None,
    }
}

// ============================================================================
// Session conversions
// ============================================================================

pub fn session_from_model(model: session::Model) -> Session {
    Session {
        id: parse_uuid(&model.id),
        player_id: parse_uuid(&model.player_id),
        token_hash: model.token_hash,
        expires_at: parse_datetime(&model.expires_at),
        created_at: parse_datetime(&model.created_at),
    }
}

/// Session model for persistence.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub player_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(player_id: Uuid, token_hash: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            player_id,
            token_hash,
            expires_at,
            created_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
