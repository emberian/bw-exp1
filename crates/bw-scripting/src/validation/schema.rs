//! Object schema definitions for nested access validation.
//!
//! Provides schemas for game objects (Ship, Player, etc.) to validate
//! that scripts access valid fields.

use rhai::Position;

// ============================================================================
// Schema Types
// ============================================================================

/// Type of a field in an object schema for nested access validation.
#[derive(Debug, Clone, PartialEq)]
pub enum NestedFieldType {
    String,
    Int,
    Float,
    Bool,
    Uuid,
    /// Array of elements with inner type
    Array(&'static str),  // Element schema name, or "dynamic" for unknown
    /// Reference to another object schema
    Object(&'static str),
    /// Optional wrapper
    Optional(&'static str),  // Inner schema name
    /// Unknown/dynamic type
    Dynamic,
}

/// A field in an object schema.
#[derive(Debug, Clone)]
pub struct NestedFieldSchema {
    pub name: &'static str,
    pub field_type: NestedFieldType,
}

/// Schema for an object type (Ship, Player, Weapon, etc.)
#[derive(Debug, Clone)]
pub struct ObjectSchema {
    pub name: &'static str,
    pub fields: &'static [NestedFieldSchema],
}

impl ObjectSchema {
    /// Find a field by name.
    pub fn get_field(&self, name: &str) -> Option<&NestedFieldSchema> {
        self.fields.iter().find(|f| f.name == name)
    }
}

// ============================================================================
// Access Path Types
// ============================================================================

/// A segment in an access path (e.g., ship["cargo"][0]["quantity"])
#[derive(Debug, Clone)]
pub enum AccessSegment {
    /// Field access with literal string key
    Field(String),
    /// Array index with literal integer
    Index(i64),
    /// Dynamic key/index that can't be validated statically
    Dynamic,
}

/// An access path representing a chain of accesses.
#[derive(Debug, Clone)]
pub struct AccessPath {
    /// The root variable name
    pub root: String,
    /// The chain of access segments
    pub segments: Vec<AccessSegment>,
    /// Source position
    pub position: Option<Position>,
}

// ============================================================================
// Static Schema Definitions
// ============================================================================

pub static SHIP_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Ship",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "owner_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "ship_class", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "position", field_type: NestedFieldType::Object("Position") },
        NestedFieldSchema { name: "hull", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "shields", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "status", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "is_player", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "faction_tag", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "is_hostile", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "sector_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "ammunition", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "fuel", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "morale", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "experience", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "faction_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "can_attack", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "can_move", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "attack", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "defense", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "speed", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "sensor_range", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "combat_stance", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "locked_target", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "cargo", field_type: NestedFieldType::Array("CargoItem") },
        NestedFieldSchema { name: "cargo_capacity", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "cargo_used", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "upgrades", field_type: NestedFieldType::Array("InstalledUpgrade") },
        // Runtime-specific fields (not in DTO, but available to scripts)
        NestedFieldSchema { name: "weapons", field_type: NestedFieldType::Array("Weapon") },
        NestedFieldSchema { name: "engagement_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "combat_target", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "docked_at", field_type: NestedFieldType::Optional("Uuid") },
    ],
};

pub static PLAYER_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Player",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "username", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "reputation", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "fame", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "faction_tag", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "squadron_tag", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "is_online", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "is_admin", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "credits", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "game_mode", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "owned_ships", field_type: NestedFieldType::Array("Uuid") },
        NestedFieldSchema { name: "active_ship_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "sector_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "faction_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "squadron_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "missions_completed", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "missions_failed", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "is_disgraced", field_type: NestedFieldType::Bool },
        // Runtime-specific fields (not in DTO, but available to scripts)
        NestedFieldSchema { name: "squadron_role", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "active_missions", field_type: NestedFieldType::Array("Uuid") },
    ],
};

pub static CARGO_ITEM_SCHEMA: ObjectSchema = ObjectSchema {
    name: "CargoItem",
    fields: &[
        NestedFieldSchema { name: "type", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "cargo_type", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "quantity", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "price", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "purchase_price", field_type: NestedFieldType::Int },
    ],
};

pub static INSTALLED_UPGRADE_SCHEMA: ObjectSchema = ObjectSchema {
    name: "InstalledUpgrade",
    fields: &[
        NestedFieldSchema { name: "upgrade_id", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "slot", field_type: NestedFieldType::String },
    ],
};

pub static POSITION_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Position",
    fields: &[
        NestedFieldSchema { name: "x", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "y", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "z", field_type: NestedFieldType::Float },
    ],
};

pub static WEAPON_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Weapon",
    fields: &[
        NestedFieldSchema { name: "name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "damage", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "range", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "accuracy", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "cooldown", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "cooldown_remaining", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "ammo_cost", field_type: NestedFieldType::Float },
    ],
};

pub static LOCATION_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Location",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "location_type", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "position", field_type: NestedFieldType::Object("Position") },
        NestedFieldSchema { name: "faction_tag", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "faction_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "services", field_type: NestedFieldType::Array("String") },
        NestedFieldSchema { name: "sector_id", field_type: NestedFieldType::Uuid },
    ],
};

pub static SECTOR_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Sector",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "danger_level", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "controlling_faction", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "locations", field_type: NestedFieldType::Array("Location") },
        NestedFieldSchema { name: "adjacent_sectors", field_type: NestedFieldType::Array("AdjacentSector") },
    ],
};

pub static ADJACENT_SECTOR_SCHEMA: ObjectSchema = ObjectSchema {
    name: "AdjacentSector",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "danger_level", field_type: NestedFieldType::String },
    ],
};

pub static COMBAT_ENGAGEMENT_SCHEMA: ObjectSchema = ObjectSchema {
    name: "CombatEngagement",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "attacker_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "defender_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "sector_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "round", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "started_at", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "status", field_type: NestedFieldType::String },
    ],
};

pub static SQUADRON_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Squadron",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "tag", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "motto", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "leader_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "leader_name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "member_count", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "reputation_bonus", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "fame_bonus", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "is_at_war", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "wargames_enabled", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "privateering_enabled", field_type: NestedFieldType::Bool },
    ],
};

pub static SQUADRON_INVITE_SCHEMA: ObjectSchema = ObjectSchema {
    name: "SquadronInvite",
    fields: &[
        NestedFieldSchema { name: "invite_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "squadron_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "inviter_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "target_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "squadron_name", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "squadron_tag", field_type: NestedFieldType::String },
    ],
};

pub static MISSION_SCHEMA: ObjectSchema = ObjectSchema {
    name: "Mission",
    fields: &[
        NestedFieldSchema { name: "id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "title", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "description", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "mission_type", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "status", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "priority", field_type: NestedFieldType::String },
        NestedFieldSchema { name: "reputation_reward", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "fame_reward", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "is_high_profile", field_type: NestedFieldType::Bool },
        NestedFieldSchema { name: "progress", field_type: NestedFieldType::Float },
        NestedFieldSchema { name: "sector_id", field_type: NestedFieldType::Uuid },
        NestedFieldSchema { name: "player_id", field_type: NestedFieldType::Uuid },
        // Additional fields used by scripts
        NestedFieldSchema { name: "min_reputation", field_type: NestedFieldType::Int },
        NestedFieldSchema { name: "accepted_by", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "pending_choice", field_type: NestedFieldType::Optional("String") },
        NestedFieldSchema { name: "choices", field_type: NestedFieldType::Array("Choice") },
        NestedFieldSchema { name: "target_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "target_sector_id", field_type: NestedFieldType::Optional("Uuid") },
        NestedFieldSchema { name: "credit_reward", field_type: NestedFieldType::Int },
    ],
};

/// Get schema by name.
pub fn get_object_schema(name: &str) -> Option<&'static ObjectSchema> {
    match name {
        "Ship" => Some(&SHIP_SCHEMA),
        "Player" => Some(&PLAYER_SCHEMA),
        "CargoItem" => Some(&CARGO_ITEM_SCHEMA),
        "InstalledUpgrade" => Some(&INSTALLED_UPGRADE_SCHEMA),
        "Position" => Some(&POSITION_SCHEMA),
        "Weapon" => Some(&WEAPON_SCHEMA),
        "Location" => Some(&LOCATION_SCHEMA),
        "Sector" => Some(&SECTOR_SCHEMA),
        "AdjacentSector" => Some(&ADJACENT_SECTOR_SCHEMA),
        "CombatEngagement" => Some(&COMBAT_ENGAGEMENT_SCHEMA),
        "Squadron" => Some(&SQUADRON_SCHEMA),
        "SquadronInvite" => Some(&SQUADRON_INVITE_SCHEMA),
        "Mission" => Some(&MISSION_SCHEMA),
        _ => None,
    }
}

/// Maps API function return types to schema names.
pub fn api_return_schema(fn_name: &str) -> Option<&'static str> {
    match fn_name {
        "query_ship" | "get_ship" => Some("Ship"),
        "query_player" | "get_player" => Some("Player"),
        "query_location" => Some("Location"),
        "query_sector" => Some("Sector"),
        "query_combat_engagement" => Some("CombatEngagement"),
        "query_squadron" => Some("Squadron"),
        "get_squadron_invite" => Some("SquadronInvite"),
        "query_mission" => Some("Mission"),
        _ => None,
    }
}
