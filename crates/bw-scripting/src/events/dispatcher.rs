//! Event dispatcher
//!
//! Dispatches game events to subscribed script handlers.

use std::sync::Arc;
use rhai::{Dynamic, Map};
use uuid::Uuid;

use bw_core::events::{GameEvent, GameEventType};
use crate::engine::ScriptEngine;
use bw_game::state::{StateAccessor, AccessPermissions, StateMutation};
use crate::context::{ScriptExecutionContext, ExecutionGuard};

use super::{EventRegistry, EventSubscription};

/// Result from dispatching an event.
#[derive(Debug)]
pub struct DispatchResult {
    /// Event that was dispatched
    pub event_type: GameEventType,
    /// Number of handlers that were called
    pub handlers_called: usize,
    /// Handlers that succeeded
    pub successes: usize,
    /// Handlers that failed
    pub failures: Vec<(Uuid, String)>,
    /// State mutations collected from handlers
    pub mutations: Vec<StateMutation>,
}

impl DispatchResult {
    fn new(event_type: GameEventType) -> Self {
        Self {
            event_type,
            handlers_called: 0,
            successes: 0,
            failures: Vec::new(),
            mutations: Vec::new(),
        }
    }
}

/// Dispatches events to script handlers.
pub struct EventDispatcher {
    registry: Arc<EventRegistry>,
    engine: Arc<ScriptEngine>,
    state_accessor: Option<Arc<StateAccessor>>,
}

impl EventDispatcher {
    /// Create a new dispatcher.
    pub fn new(registry: Arc<EventRegistry>, engine: Arc<ScriptEngine>) -> Self {
        Self {
            registry,
            engine,
            state_accessor: None,
        }
    }

    /// Set the state accessor for handler execution.
    pub fn set_state_accessor(&mut self, accessor: Arc<StateAccessor>) {
        self.state_accessor = Some(accessor);
    }

    /// Dispatch an event to all subscribed handlers.
    pub fn dispatch(&self, event: &GameEvent) -> DispatchResult {
        let mut result = DispatchResult::new(event.event_type);

        // Get handlers sorted by priority
        let handlers = self.registry.get_handlers_sorted(event.event_type);

        if handlers.is_empty() {
            return result;
        }

        // Convert event to Rhai map
        let event_data = event_to_dynamic(event);

        for handler in handlers {
            if !handler.enabled {
                continue;
            }

            // Check sector filter
            if let Some(sector_filter) = handler.sector_id
                && event.sector_id != sector_filter {
                    continue;
                }

            // Check custom filter
            if let Some(ref filter) = handler.filter
                && !filter.matches(event.sector_id, event.actor_id, event.target_id) {
                    continue;
                }

            result.handlers_called += 1;

            // Execute handler
            match self.execute_handler(&handler, event_data.clone()) {
                Ok(mutations) => {
                    result.successes += 1;
                    result.mutations.extend(mutations);
                }
                Err(e) => {
                    result.failures.push((handler.id, e));
                }
            }
        }

        if result.handlers_called > 0 {
            tracing::debug!(
                event_type = ?event.event_type,
                handlers = result.handlers_called,
                successes = result.successes,
                failures = result.failures.len(),
                "Event dispatched"
            );
        }

        result
    }

    /// Dispatch a simple event (no full GameEvent struct).
    pub fn dispatch_simple(
        &self,
        event_type: GameEventType,
        sector_id: Uuid,
        actor_id: Option<Uuid>,
        target_id: Option<Uuid>,
        data: Map,
    ) -> DispatchResult {
        let event = GameEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            event_type,
            sector_id,
            actor_id,
            target_id,
            data: serde_json::to_value(&data).unwrap_or_default(),
        };

        self.dispatch(&event)
    }

    fn execute_handler(
        &self,
        handler: &EventSubscription,
        event_data: Dynamic,
    ) -> Result<Vec<StateMutation>, String> {
        // Check script exists
        if !self.engine.has_script(&handler.script_path) {
            return Err(format!("Script not found: {}", handler.script_path));
        }

        // Set up execution context if state accessor is available
        let _guard = if let Some(ref accessor) = self.state_accessor {
            let perms = if let Some(entity_id) = handler.owner_entity_id {
                if let Some(sector_id) = handler.sector_id {
                    AccessPermissions::npc_behavior(entity_id, sector_id)
                } else {
                    AccessPermissions::trusted()
                }
            } else {
                AccessPermissions::trusted()
            };

            accessor.set_permissions(perms);
            if let Some(sector_id) = handler.sector_id {
                accessor.set_context_sector(sector_id);
            }

            // Build unified execution context
            let mut exec_ctx = ScriptExecutionContext::new(accessor.clone())
                .with_script_path(&handler.script_path)
                .with_event_registry(self.registry.clone());

            if let Some(entity_id) = handler.owner_entity_id {
                exec_ctx = exec_ctx.with_owner_entity(entity_id);
            }
            if let Some(sector_id) = handler.sector_id {
                exec_ctx = exec_ctx.with_sector(sector_id);
            }

            // Enter execution context (RAII guard handles cleanup)
            match ExecutionGuard::enter(exec_ctx) {
                Ok(guard) => Some(guard),
                Err(e) => {
                    tracing::warn!("Failed to enter execution context for event handler: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Build context map for handler
        let mut ctx = Map::new();
        if let Some(entity_id) = handler.owner_entity_id {
            ctx.insert("entity_id".into(), entity_id.to_string().into());
        }
        if let Some(sector_id) = handler.sector_id {
            ctx.insert("sector_id".into(), sector_id.to_string().into());
        }
        ctx.insert("subscription_id".into(), handler.id.to_string().into());

        // Call the handler function
        let result = self.engine.call_function_dynamic(
            &handler.script_path,
            &handler.handler_function,
            (Dynamic::from(ctx), event_data),
        );

        // Collect mutations before guard drops
        let mutations = if let Some(ref accessor) = self.state_accessor {
            accessor.take_mutations()
        } else {
            Vec::new()
        };

        // Guard drops here automatically

        match result {
            Ok(_) => Ok(mutations),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Convert a GameEvent to a Rhai Dynamic map.
fn event_to_dynamic(event: &GameEvent) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), event.id.to_string().into());
    map.insert("event_type".into(), format!("{:?}", event.event_type).into());
    map.insert("sector_id".into(), event.sector_id.to_string().into());
    map.insert("actor_id".into(),
        event.actor_id.map(|id| id.to_string()).unwrap_or_default().into());
    map.insert("target_id".into(),
        event.target_id.map(|id| id.to_string()).unwrap_or_default().into());

    // Convert JSON data to Rhai map
    if let Some(data_obj) = event.data.as_object() {
        let mut data_map = Map::new();
        for (key, value) in data_obj {
            data_map.insert(key.clone().into(), json_to_dynamic(value));
        }
        map.insert("data".into(), Dynamic::from(data_map));
    } else {
        map.insert("data".into(), Dynamic::from(Map::new()));
    }

    Dynamic::from(map)
}

/// Convert a serde_json::Value to Rhai Dynamic.
fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let vec: Vec<Dynamic> = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(vec)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_to_dynamic() {
        let event = GameEvent::new(GameEventType::ShipSpawned, Uuid::new_v4());
        let dynamic = event_to_dynamic(&event);

        assert!(dynamic.is::<Map>());
        let map = dynamic.cast::<Map>();
        assert!(map.contains_key("id"));
        assert!(map.contains_key("event_type"));
    }
}
