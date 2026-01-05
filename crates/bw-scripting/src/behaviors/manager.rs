//! Behavior manager
//!
//! Manages the lifecycle of entity behaviors, including behavior tree execution.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Dynamic, Map};
use uuid::Uuid;

use bw_core::events::GameEvent;
use crate::engine::{ScriptEngine, ScriptError};
use crate::state::{StateAccessor, AccessPermissions};
use crate::events::EventRegistry;
use crate::context::{ScriptExecutionContext, ExecutionGuard};
use crate::ai::{BehaviorTreeRunner, AiContext, BtNode};
use crate::persistence::{ScriptStateStore, PersistenceError};

use super::{EntityBehavior, EntityType, BehaviorState, BehaviorContext, BehaviorExecResult};

/// Manages entity behaviors.
pub struct BehaviorManager {
    /// All active behaviors
    behaviors: RwLock<HashMap<Uuid, EntityBehavior>>,
    /// Behaviors indexed by entity ID
    by_entity: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Script engine for execution
    engine: Arc<ScriptEngine>,
    /// Behavior tree runner
    bt_runner: BehaviorTreeRunner,
    /// State accessor for scripts
    state_accessor: Option<Arc<StateAccessor>>,
    /// Event registry for subscriptions
    event_registry: Option<Arc<EventRegistry>>,
    /// Persistent state store for behavior data
    state_store: Option<Arc<dyn ScriptStateStore>>,
    /// Current game tick
    current_tick: RwLock<u64>,
    /// Current game time in seconds
    current_game_time: RwLock<f64>,
}

impl BehaviorManager {
    /// Create a new behavior manager.
    pub fn new(engine: Arc<ScriptEngine>) -> Self {
        Self {
            behaviors: RwLock::new(HashMap::new()),
            by_entity: RwLock::new(HashMap::new()),
            engine,
            bt_runner: BehaviorTreeRunner::new(),
            state_accessor: None,
            event_registry: None,
            state_store: None,
            current_tick: RwLock::new(0),
            current_game_time: RwLock::new(0.0),
        }
    }

    /// Set the state store for persistent behavior data.
    pub fn set_state_store(&mut self, store: Arc<dyn ScriptStateStore>) {
        self.state_store = Some(store);
    }

    /// Get a reference to the behavior tree runner.
    pub fn bt_runner(&self) -> &BehaviorTreeRunner {
        &self.bt_runner
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

            return Err(ScriptError::runtime(
                script_path,
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

        // Update game time
        {
            let mut game_time = self.current_game_time.write();
            *game_time += delta_time;
        }

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

        // Run behavior trees for entities that have them
        self.tick_behavior_trees(tick, delta_time);

        results
    }

    /// Tick all behavior trees.
    fn tick_behavior_trees(&self, tick: u64, delta_time: f64) {
        let game_time = *self.current_game_time.read();

        let behaviors: Vec<(Uuid, EntityBehavior)> = self.behaviors.read()
            .iter()
            .filter(|(_, b)| b.state == BehaviorState::Active && b.behavior_tree.is_some())
            .map(|(id, b)| (*id, b.clone()))
            .collect();

        let rhai_engine = self.engine.rhai_engine();

        for (behavior_id, behavior) in behaviors {
            if let Some(ref tree) = behavior.behavior_tree {
                // Create AI context
                let mut ctx = AiContext::new(
                    behavior.entity_id,
                    tick,
                    delta_time,
                    game_time,
                );

                // Add local_data to context
                ctx.data = behavior.local_data.clone();

                // Run the behavior tree
                let result = self.bt_runner.run(rhai_engine, tree, &ctx, &behavior.script_path);

                if let Some(error) = result.error {
                    tracing::warn!(
                        behavior_id = %behavior_id,
                        entity_id = %behavior.entity_id,
                        error = %error,
                        "Behavior tree error"
                    );
                }

                // Log if debug tracing is enabled
                if !result.trace.is_empty() {
                    tracing::debug!(
                        behavior_id = %behavior_id,
                        trace = ?result.trace,
                        status = ?result.status,
                        "Behavior tree executed"
                    );
                }
            }
        }
    }

    /// Set a behavior tree for an entity's behavior.
    pub fn set_behavior_tree(&self, behavior_id: Uuid, tree: BtNode) -> bool {
        if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
            behavior.behavior_tree = Some(tree);
            true
        } else {
            false
        }
    }

    /// Clear a behavior tree for an entity's behavior.
    pub fn clear_behavior_tree(&self, behavior_id: Uuid) -> bool {
        if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
            behavior.behavior_tree = None;
            self.bt_runner.clear_entity_state(behavior.entity_id);
            true
        } else {
            false
        }
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

    // === Persistence ===

    /// Persist a behavior's local_data to the state store.
    ///
    /// This saves the behavior's current local_data so it can be restored
    /// after a server restart.
    pub fn persist_behavior(&self, entity_id: Uuid) -> Result<(), PersistenceError> {
        let Some(store) = &self.state_store else {
            return Ok(()); // No store configured, silently skip
        };

        let behaviors = self.behaviors.read();
        let entity_behaviors: Vec<_> = behaviors.values()
            .filter(|b| b.entity_id == entity_id && b.state == BehaviorState::Active)
            .collect();

        for behavior in entity_behaviors {
            // Serialize local_data as JSON
            let data = serde_json::to_vec(&behavior.local_data)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

            // Use "behavior" namespace with entity_id as key
            store.save("behavior", &entity_id.to_string(), &data)?;

            tracing::debug!(
                entity_id = %entity_id,
                behavior_id = %behavior.id,
                "Persisted behavior state"
            );
        }

        Ok(())
    }

    /// Persist all active behaviors to the state store.
    pub fn persist_all(&self) -> Result<usize, PersistenceError> {
        let Some(store) = &self.state_store else {
            return Ok(0);
        };

        let behaviors = self.behaviors.read();
        let mut count = 0;

        for behavior in behaviors.values().filter(|b| b.state == BehaviorState::Active) {
            let data = serde_json::to_vec(&behavior.local_data)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

            store.save("behavior", &behavior.entity_id.to_string(), &data)?;
            count += 1;
        }

        tracing::info!(count = count, "Persisted all behavior states");
        Ok(count)
    }

    /// Restore a behavior's local_data from the state store.
    ///
    /// Call this after attaching a behavior to restore its previous state.
    pub fn restore_behavior(&self, entity_id: Uuid) -> Result<bool, PersistenceError> {
        let Some(store) = &self.state_store else {
            return Ok(false);
        };

        let Some(data) = store.load("behavior", &entity_id.to_string())? else {
            return Ok(false); // No saved state
        };

        // Deserialize local_data
        let local_data: Map = serde_json::from_slice(&data)
            .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        // Apply to the behavior
        let mut behaviors = self.behaviors.write();
        for behavior in behaviors.values_mut() {
            if behavior.entity_id == entity_id && behavior.state == BehaviorState::Active {
                behavior.local_data = local_data.clone();
                tracing::debug!(
                    entity_id = %entity_id,
                    behavior_id = %behavior.id,
                    "Restored behavior state"
                );
            }
        }

        Ok(true)
    }

    /// Clear persisted state for an entity.
    pub fn clear_persisted_state(&self, entity_id: Uuid) -> Result<bool, PersistenceError> {
        let Some(store) = &self.state_store else {
            return Ok(false);
        };

        store.delete("behavior", &entity_id.to_string())
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
        // Set up behavior context for the script
        let ctx = BehaviorContext {
            behavior_id: behavior.id,
            entity_id: behavior.entity_id,
            entity_type: behavior.entity_type,
            sector_id: behavior.sector_id.unwrap_or_else(Uuid::nil),
            tick,
            delta_time,
            local_data: behavior.local_data.clone(),
        };

        // Set up unified execution context
        let Some(ref accessor) = self.state_accessor else {
            // No accessor - can't execute scripts safely
            return BehaviorExecResult::failure(behavior.id, "No state accessor configured".to_string());
        };

        // Configure accessor permissions
        let perms = AccessPermissions::npc_behavior(
            behavior.entity_id,
            behavior.sector_id.unwrap_or_else(Uuid::nil),
        );
        accessor.set_permissions(perms);
        if let Some(sector_id) = behavior.sector_id {
            accessor.set_context_sector(sector_id);
        }

        // Build execution context
        let mut exec_ctx = ScriptExecutionContext::new(accessor.clone())
            .with_script_path(&behavior.script_path)
            .with_owner_entity(behavior.entity_id)
            .with_tick(tick);

        if let Some(sector_id) = behavior.sector_id {
            exec_ctx = exec_ctx.with_sector(sector_id);
        }
        if let Some(ref registry) = self.event_registry {
            exec_ctx = exec_ctx.with_event_registry(registry.clone());
        }
        if let Some(ref store) = self.state_store {
            exec_ctx = exec_ctx.with_persistence_store(store.clone());
        }

        // Enter execution context (RAII guard handles cleanup and mutation application)
        let _guard = match ExecutionGuard::enter(exec_ctx) {
            Ok(g) => g,
            Err(e) => {
                return BehaviorExecResult::failure(behavior.id, e.to_string());
            }
        };

        // Call the hook
        let result = self.engine.call_function_dynamic(
            &behavior.script_path,
            hook_name,
            (ctx.to_dynamic(),),
        );

        // Guard drop applies mutations automatically

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
        // Set up behavior context for the script
        let ctx = BehaviorContext {
            behavior_id: behavior.id,
            entity_id: behavior.entity_id,
            entity_type: behavior.entity_type,
            sector_id: behavior.sector_id.unwrap_or_else(Uuid::nil),
            tick,
            delta_time: 0.0,
            local_data: behavior.local_data.clone(),
        };

        // Set up unified execution context
        let Some(ref accessor) = self.state_accessor else {
            return BehaviorExecResult::failure(behavior.id, "No state accessor configured".to_string());
        };

        // Configure accessor permissions
        let perms = AccessPermissions::npc_behavior(
            behavior.entity_id,
            behavior.sector_id.unwrap_or_else(Uuid::nil),
        );
        accessor.set_permissions(perms);
        if let Some(sector_id) = behavior.sector_id {
            accessor.set_context_sector(sector_id);
        }

        // Build execution context
        let mut exec_ctx = ScriptExecutionContext::new(accessor.clone())
            .with_script_path(&behavior.script_path)
            .with_owner_entity(behavior.entity_id)
            .with_tick(tick);

        if let Some(sector_id) = behavior.sector_id {
            exec_ctx = exec_ctx.with_sector(sector_id);
        }
        if let Some(ref registry) = self.event_registry {
            exec_ctx = exec_ctx.with_event_registry(registry.clone());
        }
        if let Some(ref store) = self.state_store {
            exec_ctx = exec_ctx.with_persistence_store(store.clone());
        }

        // Enter execution context (RAII guard handles cleanup and mutation application)
        let _guard = match ExecutionGuard::enter(exec_ctx) {
            Ok(g) => g,
            Err(e) => {
                return BehaviorExecResult::failure(behavior.id, e.to_string());
            }
        };

        // Call on_event
        let result = self.engine.call_function_dynamic(
            &behavior.script_path,
            "on_event",
            (ctx.to_dynamic(), event_data),
        );

        // Guard drop applies mutations automatically

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
