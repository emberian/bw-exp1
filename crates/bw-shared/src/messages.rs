//! WebSocket message types
//!
//! All messages between server and client.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dto::*;

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Authenticate with token
    Authenticate { token: String },

    /// Request to join a sector
    JoinSector { sector_id: Uuid },

    /// Leave current sector
    LeaveSector,

    /// Movement commands
    MoveToPosition { x: f64, y: f64, z: f64 },
    MoveToLocation { location_id: Uuid },
    MoveToSector { sector_id: Uuid },
    StopMovement,

    /// Station commands
    DockAtStation { station_id: Uuid },
    Undock,
    UseService { service: String },

    /// Mission commands
    AcceptMission { mission_id: Uuid },
    AbandonMission { mission_id: Uuid },
    MakeMissionChoice { mission_id: Uuid, choice_id: String },

    /// Combat commands
    EngageTarget { target_id: Uuid },
    DisengageCombat,
    FireWeapon { weapon_index: usize, target_id: Uuid },

    /// Chat/comms
    SendChat { message: String, channel: ChatChannel },

    /// Squadron commands
    CreateSquadron { name: String, tag: String },
    InviteToSquadron { player_id: Uuid },
    AcceptSquadronInvite { invite_id: Uuid },
    DeclineSquadronInvite { invite_id: Uuid },
    LeaveSquadron,
    SquadronAction { action: SquadronAction },

    /// Alliance commands
    AcceptAlliance { proposal_id: Uuid },
    DeclineAlliance { proposal_id: Uuid },

    /// Quick "Yo" style hail to another ship
    Hail { target_id: Uuid },

    /// Heartbeat
    Ping { timestamp: u64 },

    /// Subscribe to performance metrics updates (debug panel)
    SubscribeMetrics,

    /// Unsubscribe from performance metrics updates
    UnsubscribeMetrics,

    /// Admin/GM message (requires admin privileges)
    Admin(AdminClientMessage),
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
