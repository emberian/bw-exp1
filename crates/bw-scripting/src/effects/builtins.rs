//! Built-in effect handlers
//!
//! Rust implementations for common combat effects.
//! These are registered automatically when the effect registry is created.

use rhai::{Dynamic, Map};

use crate::handlers::{HandlerFn, HandlerResult};
use super::EffectContext;

/// Shield pierce - portion of damage bypasses shields.
///
/// Params:
/// - `bypass_percent`: f32 (0.0-1.0) - portion of damage that ignores shields
pub struct ShieldPierceHandler;

impl HandlerFn<EffectContext> for ShieldPierceHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let bypass_percent = params
            .clone()
            .try_cast::<Map>()
            .and_then(|m| m.get("bypass_percent").cloned())
            .and_then(|v| v.try_cast::<f32>())
            .unwrap_or(0.5);

        // Return data indicating shield bypass
        let mut data = Map::new();
        data.insert("shield_bypass_percent".into(), Dynamic::from(bypass_percent));
        data.insert("shield_bypass_damage".into(), Dynamic::from(ctx.modified_damage * bypass_percent));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// Armor pierce - portion of defense is ignored.
///
/// Params:
/// - `defense_ignore_percent`: f32 (0.0-1.0) - portion of defense to ignore
pub struct ArmorPierceHandler;

impl HandlerFn<EffectContext> for ArmorPierceHandler {
    fn execute(&self, _ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let defense_ignore = params
            .clone()
            .try_cast::<Map>()
            .and_then(|m| m.get("defense_ignore_percent").cloned())
            .and_then(|v| v.try_cast::<f32>())
            .unwrap_or(0.3);

        let mut data = Map::new();
        data.insert("defense_ignore_percent".into(), Dynamic::from(defense_ignore));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// Damage multiplier - multiply damage by a factor.
///
/// Params:
/// - `multiplier`: f32 - damage multiplier
pub struct DamageMultHandler;

impl HandlerFn<EffectContext> for DamageMultHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let multiplier = params
            .clone()
            .try_cast::<Map>()
            .and_then(|m| m.get("multiplier").cloned())
            .and_then(|v| v.try_cast::<f32>())
            .unwrap_or(1.0);

        let new_damage = ctx.modified_damage * multiplier;

        let mut data = Map::new();
        data.insert("modified_damage".into(), Dynamic::from(new_damage));
        data.insert("multiplier".into(), Dynamic::from(multiplier));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// Damage over time - apply periodic damage.
///
/// Params:
/// - `damage_per_tick`: f32 - damage each tick
/// - `duration_ticks`: i64 - how many ticks to last
/// - `damage_type`: String (optional) - type of damage
pub struct DotHandler;

impl HandlerFn<EffectContext> for DotHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let params_map = params.clone().try_cast::<Map>().unwrap_or_default();

        let damage_per_tick = params_map.get("damage_per_tick")
            .and_then(|v| v.clone().try_cast::<f32>())
            .unwrap_or(5.0);

        let duration_ticks = params_map.get("duration_ticks")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(30);

        let damage_type = params_map.get("damage_type")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_else(|| "fire".to_string());

        // Return data to create a status effect
        let mut data = Map::new();
        data.insert("effect_type".into(), Dynamic::from("status"));
        data.insert("status_type".into(), Dynamic::from("dot"));
        data.insert("target_id".into(), Dynamic::from(ctx.target_id.to_string()));
        data.insert("source_id".into(), Dynamic::from(ctx.source_id.to_string()));
        data.insert("damage_per_tick".into(), Dynamic::from(damage_per_tick));
        data.insert("duration_ticks".into(), Dynamic::from(duration_ticks));
        data.insert("damage_type".into(), Dynamic::from(damage_type));
        data.insert("start_tick".into(), Dynamic::from(ctx.tick as i64));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// Stat modifier - temporarily modify a stat.
///
/// Params:
/// - `stat`: String - stat to modify (attack, defense, speed, accuracy)
/// - `modifier`: f32 - modifier amount (negative for debuff)
/// - `duration_ticks`: i64 - how many ticks to last
pub struct StatModifierHandler;

impl HandlerFn<EffectContext> for StatModifierHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let params_map = params.clone().try_cast::<Map>().unwrap_or_default();

        let stat = params_map.get("stat")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_else(|| "attack".to_string());

        let modifier = params_map.get("modifier")
            .and_then(|v| v.clone().try_cast::<f32>())
            .unwrap_or(0.0);

        let duration_ticks = params_map.get("duration_ticks")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(100);

        // Return data to create a status effect
        let mut data = Map::new();
        data.insert("effect_type".into(), Dynamic::from("status"));
        data.insert("status_type".into(), Dynamic::from("stat_modifier"));
        data.insert("target_id".into(), Dynamic::from(ctx.target_id.to_string()));
        data.insert("source_id".into(), Dynamic::from(ctx.source_id.to_string()));
        data.insert("stat".into(), Dynamic::from(stat));
        data.insert("modifier".into(), Dynamic::from(modifier));
        data.insert("duration_ticks".into(), Dynamic::from(duration_ticks));
        data.insert("start_tick".into(), Dynamic::from(ctx.tick as i64));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// Accuracy bonus - increase hit chance.
///
/// Params:
/// - `bonus`: f32 - accuracy bonus (0.0-1.0)
pub struct AccuracyBonusHandler;

impl HandlerFn<EffectContext> for AccuracyBonusHandler {
    fn execute(&self, _ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let bonus = params
            .clone()
            .try_cast::<Map>()
            .and_then(|m| m.get("bonus").cloned())
            .and_then(|v| v.try_cast::<f32>())
            .unwrap_or(0.15);

        let mut data = Map::new();
        data.insert("accuracy_bonus".into(), Dynamic::from(bonus));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// Multi-attack - trigger additional attacks.
///
/// Params:
/// - `extra_attacks`: i64 - number of extra attacks
/// - `damage_mult`: f32 - damage multiplier for extra attacks
pub struct MultiAttackHandler;

impl HandlerFn<EffectContext> for MultiAttackHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let params_map = params.clone().try_cast::<Map>().unwrap_or_default();

        let extra_attacks = params_map.get("extra_attacks")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(2);

        let damage_mult = params_map.get("damage_mult")
            .and_then(|v| v.clone().try_cast::<f32>())
            .unwrap_or(0.4);

        let mut data = Map::new();
        data.insert("extra_attacks".into(), Dynamic::from(extra_attacks));
        data.insert("extra_attack_damage".into(), Dynamic::from(ctx.modified_damage * damage_mult));
        data.insert("damage_mult".into(), Dynamic::from(damage_mult));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// EMP - disable ship systems temporarily.
///
/// Params:
/// - `disable_duration`: i64 - ticks to disable for
/// - `systems`: Array - systems to disable (shields, weapons, engines)
pub struct EmpHandler;

impl HandlerFn<EffectContext> for EmpHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let params_map = params.clone().try_cast::<Map>().unwrap_or_default();

        let disable_duration = params_map.get("disable_duration")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(10);

        let systems = params_map.get("systems")
            .cloned()
            .unwrap_or_else(|| {
                Dynamic::from(vec![
                    Dynamic::from("shields"),
                    Dynamic::from("weapons"),
                ])
            });

        // Return data to create an EMP status effect
        let mut data = Map::new();
        data.insert("effect_type".into(), Dynamic::from("status"));
        data.insert("status_type".into(), Dynamic::from("emp"));
        data.insert("target_id".into(), Dynamic::from(ctx.target_id.to_string()));
        data.insert("source_id".into(), Dynamic::from(ctx.source_id.to_string()));
        data.insert("disable_duration".into(), Dynamic::from(disable_duration));
        data.insert("systems".into(), systems);
        data.insert("start_tick".into(), Dynamic::from(ctx.tick as i64));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

/// AOE - area of effect damage.
///
/// Params:
/// - `radius`: f32 - effect radius
/// - `falloff`: f32 - damage falloff at edge (0.0-1.0)
pub struct AoeHandler;

impl HandlerFn<EffectContext> for AoeHandler {
    fn execute(&self, ctx: &EffectContext, params: &Dynamic) -> HandlerResult {
        let params_map = params.clone().try_cast::<Map>().unwrap_or_default();

        let radius = params_map.get("radius")
            .and_then(|v| v.clone().try_cast::<f32>())
            .unwrap_or(50.0);

        let falloff = params_map.get("falloff")
            .and_then(|v| v.clone().try_cast::<f32>())
            .unwrap_or(0.5);

        // Return data for AOE processing
        let mut data = Map::new();
        data.insert("effect_type".into(), Dynamic::from("aoe"));
        data.insert("center_id".into(), Dynamic::from(ctx.target_id.to_string()));
        data.insert("sector_id".into(), Dynamic::from(ctx.sector_id.to_string()));
        data.insert("base_damage".into(), Dynamic::from(ctx.modified_damage));
        data.insert("radius".into(), Dynamic::from(radius));
        data.insert("falloff".into(), Dynamic::from(falloff));

        HandlerResult::success_with_data(Dynamic::from(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_ctx() -> EffectContext {
        EffectContext::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            100.0,
        )
    }

    #[test]
    fn test_shield_pierce() {
        let handler = ShieldPierceHandler;
        let ctx = test_ctx();
        let mut params = Map::new();
        params.insert("bypass_percent".into(), Dynamic::from(0.5_f32));

        let result = handler.execute(&ctx, &Dynamic::from(params));
        assert!(result.success);

        let data = result.data.unwrap().cast::<Map>();
        assert_eq!(data.get("shield_bypass_percent").unwrap().clone().cast::<f32>(), 0.5);
        assert_eq!(data.get("shield_bypass_damage").unwrap().clone().cast::<f32>(), 50.0);
    }

    #[test]
    fn test_damage_mult() {
        let handler = DamageMultHandler;
        let ctx = test_ctx();
        let mut params = Map::new();
        params.insert("multiplier".into(), Dynamic::from(1.5_f32));

        let result = handler.execute(&ctx, &Dynamic::from(params));
        assert!(result.success);

        let data = result.data.unwrap().cast::<Map>();
        assert_eq!(data.get("modified_damage").unwrap().clone().cast::<f32>(), 150.0);
    }

    #[test]
    fn test_dot() {
        let handler = DotHandler;
        let ctx = test_ctx();
        let mut params = Map::new();
        params.insert("damage_per_tick".into(), Dynamic::from(5.0_f32));
        params.insert("duration_ticks".into(), Dynamic::from(30_i64));

        let result = handler.execute(&ctx, &Dynamic::from(params));
        assert!(result.success);

        let data = result.data.unwrap().cast::<Map>();
        assert_eq!(data.get("status_type").unwrap().clone().cast::<String>(), "dot");
        assert_eq!(data.get("damage_per_tick").unwrap().clone().cast::<f32>(), 5.0);
    }
}
