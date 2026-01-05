//! Mission model - Events and objectives
//!
//! Missions are the core gameplay loop. They can be:
//! - Random crises (pirates, asteroids, terrorists, accidents)
//! - Command assigned (sector defense, fleet operations)
//! - Player initiated (investigation of suspicious activity)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Position;

/// A mission in the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    /// Unique identifier
    pub id: Uuid,

    /// Mission type
    pub mission_type: MissionType,

    /// Display title
    pub title: String,

    /// Description/briefing
    pub description: String,

    /// Rhai script path for this mission's logic
    pub script_path: String,

    /// Current state in the mission state machine
    pub current_state: String,

    /// Arbitrary mission data (passed to/from scripts)
    pub data: serde_json::Value,

    /// Sector this mission takes place in
    pub sector_id: Uuid,

    /// Target location within sector (if applicable)
    pub target_position: Option<Position>,

    /// Target entity (ship, station, etc.)
    pub target_id: Option<Uuid>,

    /// Who this mission is assigned to
    pub assigned_to: Option<Uuid>,

    /// Who can accept this mission
    pub availability: MissionAvailability,

    /// When the mission was created
    pub created_at: DateTime<Utc>,

    /// When the mission expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,

    /// Base reputation reward for success
    pub reputation_reward: i32,

    /// Base reputation penalty for failure
    pub reputation_penalty: i32,

    /// Fame change on success
    pub fame_reward: i32,

    /// Credit reward
    pub credits_reward: i64,

    /// Current status
    pub status: MissionStatus,

    /// Progress (0.0 to 1.0)
    pub progress: f32,

    /// Is this a high-profile mission (more fame impact)
    pub is_high_profile: bool,

    /// Priority level (affects order in mission list)
    pub priority: MissionPriority,
}

impl Mission {
    /// Create a new mission.
    pub fn new(
        mission_type: MissionType,
        title: String,
        sector_id: Uuid,
        script_path: String,
    ) -> Self {
        let (rep_reward, rep_penalty, fame_reward) = mission_type.default_rewards();

        Self {
            id: Uuid::new_v4(),
            mission_type,
            title,
            description: String::new(),
            script_path,
            current_state: "start".to_string(),
            data: serde_json::Value::Object(serde_json::Map::new()),
            sector_id,
            target_position: None,
            target_id: None,
            assigned_to: None,
            availability: MissionAvailability::SectorWide,
            created_at: Utc::now(),
            expires_at: None,
            reputation_reward: rep_reward,
            reputation_penalty: rep_penalty,
            fame_reward,
            credits_reward: 0,
            status: MissionStatus::Available,
            progress: 0.0,
            is_high_profile: false,
            priority: MissionPriority::Normal,
        }
    }

    /// Check if mission has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// Check if a player can accept this mission.
    pub fn can_accept(&self, player_id: Uuid, squadron_id: Option<Uuid>) -> bool {
        if !matches!(self.status, MissionStatus::Available) {
            return false;
        }

        match &self.availability {
            MissionAvailability::SectorWide => true,
            MissionAvailability::Assigned(id) => *id == player_id,
            MissionAvailability::SquadronOnly(sid) => squadron_id.as_ref() == Some(sid),
            MissionAvailability::RangeRestricted(_) => true, // Range check done elsewhere
        }
    }

    /// Accept the mission.
    pub fn accept(&mut self, player_id: Uuid) {
        self.assigned_to = Some(player_id);
        self.status = MissionStatus::InProgress;
    }

    /// Complete the mission.
    pub fn complete(&mut self, success: bool) {
        self.status = MissionStatus::Completed { success };
        self.progress = 1.0;
    }

    /// Abandon the mission.
    pub fn abandon(&mut self) {
        self.status = MissionStatus::Abandoned;
    }

    /// Get the display state for the mission (for UI).
    pub fn display_state(&self) -> &str {
        match &self.status {
            MissionStatus::Available => "Available",
            MissionStatus::InProgress => &self.current_state,
            MissionStatus::Completed { success: true } => "Completed",
            MissionStatus::Completed { success: false } => "Failed",
            MissionStatus::Expired => "Expired",
            MissionStatus::Abandoned => "Abandoned",
        }
    }
}

/// Type of mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionType {
    // Random crises (from design doc)
    /// Pirates attacking civilian vessels
    PirateIntercept,
    /// Asteroid on collision course
    AsteroidThreat,
    /// Bomb-equipped ship heading to populated area
    TerroristPlot,
    /// Equipment malfunction, workers in danger
    IndustrialAccident,
    /// Unknown signal, could be anything
    DistressSignal,
    /// Illegal goods transport
    Smugglers,

    // Command assigned
    /// Defend sector from enemy incursion
    SectorDefense,
    /// Participate in large fleet operation
    FleetOperation,
    /// Protect VIP or cargo
    Escort,

    // Player initiated (investigation)
    /// Player chose to investigate something
    Investigation,

    // Squadron missions
    /// PvP training with allied squadron
    SquadronWargames,
    /// PvP with hostile squadron
    Privateering,

    // Special
    /// Sera encounter
    SeraContact,
    /// Drone Intelligence encounter
    DroneEncounter,
}

impl MissionType {
    /// Get default rewards for this mission type.
    pub fn default_rewards(&self) -> (i32, i32, i32) {
        // (reputation_reward, reputation_penalty, fame_reward)
        match self {
            Self::PirateIntercept => (20, -10, 5),
            Self::AsteroidThreat => (40, -30, 15),
            Self::TerroristPlot => (80, -50, 30),
            Self::IndustrialAccident => (15, -5, 5),
            Self::DistressSignal => (25, -15, 10),
            Self::Smugglers => (30, -10, 5),
            Self::SectorDefense => (50, -30, 20),
            Self::FleetOperation => (100, -40, 40),
            Self::Escort => (35, -20, 10),
            Self::Investigation => (10, -5, 0), // Risky - reward depends on outcome
            Self::SquadronWargames => (5, 0, 2),
            Self::Privateering => (40, -20, 15),
            Self::SeraContact => (100, -50, 50),
            Self::DroneEncounter => (60, -30, 25),
        }
    }

    /// Get script path for this mission type.
    pub fn default_script_path(&self) -> &'static str {
        match self {
            Self::PirateIntercept => "missions/random/pirate_attack.rhai",
            Self::AsteroidThreat => "missions/random/asteroid_threat.rhai",
            Self::TerroristPlot => "missions/random/terrorist_plot.rhai",
            Self::IndustrialAccident => "missions/random/industrial_accident.rhai",
            Self::DistressSignal => "missions/random/distress_signal.rhai",
            Self::Smugglers => "missions/random/smugglers.rhai",
            Self::SectorDefense => "missions/command/sector_defense.rhai",
            Self::FleetOperation => "missions/command/fleet_operation.rhai",
            Self::Escort => "missions/command/escort.rhai",
            Self::Investigation => "missions/player/investigation.rhai",
            Self::SquadronWargames => "missions/squadron/wargames.rhai",
            Self::Privateering => "missions/squadron/privateering.rhai",
            Self::SeraContact => "missions/special/sera_contact.rhai",
            Self::DroneEncounter => "missions/special/drone_encounter.rhai",
        }
    }
}

/// Who can accept a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionAvailability {
    /// Any ship in the sector
    SectorWide,
    /// Only ships within range
    RangeRestricted(f64),
    /// Assigned to specific player
    Assigned(Uuid),
    /// Squadron members only
    SquadronOnly(Uuid),
}

/// Current status of a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionStatus {
    /// Can be accepted
    Available,
    /// Currently in progress
    InProgress,
    /// Finished
    Completed { success: bool },
    /// Time ran out
    Expired,
    /// Player gave up
    Abandoned,
}

/// Mission priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MissionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// A choice presented to the player during a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionChoice {
    /// Choice point ID
    pub id: String,
    /// Description of the situation
    pub description: String,
    /// Available choices
    pub choices: Vec<Choice>,
}

/// A single choice option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Choice ID
    pub id: String,
    /// Display text
    pub text: String,
    /// Requirements to select this choice
    pub requirements: Vec<ChoiceRequirement>,
    /// Whether player meets requirements
    pub is_available: bool,
}

/// Requirement to select a choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChoiceRequirement {
    /// Minimum crew experience
    MinExperience(i32),
    /// Minimum player fame
    MinFame(i32),
    /// Minimum player reputation
    MinReputation(i32),
    /// Specific item in cargo
    HasItem(String),
    /// Squadron members present
    SquadronPresent,
    /// Minimum ship hull integrity
    MinHull(f32),
    /// Minimum ammunition
    MinAmmo(f32),
}

impl ChoiceRequirement {
    /// Check if a requirement is met (simplified - full check in scripting layer)
    pub fn description(&self) -> String {
        match self {
            Self::MinExperience(n) => format!("Requires {} crew experience", n),
            Self::MinFame(n) => format!("Requires {} fame", n),
            Self::MinReputation(n) => format!("Requires {} reputation", n),
            Self::HasItem(item) => format!("Requires {}", item),
            Self::SquadronPresent => "Requires squadron members nearby".to_string(),
            Self::MinHull(n) => format!("Requires {}% hull integrity", n),
            Self::MinAmmo(n) => format!("Requires {}% ammunition", n),
        }
    }
}
