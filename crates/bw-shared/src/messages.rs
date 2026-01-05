//! WebSocket message types
//!
//! All messages between server and client.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dto::*;

/// Messages sent from client to server.
///
/// Infrastructure messages are handled directly by Rust.
/// All game logic is handled via ScriptAction, routed to Rhai scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // === Infrastructure (Rust-handled) ===

    /// Authenticate with token
    Authenticate { token: String },

    /// Request to join a sector
    JoinSector { sector_id: Uuid },

    /// Leave current sector
    LeaveSector,

    /// Chat/comms
    SendChat { message: String, channel: ChatChannel },

    /// Heartbeat
    Ping { timestamp: u64 },

    /// Subscribe to performance metrics updates (debug panel)
    SubscribeMetrics,

    /// Unsubscribe from performance metrics updates
    UnsubscribeMetrics,

    /// Admin/GM message (requires admin privileges)
    Admin(AdminClientMessage),

    // === Game Logic (Script-handled) ===

    /// All game actions are routed to Rhai scripts via this variant.
    ///
    /// Common actions:
    /// - Movement: "move_to_position", "move_to_location", "move_to_sector", "stop_movement"
    /// - Station: "dock", "undock", "use_service"
    /// - Missions: "accept_mission", "abandon_mission", "mission_choice"
    /// - Combat: "engage_target", "disengage_combat", "fire_weapon"
    /// - Squadron: "create_squadron", "invite_to_squadron", "accept_squadron_invite", etc.
    /// - Social: "hail"
    ScriptAction {
        /// Action identifier (e.g., "dock", "accept_mission", "engage_target")
        action: String,
        /// JSON parameters for the action
        params: serde_json::Value,
    },
}

impl ClientMessage {
    /// Create a ScriptAction message.
    pub fn action(name: impl Into<String>, params: serde_json::Value) -> Self {
        Self::ScriptAction {
            action: name.into(),
            params,
        }
    }

    // === Movement Actions ===

    pub fn move_to_position(x: f64, y: f64, z: f64) -> Self {
        Self::action("move_to_position", serde_json::json!({ "x": x, "y": y, "z": z }))
    }

    pub fn move_to_location(location_id: Uuid) -> Self {
        Self::action("move_to_location", serde_json::json!({ "location_id": location_id }))
    }

    pub fn move_to_sector(sector_id: Uuid) -> Self {
        Self::action("move_to_sector", serde_json::json!({ "sector_id": sector_id }))
    }

    pub fn stop_movement() -> Self {
        Self::action("stop_movement", serde_json::json!({}))
    }

    // === Station Actions ===

    pub fn dock(station_id: Uuid) -> Self {
        Self::action("dock", serde_json::json!({ "station_id": station_id }))
    }

    pub fn undock() -> Self {
        Self::action("undock", serde_json::json!({}))
    }

    pub fn use_service(service: impl Into<String>) -> Self {
        Self::action("use_service", serde_json::json!({ "service": service.into() }))
    }

    // === Mission Actions ===

    pub fn accept_mission(mission_id: Uuid) -> Self {
        Self::action("accept_mission", serde_json::json!({ "mission_id": mission_id }))
    }

    pub fn abandon_mission(mission_id: Uuid) -> Self {
        Self::action("abandon_mission", serde_json::json!({ "mission_id": mission_id }))
    }

    pub fn mission_choice(mission_id: Uuid, choice_id: impl Into<String>) -> Self {
        Self::action("mission_choice", serde_json::json!({
            "mission_id": mission_id,
            "choice_id": choice_id.into()
        }))
    }

    // === Combat Actions ===

    pub fn engage_target(target_id: Uuid) -> Self {
        Self::action("engage_target", serde_json::json!({ "target_id": target_id }))
    }

    pub fn disengage_combat() -> Self {
        Self::action("disengage_combat", serde_json::json!({}))
    }

    pub fn fire_weapon(weapon_index: usize, target_id: Uuid) -> Self {
        Self::action("fire_weapon", serde_json::json!({
            "weapon_index": weapon_index,
            "target_id": target_id
        }))
    }

    // === Squadron Actions ===

    pub fn create_squadron(name: impl Into<String>, tag: impl Into<String>) -> Self {
        Self::action("create_squadron", serde_json::json!({
            "name": name.into(),
            "tag": tag.into()
        }))
    }

    pub fn invite_to_squadron(player_id: Uuid) -> Self {
        Self::action("invite_to_squadron", serde_json::json!({ "player_id": player_id }))
    }

    pub fn accept_squadron_invite(invite_id: Uuid) -> Self {
        Self::action("accept_squadron_invite", serde_json::json!({ "invite_id": invite_id }))
    }

    pub fn decline_squadron_invite(invite_id: Uuid) -> Self {
        Self::action("decline_squadron_invite", serde_json::json!({ "invite_id": invite_id }))
    }

    pub fn leave_squadron() -> Self {
        Self::action("leave_squadron", serde_json::json!({}))
    }

    pub fn squadron_action(action_type: impl Into<String>, params: serde_json::Value) -> Self {
        let mut p = params;
        if let Some(obj) = p.as_object_mut() {
            obj.insert("action_type".to_string(), serde_json::Value::String(action_type.into()));
        }
        Self::action("squadron_action", p)
    }

    // === Alliance Actions ===

    pub fn accept_alliance(proposal_id: Uuid) -> Self {
        Self::action("accept_alliance", serde_json::json!({ "proposal_id": proposal_id }))
    }

    pub fn decline_alliance(proposal_id: Uuid) -> Self {
        Self::action("decline_alliance", serde_json::json!({ "proposal_id": proposal_id }))
    }

    // === Social Actions ===

    pub fn hail(target_id: Uuid) -> Self {
        Self::action("hail", serde_json::json!({ "target_id": target_id }))
    }
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Authentication result
    AuthResult { success: bool, player_id: Option<Uuid>, error: Option<String> },

    /// Initial state when joining sector
    InitialState {
        player: PlayerDto,
        ship: ShipDto,
        sector: SectorDto,
        ships: Vec<ShipDto>,
        missions: Vec<MissionDto>,
    },

    /// Delta state update (incremental)
    StateUpdate {
        tick: u64,
        ship_updates: Vec<ShipUpdateDto>,
        ship_spawns: Vec<ShipDto>,
        ship_despawns: Vec<Uuid>,
        mission_updates: Vec<MissionUpdateDto>,
        events: Vec<GameEventDto>,
    },

    /// Player resource update
    ResourceUpdate {
        reputation: i32,
        fame: i32,
        ammunition: f32,
        fuel: f32,
        morale: f32,
        experience: i32,
    },

    /// Mission choice presented
    MissionChoice {
        mission_id: Uuid,
        description: String,
        choices: Vec<ChoiceDto>,
    },

    /// Mission result
    MissionResult {
        mission_id: Uuid,
        success: bool,
        reputation_change: i32,
        fame_change: i32,
        narrative: String,
    },

    /// Combat update
    CombatUpdate {
        engagement_id: Uuid,
        round: u32,
        events: Vec<CombatEventDto>,
        is_resolved: bool,
        winner: Option<String>,
    },

    /// Chat message received
    ChatMessage {
        sender_id: Uuid,
        sender_name: String,
        message: String,
        channel: ChatChannel,
        timestamp: u64,
    },

    /// Error message
    Error { code: String, message: String },

    /// Heartbeat response
    Pong { timestamp: u64, server_tick: u64 },

    /// Player kicked (disconnected)
    Kicked { reason: String },

    /// Squadron update (creation, joining, info)
    SquadronUpdate {
        squadron: Option<SquadronDto>,
        message: String,
    },

    /// Squadron invitation received
    SquadronInvite {
        invite_id: Uuid,
        squadron_id: Uuid,
        squadron_name: String,
        squadron_tag: String,
        inviter_name: String,
    },

    /// Alliance proposal received (for squadron leaders)
    AllianceProposal {
        proposal_id: Uuid,
        from_squadron_id: Uuid,
        from_squadron_name: String,
        from_squadron_tag: String,
    },

    /// Quick "Yo" style hail received from another player
    HailReceived {
        from_id: Uuid,
        from_name: String,
    },

    /// Performance metrics update (for debug panel subscribers)
    TickMetrics(TickMetricsDto),

    /// Full metrics history (sent on subscription)
    TickMetricsHistory(TickMetricsHistoryDto),

    /// Admin/GM response message
    Admin(AdminServerMessage),

    /// Generic notification from scripts
    Notification {
        message: String,
        notification_type: String,
    },

    /// Choice dialog from scripts (non-mission)
    ChoiceRequired {
        choice_id: String,
        description: String,
        choices: Vec<ChoiceDto>,
    },

    /// Result from a script action
    ScriptActionResult {
        /// The action that was executed
        action: String,
        /// Whether the action succeeded
        success: bool,
        /// Error message if failed
        error: Option<String>,
        /// Optional response data
        data: Option<serde_json::Value>,
    },
}

/// Chat channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatChannel {
    /// Local sector chat
    Sector,
    /// Squadron chat
    Squadron,
    /// Direct message (requires target_id in context)
    Direct,
    /// System-wide (admin only)
    System,
}

/// Squadron actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SquadronAction {
    PromoteToOfficer { player_id: Uuid },
    DemoteOfficer { player_id: Uuid },
    KickMember { player_id: Uuid },
    TransferLeadership { player_id: Uuid },
    SetMotto { motto: String },
    EnableWargames { enabled: bool },
    EnablePrivateering { enabled: bool },
    DeclareWar { squadron_id: Uuid },
    MakePeace { squadron_id: Uuid },
    FormAlliance { squadron_id: Uuid },
}

/// Serialize a message to MessagePack bytes.
pub fn serialize_message<T: Serialize>(msg: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec(msg)
}

/// Deserialize a message from MessagePack bytes.
pub fn deserialize_message<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(bytes)
}

// =============================================================================
// Admin/GM Messages
// =============================================================================

/// Admin messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminClientMessage {
    /// List all script files in the scripts directory
    ListScripts,

    /// Read a script file's content
    ReadScript { path: String },

    /// Write/update a script file (staged change)
    WriteScript { path: String, content: String },

    /// Get current simulation config
    GetSimConfig,

    /// Query entities with optional filters
    QueryEntities {
        entity_type: EntityType,
        filters: Vec<EntityFilter>,
        limit: usize,
        offset: usize,
    },

    /// Get full details of a specific entity
    GetEntity { entity_type: EntityType, id: Uuid },

    /// Query all sectors with summary info
    QuerySectors,

    /// Preview staged changes (validate and compute diffs)
    PreviewStaged { changes: Vec<StagedChange> },

    /// Commit staged changes atomically
    CommitStaged { changes: Vec<StagedChange> },

    // === Playtest Management ===

    /// Create a new playtest forked from live state
    CreatePlaytest {
        name: String,
        fork_config: ForkConfigDto,
    },

    /// List all active playtests
    ListPlaytests,

    /// Get details of a specific playtest
    GetPlaytest { playtest_id: Uuid },

    /// Join a playtest (as GM or invited player)
    JoinPlaytest { playtest_id: Uuid },

    /// Leave current playtest (return to live)
    LeavePlaytest,

    /// Invite a player to the playtest
    InviteToPlaytest {
        playtest_id: Uuid,
        player_id: Uuid,
    },

    /// Remove a player from playtest
    KickFromPlaytest {
        playtest_id: Uuid,
        player_id: Uuid,
    },

    /// Pause/unpause playtest simulation
    SetPlaytestPaused {
        playtest_id: Uuid,
        paused: bool,
    },

    /// Set playtest time scale (0.5 = half speed, 2.0 = double speed)
    SetPlaytestTimeScale {
        playtest_id: Uuid,
        scale: f32,
    },

    /// Discard and destroy a playtest
    DestroyPlaytest { playtest_id: Uuid },

    /// Promote playtest changes to live state
    PromotePlaytest {
        playtest_id: Uuid,
        promote_config: PromoteConfigDto,
    },

    /// Preview what would be promoted
    PreviewPromote {
        playtest_id: Uuid,
        promote_config: PromoteConfigDto,
    },

    // === Script Debugging ===

    /// Subscribe to script errors (real-time streaming)
    SubscribeScriptErrors,

    /// Unsubscribe from script errors
    UnsubscribeScriptErrors,

    /// Reload a specific script file
    ReloadScript { path: String },

    /// Reload archetype definitions (ships, weapons)
    ReloadDefinitions,

    /// Get recent script errors
    GetRecentScriptErrors { limit: usize },
}

/// Admin messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminServerMessage {
    /// Script file listing
    ScriptList { files: Vec<ScriptFileInfo> },

    /// Script file content
    ScriptContent {
        path: String,
        content: String,
        last_modified: u64,
    },

    /// Current simulation config as JSON
    SimConfigData { config: serde_json::Value },

    /// Entity query results
    EntityList {
        entity_type: EntityType,
        entities: Vec<EntitySummary>,
        total_count: usize,
    },

    /// Full entity details
    EntityDetails {
        entity_type: EntityType,
        id: Uuid,
        data: serde_json::Value,
    },

    /// Sector list with admin-level details
    SectorList { sectors: Vec<SectorSummaryAdmin> },

    /// Preview of staged changes with validation
    StagedPreview {
        changes: Vec<ChangePreview>,
        errors: Vec<ValidationError>,
    },

    /// Result of committing staged changes
    CommitResult {
        success: bool,
        applied_count: usize,
        errors: Vec<String>,
    },

    /// Admin error response
    AdminError { code: String, message: String },

    // === Playtest Responses ===

    /// Playtest created successfully
    PlaytestCreated {
        playtest_id: Uuid,
        name: String,
    },

    /// List of active playtests
    PlaytestList {
        playtests: Vec<PlaytestSummaryDto>,
    },

    /// Full playtest details
    PlaytestDetails {
        playtest: PlaytestDetailDto,
    },

    /// Joined playtest successfully
    PlaytestJoined {
        playtest_id: Uuid,
        /// Initial state within playtest
        ship_id: Uuid,
        sector_id: Uuid,
    },

    /// Left playtest, returned to live
    PlaytestLeft,

    /// Player invited to playtest
    PlaytestInviteSent {
        playtest_id: Uuid,
        player_id: Uuid,
    },

    /// Player kicked from playtest
    PlaytestPlayerKicked {
        playtest_id: Uuid,
        player_id: Uuid,
    },

    /// Playtest state changed (pause, time scale, etc.)
    PlaytestStateChanged {
        playtest_id: Uuid,
        paused: bool,
        time_scale: f32,
        tick: u64,
    },

    /// Playtest destroyed
    PlaytestDestroyed { playtest_id: Uuid },

    /// Preview of promotion results
    PromotePreview {
        ships_to_update: usize,
        players_to_update: usize,
        new_entities: usize,
        deletions: usize,
    },

    /// Promotion completed
    PromoteResult {
        success: bool,
        applied_count: usize,
        errors: Vec<String>,
    },

    // === Script Debugging ===

    /// Single script error (streamed to subscribers)
    ScriptError {
        script: String,
        function: String,
        message: String,
        line: usize,
        column: usize,
        tick: u64,
    },

    /// Batch of script errors (response to GetRecentScriptErrors)
    ScriptErrors { errors: Vec<ScriptErrorDto> },

    /// Script hot-reload result
    ScriptReloaded {
        path: String,
        success: bool,
        error: Option<String>,
        warnings: Vec<String>,
    },

    /// Archetype definition reload result
    DefinitionsReloaded {
        file: String,
        ships_loaded: usize,
        weapons_loaded: usize,
        errors: Vec<String>,
    },

    /// Subscribed to script errors
    SubscribedToScriptErrors,

    /// Unsubscribed from script errors
    UnsubscribedFromScriptErrors,
}

/// Entity types for admin queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Ship,
    Player,
    Mission,
    Station,
    Sector,
}

/// Filter conditions for entity queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityFilter {
    /// Filter by name containing string (case-insensitive)
    NameContains(String),
    /// Filter by sector ID
    InSector(Uuid),
    /// Filter by faction ID
    ByFaction(Uuid),
    /// Filter by status string
    ByStatus(String),
    /// Filter NPC vs player ships
    IsNpc(bool),
    /// Filter hostile ships
    IsHostile(bool),
}

/// A staged change pending commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StagedChange {
    /// Update a Rhai script file
    ScriptUpdate { path: String, content: String },

    /// Update simulation config section
    SimConfigUpdate {
        section: SimConfigSection,
        value: serde_json::Value,
    },

    /// Update entity properties (partial update)
    EntityUpdate {
        entity_type: EntityType,
        id: Uuid,
        changes: serde_json::Value,
    },

    /// Spawn a new entity
    EntitySpawn {
        entity_type: EntityType,
        config: serde_json::Value,
    },

    /// Delete an entity
    EntityDelete { entity_type: EntityType, id: Uuid },

    /// Update sector properties
    SectorUpdate { id: Uuid, changes: serde_json::Value },

    /// Add a location to a sector
    LocationAdd {
        sector_id: Uuid,
        location: serde_json::Value,
    },

    /// Remove a location from a sector
    LocationRemove { sector_id: Uuid, location_id: Uuid },
}

/// Simulation config sections that can be updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimConfigSection {
    /// NPC spawning settings
    NpcSpawning,
    /// Combat settings
    Combat,
    /// Danger level multipliers
    DangerMultipliers,
    /// Replace entire config
    Full,
}

/// Script file information for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptFileInfo {
    /// Relative path from scripts directory
    pub path: String,
    /// File name only
    pub name: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// Last modified timestamp (Unix epoch seconds)
    pub last_modified: u64,
}

/// Summary of an entity for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    pub id: Uuid,
    pub name: String,
    pub entity_type: EntityType,
    pub sector_id: Option<Uuid>,
    pub status: String,
    /// Type-specific summary fields as JSON
    pub extra: serde_json::Value,
}

/// Admin sector summary with counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorSummaryAdmin {
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
    pub ship_count: usize,
    pub player_count: usize,
    pub npc_count: usize,
    pub mission_count: usize,
    pub location_count: usize,
}

/// Preview of a single staged change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePreview {
    /// Type of change (e.g., "ScriptUpdate", "EntityUpdate")
    pub change_type: String,
    /// Human-readable target description
    pub target: String,
    /// Previous value (if applicable)
    pub before: Option<serde_json::Value>,
    /// New value
    pub after: serde_json::Value,
}

/// Validation error for a staged change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Index of the change in the staged list
    pub change_index: usize,
    /// Field that failed validation
    pub field: String,
    /// Error message
    pub message: String,
}

/// Script error information for GM tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptErrorDto {
    /// Script file path
    pub script: String,
    /// Function that generated the error
    pub function: String,
    /// Error message
    pub message: String,
    /// Line number (0 if unknown)
    pub line: usize,
    /// Column number (0 if unknown)
    pub column: usize,
    /// Server tick when error occurred
    pub tick: u64,
    /// Unix timestamp in milliseconds
    pub timestamp_ms: u64,
}
