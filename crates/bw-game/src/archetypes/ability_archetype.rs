//! Ability archetype definitions
//!
//! Ability archetypes define player-usable abilities and their parameters.
//! They're loaded from scripts (scripts/definitions/abilities.rhai).
//!
//! Abilities differ from effects:
//! - Abilities are player-triggered actions
//! - Effects are internal combat mechanics
//! - Abilities may apply one or more effects when used

use rhai::{Array, Dynamic, Map};
use serde::{Deserialize, Serialize};

/// Target type for abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AbilityTarget {
    /// Targets the user's own ship.
    #[default]
    SelfTarget,
    /// Targets a friendly ship.
    Friendly,
    /// Targets an enemy ship.
    Enemy,
    /// Targets any ship.
    Any,
    /// Area effect centered on a point or the user.
    Area,
}

impl AbilityTarget {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "self" | "self_target" => AbilityTarget::SelfTarget,
            "friendly" | "ally" => AbilityTarget::Friendly,
            "enemy" | "hostile" => AbilityTarget::Enemy,
            "any" => AbilityTarget::Any,
            "area" | "aoe" => AbilityTarget::Area,
            _ => AbilityTarget::SelfTarget,
        }
    }
}

/// Resource cost for an ability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AbilityCost {
    /// Energy cost.
    pub energy: i32,
    /// Fuel cost.
    pub fuel: i32,
    /// Ammunition cost.
    pub ammunition: i32,
    /// Credits cost.
    pub credits: i64,
    /// Shields cost (drains from own shields to power ability).
    pub shields: i32,
}

impl AbilityCost {
    /// Check if there's any cost.
    pub fn is_free(&self) -> bool {
        self.energy == 0 && self.fuel == 0 && self.ammunition == 0 && self.credits == 0 && self.shields == 0
    }

    /// Parse from Rhai map.
    pub fn from_dynamic(value: Dynamic) -> Self {
        let map = match value.try_cast::<Map>() {
            Some(m) => m,
            None => return Self::default(),
        };

        Self {
            energy: get_i32(&map, "energy").unwrap_or(0),
            fuel: get_i32(&map, "fuel").unwrap_or(0),
            ammunition: get_i32(&map, "ammunition").unwrap_or(0),
            credits: get_i64(&map, "credits").unwrap_or(0),
            shields: get_i32(&map, "shields").unwrap_or(0),
        }
    }
}

/// Effect applied by an ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityEffect {
    /// Effect ID to apply.
    pub effect_id: String,
    /// Override parameters for the effect.
    pub params: Map,
}

impl AbilityEffect {
    /// Parse from Rhai map.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let effect_id = get_string(&map, "effect_id")?;
        let params = map.get("params")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap_or_default();

        Some(Self { effect_id, params })
    }
}

/// Requirements to use an ability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AbilityRequirements {
    /// Minimum ship tier required.
    pub min_tier: Option<i32>,
    /// Required ship class IDs (any of these).
    pub ship_classes: Vec<String>,
    /// Required upgrade IDs (all of these).
    pub required_upgrades: Vec<String>,
    /// Must not be in combat.
    pub out_of_combat_only: bool,
    /// Must be in combat.
    pub in_combat_only: bool,
    /// Minimum fame required.
    pub min_fame: Option<i32>,
    /// Minimum or maximum reputation required (can be negative for "shady" requirements).
    pub min_reputation: Option<i32>,
}

impl AbilityRequirements {
    /// Parse from Rhai map.
    pub fn from_dynamic(value: Dynamic) -> Self {
        let map = match value.try_cast::<Map>() {
            Some(m) => m,
            None => return Self::default(),
        };

        Self {
            min_tier: get_i32(&map, "min_tier"),
            ship_classes: get_string_array(&map, "ship_classes"),
            required_upgrades: get_string_array(&map, "required_upgrades"),
            out_of_combat_only: get_bool(&map, "out_of_combat_only").unwrap_or(false),
            in_combat_only: get_bool(&map, "in_combat_only").unwrap_or(false),
            min_fame: get_i32(&map, "min_fame"),
            min_reputation: get_i32(&map, "min_reputation"),
        }
    }
}

/// An ability archetype defining a player-usable ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityArchetype {
    /// Unique identifier (e.g., "emergency_shields").
    pub id: String,

    /// Display name.
    pub name: String,

    /// Description shown to players.
    pub description: Option<String>,

    /// Cooldown in game ticks.
    pub cooldown: u32,

    /// Resource cost to use.
    pub cost: AbilityCost,

    /// Valid targets for this ability.
    pub target: AbilityTarget,

    /// Effects applied when ability is used.
    pub effects: Vec<AbilityEffect>,

    /// Requirements to use this ability.
    pub requirements: AbilityRequirements,

    /// Icon identifier for UI.
    pub icon: Option<String>,

    /// Tier of the ability (for UI sorting).
    pub tier: i32,

    /// Whether this ability is passive (always active).
    pub is_passive: bool,
}

impl Default for AbilityArchetype {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            cooldown: 0,
            cost: AbilityCost::default(),
            target: AbilityTarget::SelfTarget,
            effects: vec![],
            requirements: AbilityRequirements::default(),
            icon: None,
            tier: 1,
            is_passive: false,
        }
    }
}

impl AbilityArchetype {
    /// Parse an ability archetype from a Rhai Dynamic value.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = get_string(&map, "id")?;
        let name = get_string(&map, "name").unwrap_or_else(|| id.clone());

        // Parse target type
        let target = get_string(&map, "target")
            .map(|s| AbilityTarget::from_str(&s))
            .unwrap_or_default();

        // Parse cost
        let cost = map.get("cost")
            .map(|v| AbilityCost::from_dynamic(v.clone()))
            .unwrap_or_default();

        // Parse effects
        let effects = map.get("effects")
            .and_then(|v| v.clone().try_cast::<Array>())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(AbilityEffect::from_dynamic)
                    .collect()
            })
            .unwrap_or_default();

        // Parse requirements
        let requirements = map.get("requirements")
            .map(|v| AbilityRequirements::from_dynamic(v.clone()))
            .unwrap_or_default();

        Some(Self {
            id,
            name,
            description: get_string(&map, "description"),
            cooldown: get_i32(&map, "cooldown").unwrap_or(0) as u32,
            cost,
            target,
            effects,
            requirements,
            icon: get_string(&map, "icon"),
            tier: get_i32(&map, "tier").unwrap_or(1),
            is_passive: get_bool(&map, "is_passive").unwrap_or(false),
        })
    }

    /// Check if the ability has any effects.
    pub fn has_effects(&self) -> bool {
        !self.effects.is_empty()
    }

    /// Check if the ability has a cooldown.
    pub fn has_cooldown(&self) -> bool {
        self.cooldown > 0
    }
}

// Helper functions for parsing Rhai maps
fn get_string(map: &Map, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.clone().into_string().ok())
}

fn get_i32(map: &Map, key: &str) -> Option<i32> {
    map.get(key).and_then(|v| v.as_int().ok().map(|i| i as i32))
}

fn get_i64(map: &Map, key: &str) -> Option<i64> {
    map.get(key).and_then(|v| v.as_int().ok())
}

fn get_bool(map: &Map, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| v.as_bool().ok())
}

fn get_string_array(map: &Map, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(|v| v.clone().try_cast::<Array>())
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
    fn test_ability_target_from_str() {
        assert_eq!(AbilityTarget::from_str("self"), AbilityTarget::SelfTarget);
        assert_eq!(AbilityTarget::from_str("friendly"), AbilityTarget::Friendly);
        assert_eq!(AbilityTarget::from_str("enemy"), AbilityTarget::Enemy);
        assert_eq!(AbilityTarget::from_str("area"), AbilityTarget::Area);
        assert_eq!(AbilityTarget::from_str("unknown"), AbilityTarget::SelfTarget);
    }

    #[test]
    fn test_ability_cost_is_free() {
        let free = AbilityCost::default();
        assert!(free.is_free());

        let not_free = AbilityCost { energy: 10, ..Default::default() };
        assert!(!not_free.is_free());
    }

    #[test]
    fn test_from_dynamic() {
        let mut map = Map::new();
        map.insert("id".into(), Dynamic::from("test_ability"));
        map.insert("cooldown".into(), Dynamic::from(60_i64));
        map.insert("target".into(), Dynamic::from("enemy"));

        let mut cost = Map::new();
        cost.insert("energy".into(), Dynamic::from(25_i64));
        map.insert("cost".into(), Dynamic::from(cost));

        let ability = AbilityArchetype::from_dynamic(Dynamic::from(map)).unwrap();
        assert_eq!(ability.id, "test_ability");
        assert_eq!(ability.cooldown, 60);
        assert_eq!(ability.target, AbilityTarget::Enemy);
        assert_eq!(ability.cost.energy, 25);
    }
}
