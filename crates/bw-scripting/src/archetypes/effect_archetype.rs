//! Effect archetype definitions
//!
//! Effect archetypes define combat effects and their default parameters.
//! They're loaded from scripts (scripts/definitions/effects.rhai).

use rhai::{Array, Dynamic, Map};
use serde::{Deserialize, Serialize};

/// Effect type determines when/how the effect is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EffectType {
    /// Applied immediately during damage calculation.
    #[default]
    Instant,
    /// Creates a status effect on the target that lasts for a duration.
    Duration,
    /// Fires on certain combat events (on_hit, on_kill, etc.).
    Trigger,
}

impl EffectType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "duration" => EffectType::Duration,
            "trigger" => EffectType::Trigger,
            _ => EffectType::Instant,
        }
    }
}

/// How multiple instances of the same effect interact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StackingBehavior {
    /// Reset duration, don't add another instance.
    #[default]
    Refresh,
    /// Multiple instances can exist.
    Stack,
    /// Don't apply if already present.
    Ignore,
}

impl StackingBehavior {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stack" => StackingBehavior::Stack,
            "ignore" => StackingBehavior::Ignore,
            _ => StackingBehavior::Refresh,
        }
    }
}

/// When a trigger effect fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectTrigger {
    OnAttack,
    OnHit,
    OnDamage,
    OnKill,
    OnTick,
}

impl EffectTrigger {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "on_attack" => Some(EffectTrigger::OnAttack),
            "on_hit" => Some(EffectTrigger::OnHit),
            "on_damage" => Some(EffectTrigger::OnDamage),
            "on_kill" => Some(EffectTrigger::OnKill),
            "on_tick" => Some(EffectTrigger::OnTick),
            _ => None,
        }
    }
}

/// An effect archetype defining a type of combat effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectArchetype {
    /// Unique identifier (e.g., "shield_piercing", "burn").
    pub id: String,

    /// Display name.
    pub name: String,

    /// Optional description.
    pub description: Option<String>,

    /// Effect type (instant, duration, trigger).
    pub effect_type: EffectType,

    /// Handler ID - maps to a registered handler in EffectRegistry.
    pub handler: String,

    /// Default parameters for this effect.
    pub params: Map,

    /// How stacking works for duration effects.
    pub stacking: StackingBehavior,

    /// When this effect triggers (for trigger type effects).
    pub trigger: Option<EffectTrigger>,
}

impl Default for EffectArchetype {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            effect_type: EffectType::Instant,
            handler: String::new(),
            params: Map::new(),
            stacking: StackingBehavior::Refresh,
            trigger: None,
        }
    }
}

impl EffectArchetype {
    /// Parse an effect archetype from a Rhai Dynamic value.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = get_string(&map, "id")?;
        let name = get_string(&map, "name").unwrap_or_else(|| id.clone());

        // Parse effect type
        let effect_type = get_string(&map, "type")
            .map(|s| EffectType::from_str(&s))
            .unwrap_or_default();

        // Parse stacking behavior
        let stacking = get_string(&map, "stacking")
            .map(|s| StackingBehavior::from_str(&s))
            .unwrap_or_default();

        // Parse trigger
        let trigger = get_string(&map, "trigger")
            .and_then(|s| EffectTrigger::from_str(&s));

        // Parse params - this is a nested map
        let params = map.get("params")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap_or_default();

        Some(Self {
            id,
            name,
            description: get_string(&map, "description"),
            effect_type,
            handler: get_string(&map, "handler").unwrap_or_default(),
            params,
            stacking,
            trigger,
        })
    }

    /// Get a parameter as f32.
    pub fn get_param_f32(&self, key: &str) -> Option<f32> {
        self.params.get(key).and_then(|v| {
            v.as_float().ok().map(|f| f as f32)
                .or_else(|| v.as_int().ok().map(|i| i as f32))
        })
    }

    /// Get a parameter as i64.
    pub fn get_param_i64(&self, key: &str) -> Option<i64> {
        self.params.get(key).and_then(|v| v.as_int().ok())
    }

    /// Get a parameter as string.
    pub fn get_param_string(&self, key: &str) -> Option<String> {
        self.params.get(key).and_then(|v| v.clone().into_string().ok())
    }

    /// Get a parameter as string array.
    pub fn get_param_string_array(&self, key: &str) -> Vec<String> {
        self.params.get(key)
            .and_then(|v| v.clone().try_cast::<Array>())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(|e| e.into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

// Helper functions for parsing Rhai maps
fn get_string(map: &Map, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.clone().into_string().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_type_from_str() {
        assert_eq!(EffectType::from_str("instant"), EffectType::Instant);
        assert_eq!(EffectType::from_str("duration"), EffectType::Duration);
        assert_eq!(EffectType::from_str("trigger"), EffectType::Trigger);
        assert_eq!(EffectType::from_str("unknown"), EffectType::Instant);
    }

    #[test]
    fn test_stacking_from_str() {
        assert_eq!(StackingBehavior::from_str("refresh"), StackingBehavior::Refresh);
        assert_eq!(StackingBehavior::from_str("stack"), StackingBehavior::Stack);
        assert_eq!(StackingBehavior::from_str("ignore"), StackingBehavior::Ignore);
    }

    #[test]
    fn test_from_dynamic() {
        let mut map = Map::new();
        map.insert("id".into(), Dynamic::from("test_effect"));
        map.insert("handler".into(), Dynamic::from("test_handler"));
        map.insert("type".into(), Dynamic::from("duration"));

        let mut params = Map::new();
        params.insert("damage".into(), Dynamic::from(10.0_f64));
        map.insert("params".into(), Dynamic::from(params));

        let effect = EffectArchetype::from_dynamic(Dynamic::from(map)).unwrap();
        assert_eq!(effect.id, "test_effect");
        assert_eq!(effect.handler, "test_handler");
        assert_eq!(effect.effect_type, EffectType::Duration);
        assert_eq!(effect.get_param_f32("damage"), Some(10.0));
    }
}
