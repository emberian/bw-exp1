//! API function signatures and context keys.
//!
//! Defines the signatures of all API functions available to Rhai scripts,
//! including their argument counts and return types.

use rhai::Position;

// ============================================================================
// API Function Signatures
// ============================================================================

/// Return type of an API function for null safety analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// Returns nothing (modify_*, send_*, emit_*)
    Unit,
    /// May return () on failure (query_* functions)
    Nullable,
    /// Always returns bool
    Bool,
    /// Always returns a Map
    Map,
    /// Always returns an Array
    Array,
    /// Always returns a number (Int or Float)
    Number,
    /// Always returns a String
    String,
    /// Unknown/varied return type
    Dynamic,
}

/// Specification of an API function available to scripts.
#[derive(Debug, Clone)]
pub struct ApiFn {
    pub name: &'static str,
    pub arg_count: usize,
    pub returns: ReturnType,
}

impl ApiFn {
    pub const fn new(name: &'static str, arg_count: usize, returns: ReturnType) -> Self {
        Self { name, arg_count, returns }
    }
}

/// All registered API functions with their signatures.
/// Used for validating function calls in scripts.
pub static API_FUNCTIONS: &[ApiFn] = &[
    // === State API - Queries ===
    ApiFn::new("query_ship", 1, ReturnType::Map),           // Throws on not found
    ApiFn::new("query_player", 1, ReturnType::Map),         // Throws on not found
    ApiFn::new("query_sector", 1, ReturnType::Nullable),    // Returns () if not found
    ApiFn::new("query_ships_in_sector", 1, ReturnType::Array),
    ApiFn::new("query_ships_in_range", 5, ReturnType::Array),
    ApiFn::new("query_ships_near", 4, ReturnType::Array),
    ApiFn::new("get_context_sector", 0, ReturnType::Nullable),

    // === State API - Modifications ===
    ApiFn::new("modify_ship", 2, ReturnType::Unit),         // Throws on error
    ApiFn::new("modify_player", 2, ReturnType::Bool),
    ApiFn::new("damage_ship", 2, ReturnType::Bool),
    ApiFn::new("move_ship_to", 4, ReturnType::Bool),
    ApiFn::new("spawn_npc", 1, ReturnType::String),
    ApiFn::new("destroy_ship", 1, ReturnType::Bool),

    // === State API - Events ===
    ApiFn::new("emit_event", 2, ReturnType::Bool),
    ApiFn::new("emit_event_with_target", 3, ReturnType::Bool),

    // === State API - Economy ===
    ApiFn::new("add_credits", 2, ReturnType::Bool),
    ApiFn::new("spend_credits", 2, ReturnType::Bool),
    ApiFn::new("get_credits", 1, ReturnType::Number),

    // === State API - Cargo ===
    ApiFn::new("add_cargo", 4, ReturnType::Bool),
    ApiFn::new("remove_cargo", 3, ReturnType::Bool),
    ApiFn::new("get_cargo", 1, ReturnType::Array),
    ApiFn::new("get_cargo_capacity", 1, ReturnType::Number),
    ApiFn::new("get_cargo_used", 1, ReturnType::Number),

    // === State API - Combat ===
    ApiFn::new("set_combat_stance", 2, ReturnType::Bool),
    ApiFn::new("get_combat_stance", 1, ReturnType::String),
    ApiFn::new("lock_target", 2, ReturnType::Bool),
    ApiFn::new("clear_target", 1, ReturnType::Bool),
    ApiFn::new("get_locked_target", 1, ReturnType::String),

    // === State API - Upgrades ===
    ApiFn::new("install_upgrade", 3, ReturnType::Bool),
    ApiFn::new("remove_upgrade", 2, ReturnType::Bool),
    ApiFn::new("get_upgrades", 1, ReturnType::Array),
    ApiFn::new("has_upgrade", 2, ReturnType::Bool),

    // === State API - Watches ===
    ApiFn::new("watch_property", 5, ReturnType::String),
    ApiFn::new("watch_property_changed", 3, ReturnType::String),
    ApiFn::new("watch_once", 5, ReturnType::String),
    ApiFn::new("unwatch", 1, ReturnType::Bool),
    ApiFn::new("unwatch_all_for_entity", 1, ReturnType::Unit),
    ApiFn::new("pause_watch", 1, ReturnType::Bool),
    ApiFn::new("resume_watch", 1, ReturnType::Bool),
    ApiFn::new("list_watches_for_entity", 1, ReturnType::Array),
    ApiFn::new("get_watch_count", 0, ReturnType::Number),

    // === State API - Utility ===
    ApiFn::new("distance", 6, ReturnType::Number),
    ApiFn::new("distance_2d", 4, ReturnType::Number),
    ApiFn::new("distance_to_ship", 4, ReturnType::Number),
    ApiFn::new("is_hostile_ship_class", 1, ReturnType::Bool),

    // === Message API ===
    ApiFn::new("send_notification", 2, ReturnType::Bool),
    ApiFn::new("send_notification_type", 3, ReturnType::Bool),
    ApiFn::new("send_warning", 2, ReturnType::Bool),
    ApiFn::new("send_error", 2, ReturnType::Bool),
    ApiFn::new("send_success", 2, ReturnType::Bool),
    ApiFn::new("send_choice", 4, ReturnType::Bool),
    ApiFn::new("broadcast_to_sector", 2, ReturnType::Bool),
    ApiFn::new("broadcast_warning_to_sector", 2, ReturnType::Bool),

    // === Action API ===
    ApiFn::new("register_action", 2, ReturnType::Bool),
    ApiFn::new("register_action_with_requirements", 3, ReturnType::Bool),
    ApiFn::new("unregister_action", 1, ReturnType::Bool),
    ApiFn::new("is_action_registered", 1, ReturnType::Bool),
    ApiFn::new("list_actions", 0, ReturnType::Array),
    ApiFn::new("set_action_enabled", 2, ReturnType::Bool),

    // === Combat API ===
    ApiFn::new("calculate_damage", 3, ReturnType::Number),
    ApiFn::new("calculate_hit_chance", 3, ReturnType::Number),
    ApiFn::new("roll_hit", 1, ReturnType::Bool),
    ApiFn::new("roll_critical", 2, ReturnType::Bool),
    ApiFn::new("apply_critical", 2, ReturnType::Number),
    ApiFn::new("create_attack_result", 3, ReturnType::Map),
    ApiFn::new("resolve_attack", 7, ReturnType::Map),

    // === Data API ===
    ApiFn::new("get_data", 2, ReturnType::Nullable),
    ApiFn::new("get_ship_def", 1, ReturnType::Nullable),
    ApiFn::new("get_upgrade_def", 1, ReturnType::Nullable),
    ApiFn::new("get_cargo_def", 1, ReturnType::Nullable),
    ApiFn::new("get_stance_def", 1, ReturnType::Nullable),
    ApiFn::new("list_data_keys", 1, ReturnType::Array),
    ApiFn::new("has_data", 2, ReturnType::Bool),

    // === Data API - Archetypes ===
    ApiFn::new("get_ship", 1, ReturnType::Nullable),
    ApiFn::new("all_ships", 0, ReturnType::Array),
    ApiFn::new("player_ships", 0, ReturnType::Array),
    ApiFn::new("get_weapon", 1, ReturnType::Nullable),
    ApiFn::new("all_weapons", 0, ReturnType::Array),
    ApiFn::new("get_effect", 1, ReturnType::Nullable),
    ApiFn::new("all_effects", 0, ReturnType::Array),
    ApiFn::new("get_ability", 1, ReturnType::Nullable),
    ApiFn::new("all_abilities", 0, ReturnType::Array),
    ApiFn::new("active_abilities", 0, ReturnType::Array),
    ApiFn::new("passive_abilities", 0, ReturnType::Array),
    ApiFn::new("all_cargo_types", 0, ReturnType::Array),
    ApiFn::new("legal_cargo_types", 0, ReturnType::Array),
    ApiFn::new("get_faction", 1, ReturnType::Nullable),
    ApiFn::new("all_factions", 0, ReturnType::Array),
    ApiFn::new("playable_factions", 0, ReturnType::Array),
    ApiFn::new("hostile_factions", 0, ReturnType::Array),
    ApiFn::new("get_faction_relation", 2, ReturnType::Number),

    // === Logging ===
    ApiFn::new("log_info", 1, ReturnType::Unit),
    ApiFn::new("log_warning", 1, ReturnType::Unit),
    ApiFn::new("log_error", 1, ReturnType::Unit),

    // === Random ===
    ApiFn::new("random_float", 0, ReturnType::Number),
    ApiFn::new("random_int", 2, ReturnType::Number),
    ApiFn::new("current_tick", 0, ReturnType::Number),

    // === Combat Engagement API ===
    ApiFn::new("create_combat_engagement", 3, ReturnType::String), // (attacker_id, defender_id, sector_id) -> engagement_id
    ApiFn::new("query_combat_engagement", 1, ReturnType::Nullable), // (engagement_id) -> engagement or ()
    ApiFn::new("end_combat_engagement", 2, ReturnType::Bool),       // (engagement_id, reason) -> success
    ApiFn::new("set_weapon_cooldown", 3, ReturnType::Unit),         // (ship_id, weapon_index, ticks)
    ApiFn::new("distance_between", 2, ReturnType::Number),          // (pos1, pos2) -> distance

    // === Location API ===
    ApiFn::new("query_location", 1, ReturnType::Nullable), // (location_id) -> location or ()

    // === Squadron API ===
    ApiFn::new("query_squadron", 1, ReturnType::Nullable),          // (squadron_id) -> squadron or ()
    ApiFn::new("modify_squadron", 2, ReturnType::Bool),             // (squadron_id, changes) -> success
    ApiFn::new("create_squadron", 3, ReturnType::String),           // (name, tag, leader_id) -> squadron_id
    ApiFn::new("disband_squadron", 1, ReturnType::Bool),            // (squadron_id) -> success
    ApiFn::new("add_to_squadron", 3, ReturnType::Bool),             // (player_id, squadron_id, role) -> success
    ApiFn::new("remove_from_squadron", 2, ReturnType::Bool),        // (player_id, squadron_id) -> success
    ApiFn::new("squadron_tag_exists", 1, ReturnType::Bool),         // (tag) -> exists
    ApiFn::new("update_squadron_leader", 2, ReturnType::Bool),      // (squadron_id, new_leader_id) -> success
    ApiFn::new("broadcast_to_squadron", 2, ReturnType::Bool),       // (squadron_id, message) -> success

    // === Squadron Invites API ===
    ApiFn::new("create_squadron_invite", 3, ReturnType::String),    // (squadron_id, inviter_id, target_id) -> invite_id
    ApiFn::new("get_squadron_invite", 1, ReturnType::Nullable),     // (invite_id) -> invite or ()
    ApiFn::new("delete_squadron_invite", 1, ReturnType::Bool),      // (invite_id) -> success
    ApiFn::new("send_squadron_invite", 2, ReturnType::Bool),        // (target_id, invite_data) -> success

    // === Squadron Relations API ===
    ApiFn::new("are_squadrons_at_war", 2, ReturnType::Bool),        // (squadron_id1, squadron_id2) -> at_war
    ApiFn::new("are_squadrons_allied", 2, ReturnType::Bool),        // (squadron_id1, squadron_id2) -> allied
    ApiFn::new("declare_war", 2, ReturnType::Bool),                 // (squadron_id1, squadron_id2) -> success
    ApiFn::new("create_peace_proposal", 2, ReturnType::String),     // (from_squadron, to_squadron) -> proposal_id
    ApiFn::new("create_alliance_proposal", 2, ReturnType::String),  // (from_squadron, to_squadron) -> proposal_id
    ApiFn::new("get_alliance_proposal", 1, ReturnType::Nullable),   // (proposal_id) -> proposal or ()
    ApiFn::new("delete_alliance_proposal", 1, ReturnType::Bool),    // (proposal_id) -> success
    ApiFn::new("send_alliance_proposal", 2, ReturnType::Bool),      // (target_squadron_id, proposal_data) -> success
    ApiFn::new("create_alliance", 2, ReturnType::Bool),             // (squadron_id1, squadron_id2) -> success

    // === Mission API ===
    ApiFn::new("query_mission", 1, ReturnType::Nullable),           // (mission_id) -> mission or ()
    ApiFn::new("add_player_mission", 2, ReturnType::Bool),          // (player_id, mission_id) -> success
    ApiFn::new("remove_player_mission", 2, ReturnType::Bool),       // (player_id, mission_id) -> success
    ApiFn::new("modify_mission", 2, ReturnType::Bool),              // (mission_id, changes) -> success
    ApiFn::new("record_mission_choice", 2, ReturnType::Bool),       // (mission_id, choice_id) -> success

    // === Communication API ===
    ApiFn::new("send_hail", 2, ReturnType::Bool),                   // (target_id, hail_data) -> success
];

/// Lookup API function by name.
pub fn get_api_function(name: &str) -> Option<&'static ApiFn> {
    API_FUNCTIONS.iter().find(|f| f.name == name)
}

/// Known keys for action context.
pub static CTX_KEYS: &[&str] = &[
    "player_id",
    "ship_id",
    "sector_id",
    "action",
    "params",
];

/// Known event types (from bw_core::events::GameEventType).
pub static KNOWN_EVENT_TYPES: &[&str] = &[
    // Player events
    "PlayerJoined",
    "PlayerLeft",
    "PlayerDocked",
    "PlayerUndocked",
    // Ship events
    "ShipSpawned",
    "ShipDestroyed",
    "ShipDamaged",
    "ShipRepaired",
    "ShipRefueled",
    "ShipRearmed",
    // Combat events
    "CombatStarted",
    "CombatEnded",
    "AttackHit",
    "AttackMissed",
    // Mission events
    "MissionSpawned",
    "MissionAccepted",
    "MissionCompleted",
    "MissionFailed",
    "MissionExpired",
    "MissionChoiceMade",
    // Reputation events
    "ReputationGained",
    "ReputationLost",
    "FameGained",
    "FameLost",
    "PlayerDisgraced",
    // Squadron events
    "SquadronCreated",
    "SquadronDissolved",
    "SquadronMemberJoined",
    "SquadronMemberLeft",
    "SquadronWarDeclared",
    "SquadronPeaceDeclared",
    // Sector events
    "SectorControlChanged",
    "StationBuilt",
    "StationDestroyed",
    // Special events
    "SeraIncursion",
    "DroneSwarmDetected",
    "AsteroidAlert",
    "DistressSignalReceived",
];

/// Check if an event type is known.
pub fn is_known_event_type(event_type: &str) -> bool {
    KNOWN_EVENT_TYPES.contains(&event_type)
}

/// Information about an event subscription in a script.
#[derive(Debug, Clone)]
pub struct EventSubscriptionInfo {
    /// The event type being subscribed to
    pub event_type: String,
    /// The handler function name
    pub handler_fn: String,
    /// Position in source
    pub position: Option<Position>,
}

/// Information about a variable in scope.
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// Name of the variable
    pub name: String,
    /// Where the variable was defined
    pub defined_at: Option<Position>,
    /// Whether the variable has been used
    pub used: bool,
    /// Whether this variable may be null (from query_* etc)
    pub nullable: bool,
    /// Whether we've confirmed it's not null (via if check)
    pub null_checked: bool,
}
