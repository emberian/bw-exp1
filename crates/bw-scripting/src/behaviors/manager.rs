//! Behavior manager
//!
//! Manages the lifecycle of entity behaviors.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Dynamic, Map};
use uuid::Uuid;

use bw_core::events::GameEvent;
use crate::engine::{ScriptEngine, ScriptError};
use crate::state::{StateAccessor, AccessPermissions};
use crate::events::EventRegistry;
use crate::bindings::state_api::{set_current_accessor, clear_current_accessor};
use crate::bindings::event_api::{set_current_registry, clear_current_registry, set_script_context, clear_script_context, ScriptContext};

use super::{EntityBehavior, EntityType, BehaviorState, BehaviorContext, BehaviorExecResult};

/// Manages entity behaviors.
pub struct BehaviorManager {
    /// All active behaviors
    behaviors: RwLock<HashMap<Uuid, EntityBehavior>>,
    /// Behaviors indexed by entity ID
    by_entity: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Script engine for execution
    engine: Arc<ScriptEngine>,
    /// State accessor for scripts
    state_accessor: Option<Arc<StateAccessor>>,
    /// Event registry for subscriptions
    event_registry: Option<Arc<EventRegistry>>,
    /// Current game tick
    current_tick: RwLock<u64>,
}

impl BehaviorManager {
    /// Create a new behavior manager.
    pub fn new(engine: Arc<ScriptEngine>) -> Self {
        Self {
            behaviors: RwLock::new(HashMap::new()),
            by_entity: RwLock::new(HashMap::new()),
            engine,
            state_accessor: None,
            event_registry: None,
            current_tick: RwLock::new(0),
        }
    }

    /// Set the state accessor.
    pub fn set_state_accessor(&mut self, accessor: Arc<StateAccessor>) {
        self.state_accessor = Some(accessor);
    }

    /// Set the event registry.
    pub fn set_event_registry(&mut self, registry: Arc<EventRegistry>) {
        self.event_registry = Some(registry);
    }

    /// Attach a behavior to an entity.
    pub fn attach(
        &self,
        entity_id: Uuid,
        entity_type: EntityType,
        script_path: &str,
        sector_id: Option<Uuid>,
    ) -> Result<Uuid, ScriptError> {
        // Verify script exists
        if !self.engine.has_script(script_path) {
            return Err(ScriptError::NotFound(script_path.to_string()));
        }

        let mut behavior = EntityBehavior::new(entity_id, entity_type, script_path);
        if let Some(sid) = sector_id {
            behavior = behavior.with_sector(sid);
        }

        let id = behavior.id;

        // Add to maps
        self.behaviors.write().insert(id, behavior.clone());
        self.by_entity.write()
            .entry(entity_id)
            .or_default()
            .push(id);

        // Call on_spawn
        let tick = *self.current_tick.read();
        let result = self.call_hook(id, "on_spawn", tick, 0.0);

        if result.success {
            // Update state to active
            if let Some(behavior) = self.behaviors.write().get_mut(&id) {
                behavior.state = BehaviorState::Active;
                if let Some(data) = result.updated_local_data {
                    behavior.local_data = data;
                }
            }

            tracing::debug!(
                behavior_id = %id,
                entity_id = %entity_id,
                script = script_path,
                "Behavior attached"
            );
        } else {
            // Remove on failure
            self.behaviors.write().remove(&id);
            if let Some(ids) = self.by_entity.write().get_mut(&entity_id) {
                ids.retain(|i| *i != id);
            }

            return Err(ScriptError::RuntimeError(
                result.error.unwrap_or_else(|| "on_spawn failed".to_string())
            ));
        }

        Ok(id)
    }

    /// Detach a behavior by ID.
    pub fn detach(&self, behavior_id: Uuid) -> bool {
        let behavior = self.behaviors.write().remove(&behavior_id);

        if let Some(mut behavior) = behavior {
            // Remove from entity index
            if let Some(ids) = self.by_entity.write().get_mut(&behavior.entity_id) {
                ids.retain(|id| *id != behavior_id);
            }

            // Call on_destroy
            behavior.state = BehaviorState::Destroying;
            let tick = *self.current_tick.read();
            let _ = self.call_hook_for_behavior(&behavior, "on_destroy", tick, 0.0);

            // Clean up event subscriptions
            if let Some(ref registry) = self.event_registry {
                registry.unsubscribe_for_entity(behavior.entity_id);
            }

            tracing::debug!(behavior_id = %behavior_id, "Behavior detached");
            true
        } else {
            false
        }
    }

    /// Detach all behaviors for an entity.
    pub fn detach_for_entity(&self, entity_id: Uuid) {
        let behavior_ids: Vec<Uuid> = self.by_entity.read()
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        for id in behavior_ids {
            self.detach(id);
        }

        self.by_entity.write().remove(&entity_id);
    }

    /// Update all active behaviors.
    pub fn update_all(&self, tick: u64, delta_time: f64) -> Vec<BehaviorExecResult> {
        *self.current_tick.write() = tick;

        let behavior_ids: Vec<Uuid> = self.behaviors.read()
            .iter()
            .filter(|(_, b)| b.state == BehaviorState::Active)
            .map(|(id, _)| *id)
            .collect();

        let mut results = Vec::new();

        for id in behavior_ids {
            let result = self.call_hook(id, "on_update", tick, delta_time);

            // Update local data if successful
            if result.success {
                if let Some(data) = &result.updated_local_data {
                    if let Some(behavior) = self.behaviors.write().get_mut(&id) {
                        behavior.local_data = data.clone();
                    }
                }
            }

            results.push(result);
        }

        results
    }

    /// Notify behaviors of an event.
    pub fn notify_event(&self, event: &GameEvent) -> Vec<BehaviorExecResult> {
        let tick = *self.current_tick.read();

        // Find behaviors that might handle this event
        // (behaviors in the same sector as the event)
        let behavior_ids: Vec<(Uuid, EntityBehavior)> = self.behaviors.read()
            .iter()
            .filter(|(_, b)| {
                b.state == BehaviorState::Active
                    && b.sector_id.map(|s| s == event.sector_id).unwrap_or(true)
            })
            .map(|(id, b)| (*id, b.clone()))
            .collect();

        let mut results = Vec::new();
        let event_data = event_to_dynamic(event);

        for (id, behavior) in behavior_ids {
            // Check if the script has on_event function
            // (we call it anyway and let it fail silently if not present)
            let result = self.call_event_hook(&behavior, event_data.clone(), tick);

            if result.success {
                if let Some(data) = &result.updated_local_data {
                    if let Some(b) = self.behaviors.write().get_mut(&id) {
                        b.local_data = data.clone();
                    }
                }
            }

            results.push(result);
        }

        results
    }

    /// Pause a behavior.
    pub fn pause(&self, behavior_id: Uuid) -> bool {
        if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
            if behavior.state == BehaviorState::Active {
                behavior.state = BehaviorState::Paused;
                return true;
            }
        }
        false
    }

    /// Resume a paused behavior.
    pub fn resume(&self, behavior_id: Uuid) -> bool {
        if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
            if behavior.state == BehaviorState::Paused {
                behavior.state = BehaviorState::Active;
                return true;
            }
        }
        false
    }

    /// Get behaviors for an entity.
    pub fn get_behaviors(&self, entity_id: Uuid) -> Vec<EntityBehavior> {
        let ids = self.by_entity.read()
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        let behaviors = self.behaviors.read();
        ids.iter()
            .filter_map(|id| behaviors.get(id).cloned())
            .collect()
    }

    /// Get a behavior by ID.
    pub fn get(&self, behavior_id: Uuid) -> Option<EntityBehavior> {
        self.behaviors.read().get(&behavior_id).cloned()
    }

    /// Get count of active behaviors.
    pub fn active_count(&self) -> usize {
        self.behaviors.read()
            .values()
            .filter(|b| b.state == BehaviorState::Active)
            .count()
    }

    /// Get count of behaviors for an entity.
    pub fn count_for_entity(&self, entity_id: Uuid) -> usize {
        self.by_entity.read()
            .get(&entity_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// List all behaviors.
    pub fn list_all(&self) -> Vec<EntityBehavior> {
        self.behaviors.read().values().cloned().collect()
    }

    /// Set local data for a behavior.
    pub fn set_local_data(&self, behavior_id: Uuid, data: Map) -> bool {
        if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
            behavior.local_data = data;
            true
        } else {
            false
        }
    }

    // === Private helpers ===

    fn call_hook(&self, behavior_id: Uuid, hook_name: &str, tick: u64, delta_time: f64) -> BehaviorExecResult {
        let behavior = match self.behaviors.read().get(&behavior_id).cloned() {
            Some(b) => b,
            None => return BehaviorExecResult::failure(behavior_id, "Behavior not found"),
        };

        self.call_hook_for_behavior(&behavior, hook_name, tick, delta_time)
    }

    fn call_hook_for_behavior(
        &self,
        behavior: &EntityBehavior,
        hook_name: &str,
        tick: u64,
        delta_time: f64,
    ) -> BehaviorExecResult {
        // Set up context
        let ctx = BehaviorContext {
            behavior_id: behavior.id,
            entity_id: behavior.entity_id,
            entity_type: behavior.entity_type,
            sector_id: behavior.sector_id.unwrap_or_else(Uuid::nil),
            tick,
            delta_time,
            local_data: behavior.local_data.clone(),
        };

        // Set up state accessor
        if let Some(ref accessor) = self.state_accessor {
            let perms = AccessPermissions::npc_behavior(
                behavior.entity_id,
                behavior.sector_id.unwrap_or_else(Uuid::nil),
            );
            accessor.set_permissions(perms);
            if let Some(sector_id) = behavior.sector_id {
                accessor.set_context_sector(sector_id);
            }
            set_current_accessor(accessor.clone());
        }

        // Set up event registry
        if let Some(ref registry) = self.event_registry {
            set_current_registry(registry.clone());
            set_script_context(ScriptContext {
                script_path: behavior.script_path.clone(),
                owner_entity_id: Some(behavior.entity_id),
                sector_id: behavior.sector_id,
            });
        }

        // Call the hook
        let result = self.engine.call_function_dynamic(
            &behavior.script_path,
            hook_name,
            (ctx.to_dynamic(),),
        );

        // Clear contexts
        clear_current_accessor();
        clear_current_registry();
        clear_script_context();

        // Apply mutations
        if let Some(ref accessor) = self.state_accessor {
            let _ = accessor.apply_pending_mutations();
        }

        match result {
            Ok(return_val) => {
                // Check if script returned updated context
                let updated_data = return_val.try_cast::<Map>()
                    .and_then(|m| m.get("local_data").cloned())
                    .and_then(|d| d.try_cast::<Map>());

                if let Some(data) = updated_data {
                    BehaviorExecResult::success_with_data(behavior.id, data)
                } else {
                    BehaviorExecResult::success(behavior.id)
                }
            }
            Err(e) => {
                // Function not found is not an error for optional hooks
                if e.to_string().contains("Function not found") {
                    BehaviorExecResult::success(behavior.id)
                } else {
                    tracing::warn!(
                        behavior_id = %behavior.id,
                        hook = hook_name,
                        error = %e,
                        "Behavior hook failed"
                    );
                    BehaviorExecResult::failure(behavior.id, e.to_string())
                }
            }
        }
    }

    fn call_event_hook(
        &self,
        behavior: &EntityBehavior,
        event_data: Dynamic,
        tick: u64,
    ) -> BehaviorExecResult {
        // Set up context
        let ctx = BehaviorContext {
            behavior_id: behavior.id,
            entity_id: behavior.entity_id,
            entity_type: behavior.entity_type,
            sector_id: behavior.sector_id.unwrap_or_else(Uuid::nil),
            tick,
            delta_time: 0.0,
            local_data: behavior.local_data.clone(),
        };

        // Set up state accessor
        if let Some(ref accessor) = self.state_accessor {
            let perms = AccessPermissions::npc_behavior(
                behavior.entity_id,
                behavior.sector_id.unwrap_or_else(Uuid::nil),
            );
            accessor.set_permissions(perms);
            if let Some(sector_id) = behavior.sector_id {
                accessor.set_context_sector(sector_id);
            }
            set_current_accessor(accessor.clone());
        }

        // Set up event registry
        if let Some(ref registry) = self.event_registry {
            set_current_registry(registry.clone());
            set_script_context(ScriptContext {
                script_path: behavior.script_path.clone(),
                owner_entity_id: Some(behavior.entity_id),
                sector_id: behavior.sector_id,
            });
        }

        // Call on_event
        let result = self.engine.call_function_dynamic(
            &behavior.script_path,
            "on_event",
            (ctx.to_dynamic(), event_data),
        );

        // Clear contexts
        clear_current_accessor();
        clear_current_registry();
        clear_script_context();

        // Apply mutations
        if let Some(ref accessor) = self.state_accessor {
            let _ = accessor.apply_pending_mutations();
        }

        match result {
            Ok(return_val) => {
                let updated_data = return_val.try_cast::<Map>()
                    .and_then(|m| m.get("local_data").cloned())
                    .and_then(|d| d.try_cast::<Map>());

                if let Some(data) = updated_data {
                    BehaviorExecResult::success_with_data(behavior.id, data)
                } else {
                    BehaviorExecResult::success(behavior.id)
                }
            }
            Err(e) => {
                // on_event not found is fine
                if e.to_string().contains("Function not found") {
                    BehaviorExecResult::success(behavior.id)
                } else {
                    BehaviorExecResult::failure(behavior.id, e.to_string())
                }
            }
        }
    }
}

/// Convert a GameEvent to Rhai Dynamic.
fn event_to_dynamic(event: &GameEvent) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), event.id.to_string().into());
    map.insert("event_type".into(), format!("{:?}", event.event_type).into());
    map.insert("sector_id".into(), event.sector_id.to_string().into());
    map.insert("actor_id".into(),
        event.actor_id.map(|id| id.to_string()).unwrap_or_default().into());
    map.insert("target_id".into(),
        event.target_id.map(|id| id.to_string()).unwrap_or_default().into());
    Dynamic::from(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_type() {
        assert_eq!(EntityType::Ship.as_str(), "ship");
        assert_eq!(EntityType::Station.as_str(), "station");
    }
}
