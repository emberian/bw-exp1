//! Effect System
//!
//! Combat effects that modify damage, apply status effects, or trigger on events.
//! Uses the generic handler infrastructure for dispatch.
//!
//! # Architecture
//!
//! Effects are internal game mechanics (not player-facing actions).
//! They're triggered by combat resolution, abilities, or other game systems.
//!
//! ```text
//! EffectContext       - Combat-specific context (source, target, damage)
//! EffectDispatcher    - HandlerDispatcher<EffectContext>
//! Built-in handlers   - Rust implementations for common effects
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // Apply an effect
//! effect_dispatcher.dispatch("shield_pierce", &ctx, &params);
//!
//! // Scripts can trigger effects
//! apply_effect("burn", source_id, target_id, #{ damage_per_tick: 5.0 });
//! ```

mod builtins;

use std::sync::Arc;
use rhai::{Dynamic, Map};
use uuid::Uuid;

use crate::handlers::{HandlerContext, HandlerDispatcher, HandlerRegistry, Handler, HandlerResult};
use crate::engine::ScriptEngine;

pub use builtins::*;

/// Context for effect execution.
///
/// Contains combat-specific information needed by effect handlers.
#[derive(Debug, Clone)]
pub struct EffectContext {
    /// Entity applying the effect (attacker, ability user).
    pub source_id: Uuid,
    /// Entity receiving the effect (defender, target).
    pub target_id: Uuid,
    /// Current sector.
    pub sector_id: Uuid,
    /// Base damage (before effects).
    pub damage: f32,
    /// Current damage after modifiers.
    pub modified_damage: f32,
    /// Weapon ID if applicable.
    pub weapon_id: Option<String>,
    /// Current game tick.
    pub tick: u64,
    /// Effect-specific data that handlers can read/write.
    pub effect_data: Map,
}

impl EffectContext {
    /// Create a new effect context.
    pub fn new(source_id: Uuid, target_id: Uuid, sector_id: Uuid, damage: f32) -> Self {
        Self {
            source_id,
            target_id,
            sector_id,
            damage,
            modified_damage: damage,
            weapon_id: None,
            tick: 0,
            effect_data: Map::new(),
        }
    }

    /// Set the weapon ID.
    pub fn with_weapon(mut self, weapon_id: impl Into<String>) -> Self {
        self.weapon_id = Some(weapon_id.into());
        self
    }

    /// Set the tick.
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    /// Get the final damage after all modifiers.
    pub fn final_damage(&self) -> f32 {
        self.modified_damage
    }

    /// Modify the damage by a multiplier.
    pub fn multiply_damage(&mut self, multiplier: f32) {
        self.modified_damage *= multiplier;
    }

    /// Add to the damage.
    pub fn add_damage(&mut self, amount: f32) {
        self.modified_damage += amount;
    }

    /// Set a value in effect_data.
    pub fn set_data(&mut self, key: &str, value: Dynamic) {
        self.effect_data.insert(key.into(), value);
    }

    /// Get a value from effect_data.
    pub fn get_data(&self, key: &str) -> Option<&Dynamic> {
        self.effect_data.get(key)
    }
}

impl HandlerContext for EffectContext {
    fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("source_id".into(), self.source_id.to_string().into());
        map.insert("target_id".into(), self.target_id.to_string().into());
        map.insert("sector_id".into(), self.sector_id.to_string().into());
        map.insert("damage".into(), Dynamic::from(self.damage));
        map.insert("modified_damage".into(), Dynamic::from(self.modified_damage));
        map.insert("tick".into(), Dynamic::from(self.tick as i64));
        if let Some(ref weapon_id) = self.weapon_id {
            map.insert("weapon_id".into(), weapon_id.clone().into());
        }
        map.insert("effect_data".into(), Dynamic::from(self.effect_data.clone()));
        Dynamic::from(map)
    }

    fn sector_id(&self) -> Option<Uuid> {
        Some(self.sector_id)
    }

    fn owner_entity_id(&self) -> Option<Uuid> {
        Some(self.source_id)
    }
}

/// Effect dispatcher - specialization of HandlerDispatcher for effects.
pub type EffectDispatcher = HandlerDispatcher<EffectContext>;

/// Effect registry - specialization of HandlerRegistry for effects.
pub type EffectRegistry = HandlerRegistry<EffectContext>;

/// Create a new effect registry with built-in handlers registered.
pub fn create_effect_registry() -> EffectRegistry {
    let registry = EffectRegistry::new();

    // Register built-in effect handlers
    register_builtin_effects(&registry);

    registry
}

/// Create a new effect dispatcher.
pub fn create_effect_dispatcher(
    registry: Arc<EffectRegistry>,
    engine: Arc<ScriptEngine>,
) -> EffectDispatcher {
    HandlerDispatcher::new(registry, engine)
}

/// Register built-in effect handlers.
fn register_builtin_effects(registry: &EffectRegistry) {
    // Damage modifiers
    registry.register(Handler::builtin("shield_pierce", Arc::new(ShieldPierceHandler)));
    registry.register(Handler::builtin("armor_pierce", Arc::new(ArmorPierceHandler)));
    registry.register(Handler::builtin("damage_mult", Arc::new(DamageMultHandler)));

    // Status effects
    registry.register(Handler::builtin("dot", Arc::new(DotHandler)));
    registry.register(Handler::builtin("stat_modifier", Arc::new(StatModifierHandler)));

    // Combat modifiers
    registry.register(Handler::builtin("accuracy_bonus", Arc::new(AccuracyBonusHandler)));
    registry.register(Handler::builtin("multi_attack", Arc::new(MultiAttackHandler)));

    // Special effects
    registry.register(Handler::builtin("emp", Arc::new(EmpHandler)));
    registry.register(Handler::builtin("aoe", Arc::new(AoeHandler)));

    tracing::debug!(count = registry.count(), "Built-in effect handlers registered");
}

/// Helper to apply an effect and return the modified context.
pub fn apply_effect(
    dispatcher: &EffectDispatcher,
    effect_id: &str,
    ctx: &mut EffectContext,
    params: &Dynamic,
) -> HandlerResult {
    let result = dispatcher.dispatch(effect_id, ctx, params);

    // If the handler returned modified damage in data, apply it
    if let Some(ref data) = result.data {
        if let Some(map) = data.clone().try_cast::<Map>() {
            if let Some(new_damage) = map.get("modified_damage")
                .and_then(|v| v.clone().try_cast::<f32>())
            {
                ctx.modified_damage = new_damage;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_context() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let sector = Uuid::new_v4();

        let mut ctx = EffectContext::new(source, target, sector, 100.0);
        assert_eq!(ctx.final_damage(), 100.0);

        ctx.multiply_damage(1.5);
        assert_eq!(ctx.final_damage(), 150.0);

        ctx.add_damage(25.0);
        assert_eq!(ctx.final_damage(), 175.0);
    }

    #[test]
    fn test_effect_context_to_dynamic() {
        let ctx = EffectContext::new(
            Uuid::nil(),
            Uuid::nil(),
            Uuid::nil(),
            50.0,
        );

        let dynamic = ctx.to_dynamic();
        let map = dynamic.cast::<Map>();

        assert!(map.contains_key("source_id"));
        assert!(map.contains_key("damage"));
        assert_eq!(map.get("damage").unwrap().clone().cast::<f32>(), 50.0);
    }
}
