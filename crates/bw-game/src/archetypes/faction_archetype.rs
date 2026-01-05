//! Faction archetype definitions
//!
//! Faction archetypes define the properties, behaviors, and relationships
//! between different groups in the game. They're loaded from scripts
//! (scripts/definitions/factions.rhai).
//!
//! Factions affect:
//! - NPC AI behaviors (aggression, patrol, flee thresholds)
//! - Combat bonuses (attack, defense, speed modifiers)
//! - Standing and relations between groups
//! - Available missions and dialogue options

use std::collections::HashMap;
use rhai::{Dynamic, Map};
use serde::{Deserialize, Serialize};

/// Behavior modifiers that affect NPC AI decisions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionBehaviors {
    /// Aggression level (0.0 = pacifist, 1.0 = extremely aggressive).
    pub aggression: f32,

    /// Trade preference (0.0 = no trading, 1.0 = prefers trading over combat).
    pub trade_preference: f32,

    /// Patrol range in sectors.
    pub patrol_range: f32,

    /// Health threshold to flee (0.0 = never flee, 1.0 = flee at first damage).
    pub flee_threshold: f32,

    /// Preferred target type (e.g., "cargo", "weakest", "strongest", "player").
    pub target_preference: Option<String>,

    /// Whether to call for reinforcements.
    pub calls_reinforcements: bool,

    /// Chance to offer surrender (0.0 - 1.0).
    pub surrender_chance: f32,
}

impl FactionBehaviors {
    /// Parse from Rhai map.
    pub fn from_dynamic(value: Dynamic) -> Self {
        let map = match value.try_cast::<Map>() {
            Some(m) => m,
            None => return Self::default(),
        };

        Self {
            aggression: get_f32(&map, "aggression").unwrap_or(0.5),
            trade_preference: get_f32(&map, "trade_preference").unwrap_or(0.5),
            patrol_range: get_f32(&map, "patrol_range").unwrap_or(1.0),
            flee_threshold: get_f32(&map, "flee_threshold").unwrap_or(0.2),
            target_preference: get_string(&map, "target_preference"),
            calls_reinforcements: get_bool(&map, "calls_reinforcements").unwrap_or(false),
            surrender_chance: get_f32(&map, "surrender_chance").unwrap_or(0.0),
        }
    }
}

/// Combat bonuses applied to faction members.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionCombatBonuses {
    /// Attack damage multiplier (1.0 = normal, 1.1 = +10%).
    pub attack_bonus: f32,

    /// Defense/damage reduction multiplier.
    pub defense_bonus: f32,

    /// Speed multiplier.
    pub speed_bonus: f32,

    /// Accuracy bonus (added to hit chance).
    pub accuracy_bonus: f32,

    /// Shield strength multiplier.
    pub shield_bonus: f32,

    /// Critical hit chance bonus.
    pub critical_bonus: f32,
}

impl FactionCombatBonuses {
    /// Parse from Rhai map.
    pub fn from_dynamic(value: Dynamic) -> Self {
        let map = match value.try_cast::<Map>() {
            Some(m) => m,
            None => return Self::default(),
        };

        Self {
            attack_bonus: get_f32(&map, "attack_bonus").unwrap_or(0.0),
            defense_bonus: get_f32(&map, "defense_bonus").unwrap_or(0.0),
            speed_bonus: get_f32(&map, "speed_bonus").unwrap_or(0.0),
            accuracy_bonus: get_f32(&map, "accuracy_bonus").unwrap_or(0.0),
            shield_bonus: get_f32(&map, "shield_bonus").unwrap_or(0.0),
            critical_bonus: get_f32(&map, "critical_bonus").unwrap_or(0.0),
        }
    }

    /// Check if any bonuses are defined.
    pub fn has_bonuses(&self) -> bool {
        self.attack_bonus != 0.0
            || self.defense_bonus != 0.0
            || self.speed_bonus != 0.0
            || self.accuracy_bonus != 0.0
            || self.shield_bonus != 0.0
            || self.critical_bonus != 0.0
    }
}

/// Standing requirements for faction-specific content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionStandingRequirements {
    /// Minimum standing to access stations.
    pub station_access: i32,
    /// Minimum standing to access missions.
    pub mission_access: i32,
    /// Minimum standing to access premium markets.
    pub premium_market_access: i32,
    /// Standing below which faction becomes hostile.
    pub hostile_threshold: i32,
}

impl FactionStandingRequirements {
    /// Parse from Rhai map.
    pub fn from_dynamic(value: Dynamic) -> Self {
        let map = match value.try_cast::<Map>() {
            Some(m) => m,
            None => return Self::default(),
        };

        Self {
            station_access: get_i32(&map, "station_access").unwrap_or(-50),
            mission_access: get_i32(&map, "mission_access").unwrap_or(0),
            premium_market_access: get_i32(&map, "premium_market_access").unwrap_or(50),
            hostile_threshold: get_i32(&map, "hostile_threshold").unwrap_or(-100),
        }
    }
}

/// A faction archetype defining a group/organization in the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionArchetype {
    /// Unique identifier (e.g., "continuity_compact", "pirates").
    pub id: String,

    /// Display name.
    pub name: String,

    /// Short tag for UI (e.g., "COMPACT", "PIRATE").
    pub tag: String,

    /// Description shown to players.
    pub description: Option<String>,

    /// Color for UI (hex string like "#FF0000").
    pub color: String,

    /// Whether players can belong to this faction.
    pub is_playable: bool,

    /// Whether this faction is always hostile to players.
    pub is_hostile: bool,

    /// Whether this faction controls territory.
    pub is_territorial: bool,

    /// AI behavior modifiers.
    pub behavior_modifiers: FactionBehaviors,

    /// Combat stat bonuses.
    pub combat_bonuses: FactionCombatBonuses,

    /// Standing requirements.
    pub standing_requirements: FactionStandingRequirements,

    /// Base relations with other factions (faction_id -> standing).
    pub relations: HashMap<String, i32>,

    /// Icon identifier for UI.
    pub icon: Option<String>,

    /// Home sector(s) for this faction.
    pub home_sectors: Vec<String>,

    /// Ship classes preferred by this faction.
    pub preferred_ships: Vec<String>,

    /// Tier/power level (1 = minor, 5 = major power).
    pub tier: i32,
}

impl Default for FactionArchetype {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            tag: String::new(),
            description: None,
            color: "#FFFFFF".to_string(),
            is_playable: false,
            is_hostile: false,
            is_territorial: false,
            behavior_modifiers: FactionBehaviors::default(),
            combat_bonuses: FactionCombatBonuses::default(),
            standing_requirements: FactionStandingRequirements::default(),
            relations: HashMap::new(),
            icon: None,
            home_sectors: vec![],
            preferred_ships: vec![],
            tier: 1,
        }
    }
}

impl FactionArchetype {
    /// Parse a faction archetype from a Rhai Dynamic value.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = get_string(&map, "id")?;
        let name = get_string(&map, "name").unwrap_or_else(|| id.clone());
        let tag = get_string(&map, "tag").unwrap_or_else(|| {
            // Generate tag from name (first 6 chars uppercase)
            name.chars().take(6).collect::<String>().to_uppercase()
        });

        // Parse behavior modifiers
        let behavior_modifiers = map.get("behavior_modifiers")
            .map(|v| FactionBehaviors::from_dynamic(v.clone()))
            .unwrap_or_default();

        // Parse combat bonuses
        let combat_bonuses = map.get("combat_bonuses")
            .map(|v| FactionCombatBonuses::from_dynamic(v.clone()))
            .unwrap_or_default();

        // Parse standing requirements
        let standing_requirements = map.get("standing_requirements")
            .map(|v| FactionStandingRequirements::from_dynamic(v.clone()))
            .unwrap_or_default();

        // Parse relations (map of faction_id -> standing)
        let relations = map.get("relations")
            .and_then(|v| v.clone().try_cast::<Map>())
            .map(|m| {
                m.into_iter()
                    .filter_map(|(k, v)| {
                        let standing = v.as_int().ok().map(|i| i as i32)?;
                        Some((k.to_string(), standing))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            id,
            name,
            tag,
            description: get_string(&map, "description"),
            color: get_string(&map, "color").unwrap_or_else(|| "#FFFFFF".to_string()),
            is_playable: get_bool(&map, "is_playable").unwrap_or(false),
            is_hostile: get_bool(&map, "is_hostile").unwrap_or(false),
            is_territorial: get_bool(&map, "is_territorial").unwrap_or(false),
            behavior_modifiers,
            combat_bonuses,
            standing_requirements,
            relations,
            icon: get_string(&map, "icon"),
            home_sectors: get_string_array(&map, "home_sectors"),
            preferred_ships: get_string_array(&map, "preferred_ships"),
            tier: get_i32(&map, "tier").unwrap_or(1),
        })
    }

    /// Get the base relation with another faction.
    ///
    /// Returns 0 (neutral) if no relation is defined.
    pub fn get_relation(&self, other_faction_id: &str) -> i32 {
        self.relations.get(other_faction_id).copied().unwrap_or(0)
    }

    /// Check if this faction is hostile to another.
    pub fn is_hostile_to(&self, other_faction_id: &str) -> bool {
        let standing = self.get_relation(other_faction_id);
        standing <= self.standing_requirements.hostile_threshold
    }

    /// Check if this faction is allied with another.
    pub fn is_allied_with(&self, other_faction_id: &str) -> bool {
        self.get_relation(other_faction_id) >= 50
    }

    /// Apply combat bonuses to stats.
    pub fn apply_combat_bonuses(&self, base_attack: f32, base_defense: f32, base_speed: f32) -> (f32, f32, f32) {
        let attack = base_attack * (1.0 + self.combat_bonuses.attack_bonus);
        let defense = base_defense * (1.0 + self.combat_bonuses.defense_bonus);
        let speed = base_speed * (1.0 + self.combat_bonuses.speed_bonus);
        (attack, defense, speed)
    }
}

// Helper functions for parsing Rhai maps
fn get_string(map: &Map, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.clone().into_string().ok())
}

fn get_i32(map: &Map, key: &str) -> Option<i32> {
    map.get(key).and_then(|v| v.as_int().ok().map(|i| i as i32))
}

fn get_f32(map: &Map, key: &str) -> Option<f32> {
    map.get(key).and_then(|v| {
        v.as_float().ok().map(|f| f as f32)
            .or_else(|| v.as_int().ok().map(|i| i as f32))
    })
}

fn get_bool(map: &Map, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| v.as_bool().ok())
}

fn get_string_array(map: &Map, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(|v| v.clone().try_cast::<rhai::Array>())
        .map(|arr| {
            arr.into_iter()
                .filter_map(|e| e.into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faction_behaviors_defaults() {
        let behaviors = FactionBehaviors::default();
        assert_eq!(behaviors.aggression, 0.0);
        assert_eq!(behaviors.flee_threshold, 0.0);
        assert!(!behaviors.calls_reinforcements);
    }

    #[test]
    fn test_combat_bonuses_has_bonuses() {
        let no_bonuses = FactionCombatBonuses::default();
        assert!(!no_bonuses.has_bonuses());

        let with_bonuses = FactionCombatBonuses {
            attack_bonus: 0.1,
            ..Default::default()
        };
        assert!(with_bonuses.has_bonuses());
    }

    #[test]
    fn test_from_dynamic() {
        let mut map = Map::new();
        map.insert("id".into(), Dynamic::from("test_faction"));
        map.insert("name".into(), Dynamic::from("Test Faction"));
        map.insert("is_hostile".into(), Dynamic::from(true));
        map.insert("color".into(), Dynamic::from("#FF0000"));

        let mut relations = Map::new();
        relations.insert("other_faction".into(), Dynamic::from(-50_i64));
        map.insert("relations".into(), Dynamic::from(relations));

        let faction = FactionArchetype::from_dynamic(Dynamic::from(map)).unwrap();
        assert_eq!(faction.id, "test_faction");
        assert_eq!(faction.name, "Test Faction");
        assert!(faction.is_hostile);
        assert_eq!(faction.color, "#FF0000");
        assert_eq!(faction.get_relation("other_faction"), -50);
    }

    #[test]
    fn test_hostility_check() {
        let mut faction = FactionArchetype::default();
        faction.relations.insert("enemy".to_string(), -100);
        faction.relations.insert("ally".to_string(), 50);
        faction.standing_requirements.hostile_threshold = -50;

        assert!(faction.is_hostile_to("enemy"));
        assert!(!faction.is_hostile_to("ally"));
        assert!(!faction.is_hostile_to("neutral")); // Unknown = 0 = not hostile
    }

    #[test]
    fn test_apply_combat_bonuses() {
        let mut faction = FactionArchetype::default();
        faction.combat_bonuses.attack_bonus = 0.1;  // +10%
        faction.combat_bonuses.defense_bonus = 0.2; // +20%
        faction.combat_bonuses.speed_bonus = -0.1;  // -10%

        let (attack, defense, speed) = faction.apply_combat_bonuses(100.0, 50.0, 80.0);
        assert!((attack - 110.0).abs() < 0.001);
        assert!((defense - 60.0).abs() < 0.001);
        assert!((speed - 72.0).abs() < 0.001);
    }
}
