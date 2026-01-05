//! Event API bindings for Rhai
//!
//! Exposes event subscription and emission functions to scripts.
//! Uses the unified `ScriptExecutionContext` for accessing event registry.

use rhai::Engine;
use uuid::Uuid;

use bw_core::events::GameEventType;
use crate::events::{EventRegistry, EventSubscription, EventFilter, parse_event_type};
use crate::context::{
    with_event_registry,
    current_script_path, current_owner_entity, current_sector,
};

/// Context for the currently executing script.
#[derive(Clone, Default)]
pub struct ScriptContext {
    pub script_path: String,
    pub owner_entity_id: Option<Uuid>,
    pub sector_id: Option<Uuid>,
}

impl ScriptContext {
    /// Create an empty context (for const initialization)
    pub const fn empty() -> Self {
        Self {
            script_path: String::new(),
            owner_entity_id: None,
            sector_id: None,
        }
    }
}

/// Get current script context from the unified execution context.
fn get_context() -> ScriptContext {
    ScriptContext {
        script_path: current_script_path().unwrap_or_default(),
        owner_entity_id: current_owner_entity(),
        sector_id: current_sector(),
    }
}

/// Access registry from the unified execution context.
fn with_registry<T, F: FnOnce(&EventRegistry) -> T>(f: F) -> Option<T> {
    with_event_registry(f)
}

/// Register event API functions with the engine.
pub fn register(engine: &mut Engine) {
    // === Subscription functions ===

    // subscribe_event(event_type: String, handler_fn: String) -> String
    // Subscribe to an event type, returns subscription ID
    engine.register_fn("subscribe_event", |event_type: String, handler_fn: String| -> String {
        let ctx = get_context();

        let event_types = match parse_event_type(&event_type) {
            Some(et) => vec![et],
            None => {
                tracing::warn!(script = true, event_type, "Unknown event type");
                return String::new();
            }
        };

        with_registry(|registry| {
            let mut sub = EventSubscription::new(
                ctx.script_path.clone(),
                handler_fn,
                event_types,
            );

            if let Some(entity_id) = ctx.owner_entity_id {
                sub = sub.with_owner(entity_id);
            }
            if let Some(sector_id) = ctx.sector_id {
                sub = sub.with_sector(sector_id);
            }

            let id = registry.subscribe(sub);
            id.to_string()
        }).unwrap_or_default()
    });

    // subscribe_events(event_types: Array, handler_fn: String) -> String
    // Subscribe to multiple event types at once
    engine.register_fn("subscribe_events", |event_types_arr: rhai::Array, handler_fn: String| -> String {
        let ctx = get_context();

        let event_types: Vec<GameEventType> = event_types_arr.iter()
            .filter_map(|v| v.clone().into_string().ok())
            .filter_map(|s| parse_event_type(&s))
            .collect();

        if event_types.is_empty() {
            return String::new();
        }

        with_registry(|registry| {
            let mut sub = EventSubscription::new(
                ctx.script_path.clone(),
                handler_fn,
                event_types,
            );

            if let Some(entity_id) = ctx.owner_entity_id {
                sub = sub.with_owner(entity_id);
            }
            if let Some(sector_id) = ctx.sector_id {
                sub = sub.with_sector(sector_id);
            }

            let id = registry.subscribe(sub);
            id.to_string()
        }).unwrap_or_default()
    });

    // subscribe_event_filtered(event_type: String, handler_fn: String, filter_type: String, filter_value: String) -> String
    // Subscribe with a filter
    engine.register_fn("subscribe_event_filtered", |event_type: String, handler_fn: String, filter_type: String, filter_value: String| -> String {
        let ctx = get_context();

        let event_types = match parse_event_type(&event_type) {
            Some(et) => vec![et],
            None => return String::new(),
        };

        let filter = match filter_type.to_lowercase().as_str() {
            "sector" | "sector_id" => {
                Uuid::parse_str(&filter_value)
                    .ok()
                    .map(EventFilter::SectorId)
            }
            "actor" | "actor_id" => {
                Uuid::parse_str(&filter_value)
                    .ok()
                    .map(EventFilter::ActorId)
            }
            "target" | "target_id" => {
                Uuid::parse_str(&filter_value)
                    .ok()
                    .map(EventFilter::TargetId)
            }
            "custom" => Some(EventFilter::Custom(filter_value)),
            _ => None,
        };

        with_registry(|registry| {
            let mut sub = EventSubscription::new(
                ctx.script_path.clone(),
                handler_fn,
                event_types,
            );

            if let Some(entity_id) = ctx.owner_entity_id {
                sub = sub.with_owner(entity_id);
            }
            if let Some(sector_id) = ctx.sector_id {
                sub = sub.with_sector(sector_id);
            }
            if let Some(f) = filter {
                sub = sub.with_filter(f);
            }

            let id = registry.subscribe(sub);
            id.to_string()
        }).unwrap_or_default()
    });

    // unsubscribe_event(subscription_id: String) -> bool
    // Unsubscribe by ID
    engine.register_fn("unsubscribe_event", |subscription_id: String| -> bool {
        let id = match Uuid::parse_str(&subscription_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        with_registry(|registry| {
            registry.unsubscribe(id)
        }).unwrap_or(false)
    });

    // unsubscribe_all() -> i64
    // Unsubscribe all subscriptions for the current entity, returns count
    engine.register_fn("unsubscribe_all", || -> i64 {
        let ctx = get_context();

        let entity_id = match ctx.owner_entity_id {
            Some(id) => id,
            None => return 0,
        };

        with_registry(|registry| {
            let count = registry.count_for_entity(entity_id) as i64;
            registry.unsubscribe_for_entity(entity_id);
            count
        }).unwrap_or(0)
    });

    // enable_subscription(subscription_id: String) -> bool
    engine.register_fn("enable_subscription", |subscription_id: String| -> bool {
        let id = match Uuid::parse_str(&subscription_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        with_registry(|registry| {
            registry.set_enabled(id, true);
            true
        }).unwrap_or(false)
    });

    // disable_subscription(subscription_id: String) -> bool
    engine.register_fn("disable_subscription", |subscription_id: String| -> bool {
        let id = match Uuid::parse_str(&subscription_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        with_registry(|registry| {
            registry.set_enabled(id, false);
            true
        }).unwrap_or(false)
    });

    // === Utility functions ===

    // is_valid_event_type(event_type: String) -> bool
    engine.register_fn("is_valid_event_type", |event_type: String| -> bool {
        parse_event_type(&event_type).is_some()
    });

    // get_subscription_count() -> i64
    // Get count of subscriptions for current entity
    engine.register_fn("get_subscription_count", || -> i64 {
        let ctx = get_context();

        match ctx.owner_entity_id {
            Some(entity_id) => {
                with_registry(|registry| {
                    registry.count_for_entity(entity_id) as i64
                }).unwrap_or(0)
            }
            None => 0,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::context::{ScriptExecutionContext, ExecutionGuard};
    use crate::state::{StateAccessor, StateProvider, ShipSnapshot, PlayerSnapshot, SectorSnapshot, StateMutation, MutationResult};
    use bw_core::models::Position;

    // Mock provider for tests
    struct MockProvider;
    impl StateProvider for MockProvider {
        fn get_ship(&self, _: Uuid) -> Option<ShipSnapshot> { None }
        fn get_ships_in_sector(&self, _: Uuid) -> Vec<ShipSnapshot> { vec![] }
        fn get_ships_in_range(&self, _: Uuid, _: Position, _: f64) -> Vec<ShipSnapshot> { vec![] }
        fn get_player(&self, _: Uuid) -> Option<PlayerSnapshot> { None }
        fn get_sector(&self, _: Uuid) -> Option<SectorSnapshot> { None }
        fn apply_mutations(&self, m: Vec<StateMutation>) -> Vec<MutationResult> {
            m.into_iter().map(MutationResult::success).collect()
        }
    }

    #[test]
    fn test_script_context_via_execution_guard() {
        let accessor = Arc::new(StateAccessor::new(Arc::new(MockProvider)));
        let entity_id = Uuid::new_v4();

        let ctx = ScriptExecutionContext::new(accessor)
            .with_script_path("test.rhai")
            .with_owner_entity(entity_id);

        let _guard = ExecutionGuard::enter(ctx).expect("Failed to enter execution context");

        // get_context() should retrieve from unified context
        let retrieved = get_context();
        assert_eq!(retrieved.script_path, "test.rhai");
        assert_eq!(retrieved.owner_entity_id, Some(entity_id));

        // Guard drops here and context is cleared
    }

    #[test]
    fn test_context_cleared_after_guard_drop() {
        let accessor = Arc::new(StateAccessor::new(Arc::new(MockProvider)));

        {
            let ctx = ScriptExecutionContext::new(accessor)
                .with_script_path("test.rhai");
            let _guard = ExecutionGuard::enter(ctx).expect("Failed to enter execution context");
            assert_eq!(get_context().script_path, "test.rhai");
        }

        // After guard drops, context should be empty
        let cleared = get_context();
        assert!(cleared.script_path.is_empty());
    }
}
