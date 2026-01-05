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

    /// Heartbeat
    Ping { timestamp: u64 },
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
