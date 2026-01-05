//! Game events
//!
//! Events that occur during gameplay.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A game event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: GameEventType,
    pub sector_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub data: serde_json::Value,
}

impl GameEvent {
    pub fn new(event_type: GameEventType, sector_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            sector_id,
            actor_id: None,
            target_id: None,
            data: serde_json::Value::Null,
        }
    }

    pub fn with_actor(mut self, actor_id: Uuid) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    pub fn with_target(mut self, target_id: Uuid) -> Self {
        self.target_id = Some(target_id);
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

/// Types of game events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameEventType {
    // Player events
    PlayerJoined,
    PlayerLeft,
    PlayerDocked,
    PlayerUndocked,

    // Ship events
    ShipSpawned,
    ShipDestroyed,
    ShipDamaged,
    ShipRepaired,
    ShipRefueled,
    ShipRearmed,

    // Combat events
    CombatStarted,
    CombatEnded,
    AttackHit,
    AttackMissed,

    // Mission events
    MissionSpawned,
    MissionAccepted,
    MissionCompleted,
    MissionFailed,
    MissionExpired,
    MissionChoiceMade,

    // Reputation events
    ReputationGained,
    ReputationLost,
    FameGained,
    FameLost,
    PlayerDisgraced,

    // Squadron events
    SquadronCreated,
    SquadronDissolved,
    SquadronMemberJoined,
    SquadronMemberLeft,
    SquadronWarDeclared,
    SquadronPeaceDeclared,

    // Sector events
    SectorControlChanged,
    StationBuilt,
    StationDestroyed,

    // Special events
    SeraIncursion,
    DroneSwarmDetected,
    AsteroidAlert,
    DistressSignalReceived,
}

impl GameEventType {
    /// Whether this event should be broadcast to all players in sector.
    pub fn is_broadcast(&self) -> bool {
        matches!(
            self,
            Self::CombatStarted
                | Self::CombatEnded
                | Self::MissionSpawned
                | Self::SectorControlChanged
                | Self::SeraIncursion
                | Self::DroneSwarmDetected
                | Self::AsteroidAlert
                | Self::DistressSignalReceived
        )
    }

    /// Whether this event should be logged to the database.
    pub fn is_persistent(&self) -> bool {
        matches!(
            self,
            Self::PlayerJoined
                | Self::PlayerLeft
                | Self::ShipDestroyed
                | Self::MissionCompleted
                | Self::MissionFailed
                | Self::PlayerDisgraced
                | Self::SquadronCreated
                | Self::SquadronDissolved
                | Self::SectorControlChanged
        )
    }
}
