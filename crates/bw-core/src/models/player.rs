//! Player model - The artilect officers
//!
//! Players are artilects (artificial intelligences) serving in the Space Guard.
//! Their ship IS them - they don't pilot the ship, they ARE the ship.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::PlayerResources;

/// A player in the game.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct Player {
    /// Unique identifier
    pub id: Uuid,

    /// Display name / callsign
    pub username: String,

    /// Account creation time
    pub created_at: DateTime<Utc>,

    /// Player resources (reputation, fame)
    pub resources: PlayerResources,

    /// Currently active ship
    pub active_ship_id: Uuid,

    /// Assigned patrol sector
    pub patrol_sector_id: Uuid,

    /// Primary faction affiliation
    pub faction_id: Uuid,

    /// Squadron membership (alliance)
    pub squadron_id: Option<Uuid>,

    /// Rank within squadron
    pub squadron_rank: Option<SquadronRank>,

    /// Standing with each faction
    pub faction_standings: Vec<FactionStanding>,

    /// Whether player is currently online
    pub is_online: bool,

    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,

    /// Remaining offline attack protection (from design doc)
    /// Players can only be attacked X times while offline to prevent ganking
    pub offline_attacks_remaining: i32,

    /// Total missions completed
    pub missions_completed: i32,

    /// Total missions failed
    pub missions_failed: i32,

    /// Career statistics
    pub stats: PlayerStats,

    /// Credits (spendable currency for purchases)
    pub credits: i64,

    /// Game mode (Standard or Hardcore)
    pub game_mode: GameMode,

    /// Ships owned by this player (for hardcore backup ships)
    pub owned_ships: Vec<Uuid>,
}

impl Player {
    /// Create a new player with starting values.
    pub fn new(
        username: String,
        ship_id: Uuid,
        sector_id: Uuid,
        faction_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            created_at: Utc::now(),
            resources: PlayerResources::new(100), // Starting reputation
            active_ship_id: ship_id,
            patrol_sector_id: sector_id,
            faction_id,
            squadron_id: None,
            squadron_rank: None,
            faction_standings: Vec::new(),
            is_online: false,
            last_seen: Utc::now(),
            offline_attacks_remaining: 5, // Can be attacked 5 times offline
            missions_completed: 0,
            missions_failed: 0,
            stats: PlayerStats::default(),
            credits: 0,
            game_mode: GameMode::Standard,
            owned_ships: vec![ship_id], // Starting ship is owned
        }
    }

    /// Get standing with a specific faction.
    pub fn get_faction_standing(&self, faction_id: Uuid) -> Option<&FactionStanding> {
        self.faction_standings.iter().find(|s| s.faction_id == faction_id)
    }

    /// Get mutable standing with a specific faction.
    pub fn get_faction_standing_mut(&mut self, faction_id: Uuid) -> Option<&mut FactionStanding> {
        self.faction_standings.iter_mut().find(|s| s.faction_id == faction_id)
    }

    /// Modify standing with a faction.
    pub fn modify_faction_standing(&mut self, faction_id: Uuid, change: i32) {
        if let Some(standing) = self.get_faction_standing_mut(faction_id) {
            standing.standing = (standing.standing + change).clamp(-100, 100);
            standing.update_rank();
        } else {
            let mut new_standing = FactionStanding::new(faction_id);
            new_standing.standing = change.clamp(-100, 100);
            new_standing.update_rank();
            self.faction_standings.push(new_standing);
        }
    }

    /// Check if player is an officer or leader of their squadron.
    pub fn is_squadron_officer(&self) -> bool {
        matches!(
            self.squadron_rank,
            Some(SquadronRank::Officer) | Some(SquadronRank::Leader)
        )
    }

    /// Record mission completion.
    pub fn record_mission_success(&mut self) {
        self.missions_completed += 1;
        self.stats.total_missions += 1;
    }

    /// Record mission failure.
    pub fn record_mission_failure(&mut self) {
        self.missions_failed += 1;
        self.stats.total_missions += 1;
    }

    /// Check if player is disgraced (reputation = 0).
    pub fn is_disgraced(&self) -> bool {
        self.resources.is_disgraced()
    }

    /// Update online status.
    pub fn set_online(&mut self, online: bool) {
        self.is_online = online;
        if online {
            self.offline_attacks_remaining = 5; // Reset protection on login
        }
        self.last_seen = Utc::now();
    }

    /// Add credits to player account.
    pub fn add_credits(&mut self, amount: i64) {
        self.credits = self.credits.saturating_add(amount);
    }

    /// Spend credits if player has enough. Returns true if successful.
    pub fn spend_credits(&mut self, amount: i64) -> bool {
        if self.credits >= amount {
            self.credits -= amount;
            true
        } else {
            false
        }
    }

    /// Check if player is in hardcore mode.
    pub fn is_hardcore(&self) -> bool {
        self.game_mode == GameMode::Hardcore
    }

    /// Add a ship to owned ships list.
    pub fn add_owned_ship(&mut self, ship_id: Uuid) {
        if !self.owned_ships.contains(&ship_id) {
            self.owned_ships.push(ship_id);
        }
    }

    /// Remove a ship from owned ships (when destroyed in hardcore).
    pub fn remove_owned_ship(&mut self, ship_id: Uuid) {
        self.owned_ships.retain(|&id| id != ship_id);
    }
}

/// Standing with a specific faction.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct FactionStanding {
    pub faction_id: Uuid,
    /// Standing value from -100 (hostile) to 100 (allied)
    pub standing: i32,
    /// Derived rank based on standing
    pub rank: FactionRank,
}

impl FactionStanding {
    pub fn new(faction_id: Uuid) -> Self {
        Self {
            faction_id,
            standing: 0,
            rank: FactionRank::Neutral,
        }
    }

    /// Update rank based on current standing.
    pub fn update_rank(&mut self) {
        self.rank = match self.standing {
            -100..=-61 => FactionRank::Hostile,
            -60..=-21 => FactionRank::Unfriendly,
            -20..=20 => FactionRank::Neutral,
            21..=60 => FactionRank::Friendly,
            61..=85 => FactionRank::Trusted,
            86..=100 => FactionRank::Allied,
            _ => FactionRank::Neutral,
        };
    }

    /// Check if this faction is hostile.
    pub fn is_hostile(&self) -> bool {
        matches!(self.rank, FactionRank::Hostile)
    }

    /// Check if this faction is friendly or better.
    pub fn is_friendly(&self) -> bool {
        matches!(
            self.rank,
            FactionRank::Friendly | FactionRank::Trusted | FactionRank::Allied
        )
    }
}

/// Rank with a faction based on standing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactionRank {
    /// Kill on sight
    Hostile,
    /// Will not trade or cooperate
    Unfriendly,
    /// Default state
    Neutral,
    /// Will trade and provide basic services
    Friendly,
    /// Access to advanced services
    Trusted,
    /// Full cooperation, mutual defense
    Allied,
}

/// Rank within a squadron.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SquadronRank {
    /// Regular member
    Member,
    /// Can manage members, has some permissions
    Officer,
    /// Full control over squadron
    Leader,
}

/// Game mode determines death penalties and progression rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GameMode {
    /// Standard mode - soft death penalties, ship recovered
    #[default]
    Standard,
    /// Hardcore mode - permadeath, ship lost on destruction
    Hardcore,
}

/// Player career statistics.
#[derive(Debug, Clone, Default, Hash, Serialize, Deserialize)]
pub struct PlayerStats {
    /// Total missions attempted
    pub total_missions: i32,
    /// Ships destroyed in combat
    pub ships_destroyed: i32,
    /// Times player ship was destroyed
    pub times_destroyed: i32,
    /// Total reputation earned (lifetime)
    pub total_reputation_earned: i64,
    /// Highest fame achieved
    pub peak_fame: i32,
    /// Sectors patrolled
    pub sectors_patrolled: i32,
    /// Asteroids intercepted
    pub asteroids_intercepted: i32,
    /// Terrorists stopped
    pub terrorists_stopped: i32,
    /// Civilians saved
    pub civilians_saved: i32,
}
