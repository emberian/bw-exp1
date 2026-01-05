//! Effect API bindings for Rhai
//!
//! Exposes the effect system to scripts, allowing them to:
//! - Apply effects during combat
//! - Register custom effect handlers
//! - Query effect archetype data

use rhai::{Engine, Array, Map, Dynamic};
use uuid::Uuid;

use crate::effects::EffectContext;
use crate::handlers::Handler;
use crate::context::{with_effect_dispatcher, current_script_path, current_sector, current_tick};

/// Register effect API functions with the engine.
pub fn register(engine: &mut Engine) {
    // apply_effect(effect_id: String, source_id: String, target_id: String, params: Map) -> Map
    // Apply a combat effect. Returns result with success/error/data.
    engine.register_fn("apply_effect",
        |effect_id: String, source_id: String, target_id: String, params: Map| -> Map {
            let source = match Uuid::parse_str(&source_id) {
                Ok(id) => id,
                Err(_) => {
                    let mut result = Map::new();
                    result.insert("success".into(), Dynamic::from(false));
                    result.insert("error".into(), Dynamic::from("Invalid source_id UUID"));
                    return result;
                }
            };

            let target = match Uuid::parse_str(&target_id) {
                Ok(id) => id,
                Err(_) => {
                    let mut result = Map::new();
                    result.insert("success".into(), Dynamic::from(false));
                    result.insert("error".into(), Dynamic::from("Invalid target_id UUID"));
                    return result;
                }
            };

            let sector_id = current_sector().unwrap_or(Uuid::nil());
            let tick = current_tick();

            with_effect_dispatcher(|dispatcher| {
                // Create effect context
                let ctx = EffectContext::new(source, target, sector_id, 0.0)
                    .with_tick(tick);

                // Dispatch the effect
                let handler_result = dispatcher.dispatch(&effect_id, &ctx, &Dynamic::from(params));

                // Build result map
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(handler_result.success));

                if let Some(ref error) = handler_result.error {
                    result.insert("error".into(), Dynamic::from(error.clone()));
                }

                if let Some(ref data) = handler_result.data {
                    result.insert("data".into(), data.clone());
                }

                // Include modified damage if relevant
                result.insert("modified_damage".into(), Dynamic::from(ctx.modified_damage));

                result
            }).unwrap_or_else(|| {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(false));
                result.insert("error".into(), Dynamic::from("No effect dispatcher available"));
                result
            })
        }
    );

    // apply_damage_effect(effect_id: String, source_id: String, target_id: String, base_damage: f64, params: Map) -> Map
    // Apply an effect with base damage. Returns result including modified_damage.
    engine.register_fn("apply_damage_effect",
        |effect_id: String, source_id: String, target_id: String, base_damage: f64, params: Map| -> Map {
            let source = match Uuid::parse_str(&source_id) {
                Ok(id) => id,
                Err(_) => {
                    let mut result = Map::new();
                    result.insert("success".into(), Dynamic::from(false));
                    result.insert("error".into(), Dynamic::from("Invalid source_id UUID"));
                    return result;
                }
            };

            let target = match Uuid::parse_str(&target_id) {
                Ok(id) => id,
                Err(_) => {
                    let mut result = Map::new();
                    result.insert("success".into(), Dynamic::from(false));
                    result.insert("error".into(), Dynamic::from("Invalid target_id UUID"));
                    return result;
                }
            };

            let sector_id = current_sector().unwrap_or(Uuid::nil());
            let tick = current_tick();

            with_effect_dispatcher(|dispatcher| {
                // Create effect context with base damage
                let ctx = EffectContext::new(source, target, sector_id, base_damage as f32)
                    .with_tick(tick);

                // Dispatch the effect
                let handler_result = dispatcher.dispatch(&effect_id, &ctx, &Dynamic::from(params));

                // Build result map
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(handler_result.success));

                if let Some(ref error) = handler_result.error {
                    result.insert("error".into(), Dynamic::from(error.clone()));
                }

                if let Some(ref data) = handler_result.data {
                    result.insert("data".into(), data.clone());
                }

                // Include both original and modified damage
                result.insert("base_damage".into(), Dynamic::from(base_damage));
                result.insert("modified_damage".into(), Dynamic::from(ctx.modified_damage as f64));

                result
            }).unwrap_or_else(|| {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(false));
                result.insert("error".into(), Dynamic::from("No effect dispatcher available"));
                result
            })
        }
    );

    // register_effect_handler(name: String, handler_fn: String) -> bool
    // Register a script-based effect handler.
    engine.register_fn("register_effect_handler",
        |name: String, handler_fn: String| -> bool {
            let script_path = match current_script_path() {
                Some(p) => p,
                None => {
                    tracing::warn!(script = true, "register_effect_handler called outside script context");
                    return false;
                }
            };

            with_effect_dispatcher(|dispatcher| {
                let handler = Handler::script(
                    name.clone(),
                    script_path,
                    handler_fn,
                );
                dispatcher.registry().register(handler);
                tracing::debug!(script = true, effect = %name, "Effect handler registered");
                true
            }).unwrap_or(false)
        }
    );

    // is_effect_registered(name: String) -> bool
    // Check if an effect handler is registered.
    engine.register_fn("is_effect_registered", |name: String| -> bool {
        with_effect_dispatcher(|dispatcher| {
            dispatcher.registry().get(&name).is_some()
        }).unwrap_or(false)
    });

    // list_effects() -> Array
    // List all registered effect handler names.
    engine.register_fn("list_effects", || -> Array {
        with_effect_dispatcher(|dispatcher| {
            dispatcher.registry().list_all()
                .into_iter()
                .map(Dynamic::from)
                .collect()
        }).unwrap_or_default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_parsing() {
        let valid = "550e8400-e29b-41d4-a716-446655440000";
        let invalid = "not-a-uuid";

        assert!(Uuid::parse_str(valid).is_ok());
        assert!(Uuid::parse_str(invalid).is_err());
    }
}
