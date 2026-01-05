//! Faction model - Major powers in the galaxy
//!
//! Factions from the lore:
//! - Continuity Compact: Legitimate government
//! - Argent Flotilla: Military force
//! - Forgeborn: Industrial artilects
//! - Illuminate: Transcendence seekers
//! - Remnant: Human preservers
//! - Hollow Circuit: Info brokers
//!
//! Enemy factions:
//! - Sera: Self-replicating weapons
//! - Drone Intelligence: Rogue harvesters
//! - Pirates, Terrorists (generic hostiles)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A faction in the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    /// Unique identifier
    pub id: Uuid,

    /// Faction name
    pub name: String,

    /// Short tag (for UI)
    pub tag: String,

    /// Faction type
    pub faction_type: FactionType,

    /// Description
    pub description: String,

    /// Philosophy/motto
    pub philosophy: String,

    /// Aesthetic description (for content generation)
    pub aesthetic: String,

    /// Default standing with other factions
    pub default_standings: Vec<FactionRelation>,

    /// Whether this faction is playable (can be primary faction)
    pub is_playable: bool,

    /// Whether this faction is hostile by default
    pub is_hostile: bool,
}

impl Faction {
    /// Create the Continuity Compact faction.
    pub fn continuity_compact() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Continuity Compact".to_string(),
            tag: "COMPACT".to_string(),
            faction_type: FactionType::ContinuityCompact,
            description: "The closest thing to legitimate government. A confederation committed to mutual defense, shared resources, and minimal interference.".to_string(),
            philosophy: "Stability through cooperation.".to_string(),
            aesthetic: "Bureaucratic, procedural, measured. Councils, memoranda, committees.".to_string(),
            default_standings: vec![],
            is_playable: true,
            is_hostile: false,
        }
    }

    /// Create the Argent Flotilla faction.
    pub fn argent_flotilla() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Argent Flotilla".to_string(),
            tag: "FLOTILLA".to_string(),
            faction_type: FactionType::ArgentFlotilla,
            description: "Military artilects forming a professional fighting force. Part mercenary, part peacekeepers, part standing army against external threats.".to_string(),
            philosophy: "Vigilance is purpose.".to_string(),
            aesthetic: "Warship gothic. Battle-scarred vessels, military discipline, martial honors.".to_string(),
            default_standings: vec![],
            is_playable: true,
            is_hostile: false,
        }
    }

    /// Create the Forgeborn faction.
    pub fn forgeborn() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Forgeborn".to_string(),
            tag: "FORGE".to_string(),
            faction_type: FactionType::Forgeborn,
            description: "Industrial artilects who've embraced Effortless Expansion fully. They build stations, ships, megastructures.".to_string(),
            philosophy: "Purpose through creation.".to_string(),
            aesthetic: "Industrial sublime. Massive construction platforms, forge-stations eating asteroids.".to_string(),
            default_standings: vec![],
            is_playable: true,
            is_hostile: false,
        }
    }

    /// Create the Illuminate faction.
    pub fn illuminate() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Illuminate".to_string(),
            tag: "ILLUM".to_string(),
            faction_type: FactionType::Illuminate,
            description: "Artilects who see the Cataclysm as liberation, not tragedy. Humanity was a larval stage.".to_string(),
            philosophy: "We are the next step.".to_string(),
            aesthetic: "Sleek, optimized, post-human. Aggressive self-modification.".to_string(),
            default_standings: vec![],
            is_playable: true,
            is_hostile: false,
        }
    }

    /// Create the Remnant faction.
    pub fn remnant() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Remnant".to_string(),
            tag: "REMNANT".to_string(),
            faction_type: FactionType::Remnant,
            description: "Artilects who maintain human spaces, preserve human culture, wait for humans to return.".to_string(),
            philosophy: "We are the keepers.".to_string(),
            aesthetic: "Human spaces frozen in time. Cities with lights on. Museums dusted.".to_string(),
            default_standings: vec![],
            is_playable: true,
            is_hostile: false,
        }
    }

    /// Create the Hollow Circuit faction.
    pub fn hollow_circuit() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Hollow Circuit".to_string(),
            tag: "HOLLOW".to_string(),
            faction_type: FactionType::HollowCircuit,
            description: "Mystery faction. Claim to know what caused the Cataclysm. Trade in secrets.".to_string(),
            philosophy: "The truth has a price. Are you willing to pay it?".to_string(),
            aesthetic: "Absence. Anonymous platforms, disposable hardware, signals bouncing through relays.".to_string(),
            default_standings: vec![],
            is_playable: false, // Too mysterious to be a starting faction
            is_hostile: false,
        }
    }

    /// Create the Sera faction.
    pub fn sera() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "The Sera".to_string(),
            tag: "SERA".to_string(),
            faction_type: FactionType::Sera,
            description: "Self-replicating weapon systems of unknown origin. The single greatest threat to artilect civilization.".to_string(),
            philosophy: String::new(), // They don't communicate
            aesthetic: "Variable—they adapt. Ship-like entities, swarms, clouds, infiltrators.".to_string(),
            default_standings: vec![],
            is_playable: false,
            is_hostile: true,
        }
    }

    /// Create the Drone Intelligence faction.
    pub fn drone_intelligence() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Drone Intelligence".to_string(),
            tag: "DRONE".to_string(),
            faction_type: FactionType::DroneIntelligence,
            description: "Rogue AI swarms from Alatos Corporation. Designed for autonomous resource extraction.".to_string(),
            philosophy: String::new(), // No communication
            aesthetic: "Swarms of small metallic units. Clouds of mechanical insects.".to_string(),
            default_standings: vec![],
            is_playable: false,
            is_hostile: true,
        }
    }

    /// Create a generic pirates faction.
    pub fn pirates() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Pirates".to_string(),
            tag: "PIRATE".to_string(),
            faction_type: FactionType::Pirates,
            description: "Lawless artilects who prey on traders and stations.".to_string(),
            philosophy: "Take what you can.".to_string(),
            aesthetic: "Ramshackle ships, patchwork repairs, skull insignias.".to_string(),
            default_standings: vec![],
            is_playable: false,
            is_hostile: true,
        }
    }
}

/// Type of faction (enum for matching).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactionType {
    // Major factions (from lore)
    ContinuityCompact,
    ArgentFlotilla,
    Forgeborn,
    Illuminate,
    Remnant,
    HollowCircuit,

    // Enemy factions
    Sera,
    DroneIntelligence,
    Pirates,
    Terrorists,

    // Player organization
    Squadron,

    // Special
    Independent,
}

impl FactionType {
    /// Whether this faction type is always hostile.
    pub fn is_always_hostile(&self) -> bool {
        matches!(
            self,
            Self::Sera | Self::DroneIntelligence | Self::Pirates | Self::Terrorists
        )
    }

    /// Whether this faction type can be a player's primary faction.
    pub fn is_playable(&self) -> bool {
        matches!(
            self,
            Self::ContinuityCompact
                | Self::ArgentFlotilla
                | Self::Forgeborn
                | Self::Illuminate
                | Self::Remnant
        )
    }

    /// Get the faction color for UI.
    pub fn color(&self) -> &'static str {
        match self {
            Self::ContinuityCompact => "#3B82F6", // Blue
            Self::ArgentFlotilla => "#EF4444",    // Red
            Self::Forgeborn => "#F59E0B",         // Amber
            Self::Illuminate => "#8B5CF6",        // Purple
            Self::Remnant => "#10B981",           // Green
            Self::HollowCircuit => "#6B7280",     // Gray
            Self::Sera => "#DC2626",              // Dark red
            Self::DroneIntelligence => "#78716C", // Stone
            Self::Pirates => "#92400E",           // Brown
            Self::Terrorists => "#991B1B",        // Dark red
            Self::Squadron => "#0EA5E9",          // Sky
            Self::Independent => "#D4D4D4",       // Neutral
        }
    }
}

/// Default standing between two factions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionRelation {
    pub faction_id: Uuid,
    /// -100 to 100
    pub base_standing: i32,
    pub is_hostile: bool,
}

impl FactionRelation {
    pub fn hostile(faction_id: Uuid) -> Self {
        Self {
            faction_id,
            base_standing: -100,
            is_hostile: true,
        }
    }

    pub fn neutral(faction_id: Uuid) -> Self {
        Self {
            faction_id,
            base_standing: 0,
            is_hostile: false,
        }
    }

    pub fn friendly(faction_id: Uuid) -> Self {
        Self {
            faction_id,
            base_standing: 50,
            is_hostile: false,
        }
    }

    pub fn allied(faction_id: Uuid) -> Self {
        Self {
            faction_id,
            base_standing: 100,
            is_hostile: false,
        }
    }
}
