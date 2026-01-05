//! Database row types
//!
//! These types map directly to SQLite table rows. They use String for UUIDs
//! and JSON fields, with conversion handled in the converters module.

use sqlx::FromRow;

/// Player table row.
#[derive(Debug, Clone, FromRow)]
pub struct PlayerRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub reputation: i32,
    pub fame: i32,
    pub faction_standings: String, // JSON
    pub stats: String,             // JSON
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
    pub updated_at: String,
}

/// Ship table row.
#[derive(Debug, Clone, FromRow)]
pub struct ShipRow {
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
    pub weapons: String, // JSON
    pub status: String,
    pub status_data: String, // JSON
    pub is_player_ship: i32,
    pub faction_id: Option<String>,
    pub squadron_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Sector table row.
#[derive(Debug, Clone, FromRow)]
pub struct SectorRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub bounds_min_x: f64,
    pub bounds_min_y: f64,
    pub bounds_min_z: f64,
    pub bounds_max_x: f64,
    pub bounds_max_y: f64,
    pub bounds_max_z: f64,
    pub danger_level: String,
    pub traffic_density: String,
    pub fuel_cost_modifier: f32,
    pub is_core_sector: i32,
    pub controlling_faction_id: Option<String>,
    pub controlling_squadron_id: Option<String>,
    pub adjacent_sectors: String, // JSON array
    pub created_at: String,
}

/// Location table row.
#[derive(Debug, Clone, FromRow)]
pub struct LocationRow {
    pub id: String,
    pub sector_id: String,
    pub name: String,
    pub description: Option<String>,
    pub location_type: String,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub faction_id: Option<String>,
    pub services: String, // JSON array
    pub is_active: i32,
    pub created_at: String,
}

/// Faction table row.
#[derive(Debug, Clone, FromRow)]
pub struct FactionRow {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub faction_type: String,
    pub description: Option<String>,
    pub philosophy: Option<String>,
    pub aesthetic: Option<String>,
    pub is_playable: i32,
    pub is_hostile: i32,
    pub default_standings: String, // JSON
    pub created_at: String,
}

/// Squadron table row.
#[derive(Debug, Clone, FromRow)]
pub struct SquadronRow {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub motto: Option<String>,
    pub description: Option<String>,
    pub leader_id: Option<String>,
    pub officers: String,       // JSON array
    pub members: String,        // JSON array
    pub patrol_sectors: String, // JSON array
    pub owned_stations: String, // JSON array
    pub owned_ships: String,    // JSON array
    pub treasury: i64,
    pub reputation_bonus: f64,
    pub fame_bonus: f64,
    pub allied_squadrons: String,  // JSON array
    pub hostile_squadrons: String, // JSON array
    pub wargames_enabled: i32,
    pub privateering_enabled: i32,
    pub settings: String, // JSON
    pub stats: String,    // JSON
    pub founded_at: String,
    pub updated_at: String,
}

/// Mission table row.
#[derive(Debug, Clone, FromRow)]
pub struct MissionRow {
    pub id: String,
    pub mission_type: String,
    pub title: String,
    pub description: Option<String>,
    pub script_path: String,
    pub current_state: String,
    pub data: String, // JSON
    pub sector_id: String,
    pub target_position_x: Option<f64>,
    pub target_position_y: Option<f64>,
    pub target_position_z: Option<f64>,
    pub target_id: Option<String>,
    pub assigned_to: Option<String>,
    pub availability: String,
    pub availability_data: Option<String>,
    pub reputation_reward: i32,
    pub reputation_penalty: i32,
    pub fame_reward: i32,
    pub credits_reward: i32,
    pub status: String,
    pub progress: f64,
    pub is_high_profile: i32,
    pub priority: String,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Session table row (for authentication).
#[derive(Debug, Clone, FromRow)]
pub struct SessionRow {
    pub id: String,
    pub player_id: String,
    pub token_hash: String,
    pub expires_at: String,
    pub created_at: String,
}

/// Combat log table row.
#[derive(Debug, Clone, FromRow)]
pub struct CombatLogRow {
    pub id: String,
    pub sector_id: Option<String>,
    pub attackers: String,    // JSON
    pub defenders: String,    // JSON
    pub winner: Option<String>,
    pub outcome_data: String, // JSON
    pub events: String,       // JSON array
    pub started_at: String,
    pub ended_at: Option<String>,
}

/// Mission choice table row.
#[derive(Debug, Clone, FromRow)]
pub struct MissionChoiceRow {
    pub id: String,
    pub mission_id: String,
    pub player_id: String,
    pub choice_id: String,
    pub choice_label: Option<String>,
    pub outcome: Option<String>, // JSON
    pub created_at: String,
}
