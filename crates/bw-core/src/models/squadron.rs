//! Squadron model - Player alliances
//!
//! Squadrons are player organizations that can:
//! - Control patrol sectors
//! - Build stations and NPC ships (at Reputation cost)
//! - Engage in war games (PvP training) or privateering (actual PvP)
//! - Provide collective bonuses to members

use chrono::{DateTime, Utc};
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hash_helpers::hash_f32;

/// A player squadron (alliance).
#[derive(Debug, Clone, Serialize, Deserialize, Derivative)]
#[derivative(Hash)]
pub struct Squadron {
    /// Unique identifier
    pub id: Uuid,

    /// Squadron name
    pub name: String,

    /// Short tag shown in player names [TAG]
    pub tag: String,

    /// Squadron motto
    pub motto: Option<String>,

    /// Squadron description
    pub description: String,

    /// When the squadron was founded
    pub founded_at: DateTime<Utc>,

    /// Leader player ID
    pub leader_id: Uuid,

    /// Officer player IDs (have management permissions)
    pub officers: Vec<Uuid>,

    /// All member player IDs (includes leader and officers)
    pub members: Vec<Uuid>,

    /// Patrol sectors the squadron controls
    pub patrol_sectors: Vec<Uuid>,

    /// Stations built by the squadron
    pub owned_stations: Vec<Uuid>,

    /// NPC ships owned by the squadron
    pub owned_ships: Vec<Uuid>,

    /// Squadron treasury (accumulated from member contributions)
    pub treasury: i64,

    /// Collective reputation bonus for members
    #[derivative(Hash(hash_with = "hash_f32"))]
    pub reputation_bonus: f32,

    /// Collective fame bonus for members
    #[derivative(Hash(hash_with = "hash_f32"))]
    pub fame_bonus: f32,

    /// Allied squadrons
    pub allied_squadrons: Vec<Uuid>,

    /// Hostile squadrons (active conflict)
    pub hostile_squadrons: Vec<Uuid>,

    /// Whether PvP war games are enabled
    pub wargames_enabled: bool,

    /// Whether privateering (real PvP) is enabled
    pub privateering_enabled: bool,

    /// Squadron settings
    pub settings: SquadronSettings,

    /// Statistics
    pub stats: SquadronStats,
}

impl Squadron {
    /// Create a new squadron.
    pub fn new(name: String, tag: String, leader_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            tag,
            motto: None,
            description: String::new(),
            founded_at: Utc::now(),
            leader_id,
            officers: Vec::new(),
            members: vec![leader_id],
            patrol_sectors: Vec::new(),
            owned_stations: Vec::new(),
            owned_ships: Vec::new(),
            treasury: 0,
            reputation_bonus: 0.0,
            fame_bonus: 0.0,
            allied_squadrons: Vec::new(),
            hostile_squadrons: Vec::new(),
            wargames_enabled: false,
            privateering_enabled: false,
            settings: SquadronSettings::default(),
            stats: SquadronStats::default(),
        }
    }

    /// Get total member count.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Check if a player is the leader.
    pub fn is_leader(&self, player_id: Uuid) -> bool {
        self.leader_id == player_id
    }

    /// Check if a player is an officer.
    pub fn is_officer(&self, player_id: Uuid) -> bool {
        self.officers.contains(&player_id)
    }

    /// Check if a player is a member.
    pub fn is_member(&self, player_id: Uuid) -> bool {
        self.members.contains(&player_id)
    }

    /// Check if a player can manage squadron (leader or officer).
    pub fn can_manage(&self, player_id: Uuid) -> bool {
        self.is_leader(player_id) || self.is_officer(player_id)
    }

    /// Add a member.
    pub fn add_member(&mut self, player_id: Uuid) {
        if !self.members.contains(&player_id) {
            self.members.push(player_id);
            self.recalculate_bonuses();
        }
    }

    /// Remove a member.
    pub fn remove_member(&mut self, player_id: Uuid) {
        if player_id == self.leader_id {
            return; // Can't remove leader
        }
        self.members.retain(|&id| id != player_id);
        self.officers.retain(|&id| id != player_id);
        self.recalculate_bonuses();
    }

    /// Promote a member to officer.
    pub fn promote_to_officer(&mut self, player_id: Uuid) {
        if self.is_member(player_id) && !self.is_officer(player_id) && !self.is_leader(player_id) {
            self.officers.push(player_id);
        }
    }

    /// Demote an officer to member.
    pub fn demote_officer(&mut self, player_id: Uuid) {
        self.officers.retain(|&id| id != player_id);
    }

    /// Transfer leadership.
    pub fn transfer_leadership(&mut self, new_leader_id: Uuid) {
        if self.is_member(new_leader_id) {
            // Old leader becomes officer
            let old_leader = self.leader_id;
            if !self.officers.contains(&old_leader) {
                self.officers.push(old_leader);
            }

            // New leader
            self.leader_id = new_leader_id;
            self.officers.retain(|&id| id != new_leader_id);
        }
    }

    /// Recalculate collective bonuses based on member count and achievements.
    fn recalculate_bonuses(&mut self) {
        let member_count = self.member_count() as f32;

        // More members = smaller bonus per person (diminishing returns)
        self.reputation_bonus = (member_count.sqrt() * 2.0).min(10.0);
        self.fame_bonus = (member_count.sqrt() * 1.5).min(8.0);
    }

    /// Check if squadron is at war with another.
    pub fn is_hostile_with(&self, other_squadron_id: Uuid) -> bool {
        self.hostile_squadrons.contains(&other_squadron_id)
    }

    /// Check if squadron is allied with another.
    pub fn is_allied_with(&self, other_squadron_id: Uuid) -> bool {
        self.allied_squadrons.contains(&other_squadron_id)
    }

    /// Declare war on another squadron.
    pub fn declare_war(&mut self, other_squadron_id: Uuid) {
        if !self.hostile_squadrons.contains(&other_squadron_id) {
            self.hostile_squadrons.push(other_squadron_id);
        }
        self.allied_squadrons.retain(|&id| id != other_squadron_id);
    }

    /// Make peace with another squadron.
    pub fn make_peace(&mut self, other_squadron_id: Uuid) {
        self.hostile_squadrons.retain(|&id| id != other_squadron_id);
    }

    /// Form alliance with another squadron.
    pub fn form_alliance(&mut self, other_squadron_id: Uuid) {
        if !self.allied_squadrons.contains(&other_squadron_id) {
            self.allied_squadrons.push(other_squadron_id);
        }
        self.hostile_squadrons.retain(|&id| id != other_squadron_id);
    }

    /// Break alliance with another squadron.
    pub fn break_alliance(&mut self, other_squadron_id: Uuid) {
        self.allied_squadrons.retain(|&id| id != other_squadron_id);
    }

    /// Calculate the Reputation cost to build something.
    pub fn build_cost(&self, building_type: SquadronBuildingType) -> i32 {
        match building_type {
            SquadronBuildingType::Outpost => 100,
            SquadronBuildingType::Station => 500,
            SquadronBuildingType::Shipyard => 1000,
            SquadronBuildingType::PatrolShip => 50,
            SquadronBuildingType::DefenseShip => 150,
        }
    }
}

/// Squadron settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Derivative)]
#[derivative(Hash)]
pub struct SquadronSettings {
    /// Whether to accept join requests automatically
    pub auto_accept: bool,
    /// Minimum reputation to join
    pub min_reputation_to_join: i32,
    /// Whether members can invite
    pub members_can_invite: bool,
    /// Tax rate on member mission rewards (0.0 - 0.5)
    #[derivative(Hash(hash_with = "hash_f32"))]
    pub tax_rate: f32,
}

/// Squadron statistics.
#[derive(Debug, Clone, Default, Hash, Serialize, Deserialize)]
pub struct SquadronStats {
    /// Total missions completed by all members
    pub total_missions: i32,
    /// Total reputation earned by all members
    pub total_reputation_earned: i64,
    /// Sectors controlled (current)
    pub sectors_controlled: i32,
    /// Wars won
    pub wars_won: i32,
    /// Wars lost
    pub wars_lost: i32,
}

/// Types of things a squadron can build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SquadronBuildingType {
    /// Small presence in a sector
    Outpost,
    /// Full station with services
    Station,
    /// Can build ships
    Shipyard,
    /// NPC patrol ship
    PatrolShip,
    /// NPC defense ship
    DefenseShip,
}
