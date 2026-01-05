//! Event API bindings for Rhai
//!
//! Exposes event subscription and emission functions to scripts.
//! Uses thread-local context for the current script's information.
//!
//! # Thread Safety
//!
//! Similar to state_api, these thread-locals are safe because:
//! 1. Rhai script execution is synchronous
//! 2. The state_api guard prevents re-entrant execution
//! 3. Callers must hold appropriate locks before executing scripts

use std::cell::RefCell;
use std::sync::Arc;
use rhai::Engine;
use uuid::Uuid;

use bw_core::events::GameEventType;
use crate::events::{EventRegistry, EventSubscription, EventFilter, parse_event_type};

thread_local! {
    /// Current event registry for subscriptions
    static CURRENT_REGISTRY: RefCell<Option<Arc<EventRegistry>>> = const { RefCell::new(None) };
    /// Current script context (script path, owner entity, sector)
    static CURRENT_CONTEXT: RefCell<ScriptContext> = const { RefCell::new(ScriptContext::empty()) };
}

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

/// Set the event registry for the current thread.
pub fn set_current_registry(registry: Arc<EventRegistry>) {
    CURRENT_REGISTRY.with(|cell| {
        *cell.borrow_mut() = Some(registry);
    });
}

/// Clear the event registry.
pub fn clear_current_registry() {
    CURRENT_REGISTRY.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Set the script context for the current thread.
pub fn set_script_context(ctx: ScriptContext) {
    CURRENT_CONTEXT.with(|cell| {
        *cell.borrow_mut() = ctx;
    });
}

/// Clear the script context.
pub fn clear_script_context() {
    CURRENT_CONTEXT.with(|cell| {
        *cell.borrow_mut() = ScriptContext::default();
    });
}

/// Get current script context.
fn get_context() -> ScriptContext {
    CURRENT_CONTEXT.with(|cell| cell.borrow().clone())
}

/// Access registry.
fn with_registry<T, F: FnOnce(&EventRegistry) -> T>(f: F) -> Option<T> {
    CURRENT_REGISTRY.with(|cell| {
        cell.borrow().as_ref().map(|reg| f(reg))
    })
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

    #[test]
    fn test_script_context() {
        let ctx = ScriptContext {
            script_path: "test.rhai".to_string(),
            owner_entity_id: Some(Uuid::new_v4()),
            sector_id: None,
        };

        set_script_context(ctx.clone());
        let retrieved = get_context();
        assert_eq!(retrieved.script_path, "test.rhai");

        clear_script_context();
        let cleared = get_context();
        assert!(cleared.script_path.is_empty());
    }
}
