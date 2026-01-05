//! Playtest configuration types
//!
//! Configuration for forking state and promoting changes.

use uuid::Uuid;

/// Error types for playtest operations.
#[derive(Debug, Clone)]
pub enum PlaytestError {
    /// Playtest not found
    NotFound,
    /// Player not authorized for this operation
    Unauthorized,
    /// Maximum playtest limit reached
    LimitReached,
    /// Maximum participant limit reached for a single playtest
    ParticipantLimitReached,
    /// Player is already in a playtest
    AlreadyInPlaytest,
    /// Player is not in any playtest
    NotInPlaytest,
    /// Invalid configuration
    InvalidConfig(String),
    /// Fork failed
    ForkFailed(String),
    /// Promote failed
    PromoteFailed(String),
}

impl std::fmt::Display for PlaytestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Playtest not found"),
            Self::Unauthorized => write!(f, "Not authorized for this operation"),
            Self::LimitReached => write!(f, "Maximum playtest limit reached"),
            Self::ParticipantLimitReached => write!(f, "Maximum participant limit reached"),
            Self::AlreadyInPlaytest => write!(f, "Player is already in a playtest"),
            Self::NotInPlaytest => write!(f, "Player is not in any playtest"),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::ForkFailed(msg) => write!(f, "Fork failed: {}", msg),
            Self::PromoteFailed(msg) => write!(f, "Promote failed: {}", msg),
        }
    }
}

impl std::error::Error for PlaytestError {}

/// Configuration for what to include when forking state.
#[derive(Debug, Clone)]
pub struct ForkConfig {
    /// Sectors to include (empty = all sectors)
    pub sectors: Vec<Uuid>,

    /// Whether to include all ships in selected sectors
    pub include_ships: bool,

    /// Specific ships to include (overrides include_ships if non-empty)
    pub specific_ships: Vec<Uuid>,

    /// Whether to include missions
    pub include_missions: bool,

    /// Whether to include NPC ships
    pub include_npcs: bool,

    /// Whether to include other players' ships (except invited players)
    pub include_other_players: bool,
}

impl Default for ForkConfig {
    fn default() -> Self {
        Self {
            sectors: vec![], // All sectors
            include_ships: true,
            specific_ships: vec![],
            include_missions: true,
            include_npcs: true,
            include_other_players: false, // Only invited players
        }
    }
}

impl ForkConfig {
    /// Fork a single sector with all entities.
    pub fn single_sector(sector_id: Uuid) -> Self {
        Self {
            sectors: vec![sector_id],
            include_ships: true,
            specific_ships: vec![],
            include_missions: true,
            include_npcs: true,
            include_other_players: false,
        }
    }

    /// Fork specific sectors with all entities.
    pub fn sectors(sector_ids: Vec<Uuid>) -> Self {
        Self {
            sectors: sector_ids,
            include_ships: true,
            specific_ships: vec![],
            include_missions: true,
            include_npcs: true,
            include_other_players: false,
        }
    }

    /// Minimal fork - just sectors and specific ships.
    pub fn minimal(sector_ids: Vec<Uuid>, ship_ids: Vec<Uuid>) -> Self {
        Self {
            sectors: sector_ids,
            include_ships: false,
            specific_ships: ship_ids,
            include_missions: false,
            include_npcs: false,
            include_other_players: false,
        }
    }
}

/// Configuration for promoting changes from playtest to live.
#[derive(Debug, Clone, Default)]
pub struct PromoteConfig {
    /// Ship modifications to promote (by ship_id)
    pub ships: Vec<Uuid>,

    /// Player modifications to promote (by player_id)
    pub players: Vec<Uuid>,

    /// Whether to spawn new entities created in playtest
    pub spawn_new_entities: bool,

    /// Whether to apply entity deletions
    pub apply_deletions: bool,
}

impl PromoteConfig {
    /// Promote all changes.
    pub fn all() -> Self {
        Self {
            ships: vec![],
            players: vec![],
            spawn_new_entities: true,
            apply_deletions: true,
        }
    }

    /// Promote only specific ships.
    pub fn ships_only(ship_ids: Vec<Uuid>) -> Self {
        Self {
            ships: ship_ids,
            players: vec![],
            spawn_new_entities: false,
            apply_deletions: false,
        }
    }
}

/// Result of a promote operation.
#[derive(Debug, Clone, Default)]
pub struct PromoteResult {
    /// Number of ships updated
    pub ships_updated: usize,
    /// Number of players updated
    pub players_updated: usize,
    /// Number of new entities spawned
    pub entities_spawned: usize,
    /// Number of entities deleted
    pub entities_deleted: usize,
    /// Any errors that occurred (non-fatal)
    pub errors: Vec<String>,
}

impl PromoteResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.ships_updated + self.players_updated + self.entities_spawned + self.entities_deleted
    }
}
