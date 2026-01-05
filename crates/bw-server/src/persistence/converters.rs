//! Converters between domain models and database rows.
//!
//! Handles UUID parsing, JSON serialization/deserialization, and timestamp conversion.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use bw_core::models::*;

use super::models::*;

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

fn uuid_vec_to_json(uuids: &[Uuid]) -> String {
    let strings: Vec<String> = uuids.iter().map(|u| u.to_string()).collect();
    serde_json::to_string(&strings).unwrap_or_else(|_| "[]".to_string())
}

// ============================================================================
// Player conversions
// ============================================================================

pub fn player_from_row(row: PlayerRow) -> Player {
    let faction_standings: Vec<FactionStanding> =
        serde_json::from_str(&row.faction_standings).unwrap_or_default();
    let stats: PlayerStats = serde_json::from_str(&row.stats).unwrap_or_default();

    Player {
        id: parse_uuid(&row.id),
        username: row.username,
        created_at: parse_datetime(&row.created_at),
        resources: PlayerResources {
            reputation: row.reputation,
            fame: row.fame,
        },
        active_ship_id: parse_uuid_opt(&row.active_ship_id).unwrap_or_else(Uuid::nil),
        patrol_sector_id: parse_uuid_opt(&row.patrol_sector_id).unwrap_or_else(Uuid::nil),
        faction_id: parse_uuid(&row.faction_id),
        squadron_id: parse_uuid_opt(&row.squadron_id),
        squadron_rank: row.squadron_rank.as_ref().and_then(|r| parse_squadron_rank(r)),
        faction_standings,
        is_online: row.is_online != 0,
        last_seen: parse_datetime_opt(&row.last_seen).unwrap_or_else(Utc::now),
        offline_attacks_remaining: row.offline_attacks_remaining,
        missions_completed: row.missions_completed,
        missions_failed: row.missions_failed,
        stats,
    }
}

pub fn player_to_insert_params(player: &Player, password_hash: &str) -> PlayerInsertParams {
    PlayerInsertParams {
        id: player.id.to_string(),
        username: player.username.clone(),
        password_hash: password_hash.to_string(),
        reputation: player.resources.reputation,
        fame: player.resources.fame,
        faction_standings: serde_json::to_string(&player.faction_standings).unwrap_or_default(),
        stats: serde_json::to_string(&player.stats).unwrap_or_default(),
        squadron_id: player.squadron_id.map(|id| id.to_string()),
        squadron_rank: player.squadron_rank.map(|r| format!("{:?}", r)),
        active_ship_id: Some(player.active_ship_id.to_string()),
        faction_id: player.faction_id.to_string(),
        patrol_sector_id: Some(player.patrol_sector_id.to_string()),
        is_online: if player.is_online { 1 } else { 0 },
        last_seen: Some(player.last_seen.to_rfc3339()),
        offline_attacks_remaining: player.offline_attacks_remaining,
        missions_completed: player.missions_completed,
        missions_failed: player.missions_failed,
        created_at: player.created_at.to_rfc3339(),
    }
}

/// Parameters for inserting a player.
pub struct PlayerInsertParams {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub reputation: i32,
    pub fame: i32,
    pub faction_standings: String,
    pub stats: String,
    pub squadron_id: Option<String>,
    pub squadron_rank: Option<String>,
    pub active_ship_id: Option<String>,
    pub faction_id: String,
    pub patrol_sector_id: Option<String>,
    pub is_online: i32,
    pub last_seen: Option<String>,
    pub offline_attacks_remaining: i32,
    pub missions_completed: i32,
    pub missions_failed: i32,
    pub created_at: String,
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

pub fn ship_from_row(row: ShipRow) -> Ship {
    let weapons: Vec<WeaponMount> = serde_json::from_str(&row.weapons).unwrap_or_default();
    let status = parse_ship_status(&row.status, &row.status_data);

    Ship {
        id: parse_uuid(&row.id),
        name: row.name,
        owner_id: parse_uuid_opt(&row.owner_id),
        ship_class: parse_ship_class(&row.ship_class),
        sector_id: parse_uuid_opt(&row.sector_id).unwrap_or_else(Uuid::nil),
        position: Position::new(row.position_x, row.position_y, row.position_z),
        resources: ShipResources {
            ammunition: row.ammunition,
            fuel: row.fuel,
        },
        crew: CrewResources {
            morale: row.morale,
            experience: row.experience,
        },
        hull_integrity: row.hull_integrity,
        shield_strength: row.shield_strength,
        weapons,
        status,
        is_player_ship: row.is_player_ship != 0,
        faction_id: parse_uuid_opt(&row.faction_id),
        squadron_id: parse_uuid_opt(&row.squadron_id),
    }
}

pub fn ship_to_insert_params(ship: &Ship) -> ShipInsertParams {
    let (status, status_data) = serialize_ship_status(&ship.status);

    ShipInsertParams {
        id: ship.id.to_string(),
        owner_id: ship.owner_id.map(|id| id.to_string()),
        name: ship.name.clone(),
        ship_class: format!("{:?}", ship.ship_class),
        sector_id: Some(ship.sector_id.to_string()),
        position_x: ship.position.x,
        position_y: ship.position.y,
        position_z: ship.position.z,
        hull_integrity: ship.hull_integrity,
        shield_strength: ship.shield_strength,
        ammunition: ship.resources.ammunition,
        fuel: ship.resources.fuel,
        morale: ship.crew.morale,
        experience: ship.crew.experience,
        weapons: serde_json::to_string(&ship.weapons).unwrap_or_else(|_| "[]".to_string()),
        status,
        status_data,
        is_player_ship: if ship.is_player_ship { 1 } else { 0 },
        faction_id: ship.faction_id.map(|id| id.to_string()),
        squadron_id: ship.squadron_id.map(|id| id.to_string()),
    }
}

/// Parameters for inserting a ship.
pub struct ShipInsertParams {
    pub id: String,
    pub owner_id: Option<String>,
    pub name: String,
    pub ship_class: String,
    pub sector_id: Option<String>,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub hull_integrity: f32,
    pub shield_strength: f32,
    pub ammunition: f32,
    pub fuel: f32,
    pub morale: f32,
    pub experience: i32,
    pub weapons: String,
    pub status: String,
    pub status_data: String,
    pub is_player_ship: i32,
    pub faction_id: Option<String>,
    pub squadron_id: Option<String>,
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

pub fn faction_from_row(row: FactionRow) -> Faction {
    Faction {
        id: parse_uuid(&row.id),
        name: row.name,
        tag: row.tag,
        faction_type: parse_faction_type(&row.faction_type),
        description: row.description.unwrap_or_default(),
        philosophy: row.philosophy.unwrap_or_default(),
        aesthetic: row.aesthetic.unwrap_or_default(),
        default_standings: Vec::new(), // Loaded separately if needed
        is_playable: row.is_playable != 0,
        is_hostile: row.is_hostile != 0,
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

pub fn sector_from_row(row: SectorRow) -> Sector {
    Sector {
        id: parse_uuid(&row.id),
        name: row.name,
        description: row.description.unwrap_or_default(),
        bounds: SectorBounds {
            min: Position::new(row.bounds_min_x, row.bounds_min_y, row.bounds_min_z),
            max: Position::new(row.bounds_max_x, row.bounds_max_y, row.bounds_max_z),
        },
        adjacent_sectors: parse_uuid_vec(&row.adjacent_sectors),
        locations: Vec::new(), // Loaded separately
        controlling_faction: parse_uuid_opt(&row.controlling_faction_id),
        controlling_squadron: parse_uuid_opt(&row.controlling_squadron_id),
        danger_level: parse_danger_level(&row.danger_level),
        fuel_cost_modifier: row.fuel_cost_modifier,
        traffic_density: parse_traffic_density(&row.traffic_density),
        is_core_sector: row.is_core_sector != 0,
    }
}

pub fn location_from_row(row: LocationRow) -> Location {
    let services: Vec<String> = serde_json::from_str(&row.services).unwrap_or_default();
    let services: Vec<StationService> = services.iter().filter_map(|s| parse_service(s)).collect();

    Location {
        id: parse_uuid(&row.id),
        name: row.name,
        description: row.description.unwrap_or_default(),
        location_type: parse_location_type(&row.location_type),
        position: Position::new(row.position_x, row.position_y, row.position_z),
        faction_id: parse_uuid_opt(&row.faction_id),
        services,
        is_active: row.is_active != 0,
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
        "ResearchStation" => LocationType::CivilianStation, // Map to civilian station
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
// Squadron conversions
// ============================================================================

pub fn squadron_from_row(row: SquadronRow) -> Squadron {
    let settings: SquadronSettings = serde_json::from_str(&row.settings).unwrap_or_default();
    let stats: SquadronStats = serde_json::from_str(&row.stats).unwrap_or_default();

    Squadron {
        id: parse_uuid(&row.id),
        name: row.name,
        tag: row.tag,
        motto: row.motto,
        description: row.description.unwrap_or_default(),
        founded_at: parse_datetime(&row.founded_at),
        leader_id: parse_uuid_opt(&row.leader_id).unwrap_or_else(Uuid::nil),
        officers: parse_uuid_vec(&row.officers),
        members: parse_uuid_vec(&row.members),
        patrol_sectors: parse_uuid_vec(&row.patrol_sectors),
        owned_stations: parse_uuid_vec(&row.owned_stations),
        owned_ships: parse_uuid_vec(&row.owned_ships),
        treasury: row.treasury,
        reputation_bonus: row.reputation_bonus as f32,
        fame_bonus: row.fame_bonus as f32,
        allied_squadrons: parse_uuid_vec(&row.allied_squadrons),
        hostile_squadrons: parse_uuid_vec(&row.hostile_squadrons),
        wargames_enabled: row.wargames_enabled != 0,
        privateering_enabled: row.privateering_enabled != 0,
        settings,
        stats,
    }
}

pub fn squadron_to_insert_params(squadron: &Squadron) -> SquadronInsertParams {
    SquadronInsertParams {
        id: squadron.id.to_string(),
        name: squadron.name.clone(),
        tag: squadron.tag.clone(),
        motto: squadron.motto.clone(),
        description: Some(squadron.description.clone()),
        leader_id: Some(squadron.leader_id.to_string()),
        officers: uuid_vec_to_json(&squadron.officers),
        members: uuid_vec_to_json(&squadron.members),
        patrol_sectors: uuid_vec_to_json(&squadron.patrol_sectors),
        owned_stations: uuid_vec_to_json(&squadron.owned_stations),
        owned_ships: uuid_vec_to_json(&squadron.owned_ships),
        treasury: squadron.treasury,
        reputation_bonus: squadron.reputation_bonus as f64,
        fame_bonus: squadron.fame_bonus as f64,
        allied_squadrons: uuid_vec_to_json(&squadron.allied_squadrons),
        hostile_squadrons: uuid_vec_to_json(&squadron.hostile_squadrons),
        wargames_enabled: if squadron.wargames_enabled { 1 } else { 0 },
        privateering_enabled: if squadron.privateering_enabled { 1 } else { 0 },
        settings: serde_json::to_string(&squadron.settings).unwrap_or_default(),
        stats: serde_json::to_string(&squadron.stats).unwrap_or_default(),
        founded_at: squadron.founded_at.to_rfc3339(),
    }
}

/// Parameters for inserting a squadron.
pub struct SquadronInsertParams {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub motto: Option<String>,
    pub description: Option<String>,
    pub leader_id: Option<String>,
    pub officers: String,
    pub members: String,
    pub patrol_sectors: String,
    pub owned_stations: String,
    pub owned_ships: String,
    pub treasury: i64,
    pub reputation_bonus: f64,
    pub fame_bonus: f64,
    pub allied_squadrons: String,
    pub hostile_squadrons: String,
    pub wargames_enabled: i32,
    pub privateering_enabled: i32,
    pub settings: String,
    pub stats: String,
    pub founded_at: String,
}

// ============================================================================
// Mission conversions
// ============================================================================

pub fn mission_from_row(row: MissionRow) -> Mission {
    let data: serde_json::Value = serde_json::from_str(&row.data).unwrap_or_default();
    let target_position = if row.target_position_x.is_some() {
        Some(Position::new(
            row.target_position_x.unwrap_or(0.0),
            row.target_position_y.unwrap_or(0.0),
            row.target_position_z.unwrap_or(0.0),
        ))
    } else {
        None
    };

    Mission {
        id: parse_uuid(&row.id),
        mission_type: parse_mission_type(&row.mission_type),
        title: row.title,
        description: row.description.unwrap_or_default(),
        script_path: row.script_path,
        current_state: row.current_state,
        data,
        sector_id: parse_uuid(&row.sector_id),
        target_position,
        target_id: parse_uuid_opt(&row.target_id),
        assigned_to: parse_uuid_opt(&row.assigned_to),
        availability: parse_mission_availability(&row.availability, &row.availability_data),
        created_at: parse_datetime(&row.created_at),
        expires_at: parse_datetime_opt(&row.expires_at),
        reputation_reward: row.reputation_reward,
        reputation_penalty: row.reputation_penalty,
        fame_reward: row.fame_reward,
        credits_reward: row.credits_reward as i64,
        status: parse_mission_status(&row.status),
        progress: row.progress as f32,
        is_high_profile: row.is_high_profile != 0,
        priority: parse_mission_priority(&row.priority),
    }
}

fn parse_mission_type(s: &str) -> MissionType {
    match s {
        "PirateIntercept" => MissionType::PirateIntercept,
        "AsteroidThreat" => MissionType::AsteroidThreat,
        "TerroristPlot" => MissionType::TerroristPlot,
        "IndustrialAccident" => MissionType::IndustrialAccident,
        "DistressSignal" => MissionType::DistressSignal,
        "Smugglers" => MissionType::Smugglers,
        "SectorDefense" => MissionType::SectorDefense,
        "FleetOperation" => MissionType::FleetOperation,
        "Escort" => MissionType::Escort,
        "Investigation" => MissionType::Investigation,
        "SquadronWargames" => MissionType::SquadronWargames,
        "Privateering" => MissionType::Privateering,
        "SeraContact" => MissionType::SeraContact,
        "DroneEncounter" => MissionType::DroneEncounter,
        _ => MissionType::DistressSignal,
    }
}

fn parse_mission_availability(availability: &str, data: &Option<String>) -> MissionAvailability {
    match availability {
        "sector_wide" => MissionAvailability::SectorWide,
        "range_restricted" => {
            let range = data
                .as_ref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(100.0);
            MissionAvailability::RangeRestricted(range)
        }
        "assigned" => {
            let player_id = data
                .as_ref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::nil);
            MissionAvailability::Assigned(player_id)
        }
        "squadron_only" => {
            let squadron_id = data
                .as_ref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::nil);
            MissionAvailability::SquadronOnly(squadron_id)
        }
        _ => MissionAvailability::SectorWide,
    }
}

fn parse_mission_status(s: &str) -> MissionStatus {
    match s {
        "available" => MissionStatus::Available,
        "in_progress" => MissionStatus::InProgress,
        "completed_success" => MissionStatus::Completed { success: true },
        "completed_failure" => MissionStatus::Completed { success: false },
        "expired" => MissionStatus::Expired,
        "abandoned" => MissionStatus::Abandoned,
        _ => MissionStatus::Available,
    }
}

fn parse_mission_priority(s: &str) -> MissionPriority {
    match s.to_lowercase().as_str() {
        "low" => MissionPriority::Low,
        "normal" => MissionPriority::Normal,
        "high" => MissionPriority::High,
        "critical" => MissionPriority::Critical,
        _ => MissionPriority::Normal,
    }
}

// ============================================================================
// Session conversions
// ============================================================================

pub fn session_from_row(row: SessionRow) -> Session {
    Session {
        id: parse_uuid(&row.id),
        player_id: parse_uuid(&row.player_id),
        token_hash: row.token_hash,
        expires_at: parse_datetime(&row.expires_at),
        created_at: parse_datetime(&row.created_at),
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
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
