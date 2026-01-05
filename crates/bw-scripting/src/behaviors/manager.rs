//! Behavior manager
//!
//! Manages the lifecycle of entity behaviors, including behavior tree execution.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Dynamic, Map, Engine};
use uuid::Uuid;

use bw_core::events::GameEvent;
use crate::engine::{ScriptEngine, ScriptError};
use crate::state::{StateAccessor, AccessPermissions};
use crate::events::EventRegistry;
use crate::context::{ScriptExecutionContext, ExecutionGuard};
use crate::ai::{BehaviorTreeRunner, AiContext, BtNode};
use crate::persistence::{ScriptStateStore, PersistenceError};
use crate::debug::{DebugController, DebugTarget, EntityContext};

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
    /// Debug controller for script debugging
    debug_controller: Option<Arc<DebugController>>,
    /// Debug target context (Live or Playtest(id))
    debug_target: DebugTarget,
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
            debug_controller: None,
            debug_target: DebugTarget::Live,
            current_tick: RwLock::new(0),
            current_game_time: RwLock::new(0.0),
        }
    }

    /// Set the debug controller for script debugging.
    pub fn set_debug_controller(&mut self, controller: Arc<DebugController>) {
        self.debug_controller = Some(controller);
    }

    /// Set the debug target context for this behavior manager.
    ///
    /// For the live server, this should be `DebugTarget::Live`.
    /// For a playtest, this should be `DebugTarget::Playtest(playtest_id)`.
    pub fn set_debug_target(&mut self, target: DebugTarget) {
        self.debug_target = target;
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
            {
                let mut by_entity = self.by_entity.write();
                if let Some(ids) = by_entity.get_mut(&entity_id) {
                    ids.retain(|i| *i != id);
                    // Clean up empty vectors to prevent memory leak
                    if ids.is_empty() {
                        by_entity.remove(&entity_id);
                    }
                }
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
            {
                let mut by_entity = self.by_entity.write();
                if let Some(ids) = by_entity.get_mut(&behavior.entity_id) {
                    ids.retain(|id| *id != behavior_id);
                    // Clean up empty vectors to prevent memory leak
                    if ids.is_empty() {
                        by_entity.remove(&behavior.entity_id);
                    }
                }
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

                // Save updated local_data if any was returned
                if let Some(updated_data) = result.updated_data {
                    if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
                        // Merge updated data into existing local_data
                        for (key, value) in updated_data {
                            behavior.local_data.insert(key, value);
                        }
                    }
                }

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

            // Use "behavior" namespace with behavior.id as key (not entity_id)
            // to avoid collision when entity has multiple behaviors
            store.save("behavior", &behavior.id.to_string(), &data)?;

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

            // Use behavior.id as key to avoid collision when entity has multiple behaviors
            store.save("behavior", &behavior.id.to_string(), &data)?;
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

        // Find all behaviors for this entity and restore each one individually
        let behavior_ids: Vec<Uuid> = self.behaviors.read()
            .values()
            .filter(|b| b.entity_id == entity_id && b.state == BehaviorState::Active)
            .map(|b| b.id)
            .collect();

        let mut any_restored = false;
        for behavior_id in behavior_ids {
            let Some(data) = store.load("behavior", &behavior_id.to_string())? else {
                continue; // No saved state for this behavior
            };

            // Deserialize local_data
            let local_data: Map = serde_json::from_slice(&data)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

            // Apply to the behavior
            if let Some(behavior) = self.behaviors.write().get_mut(&behavior_id) {
                behavior.local_data = local_data;
                tracing::debug!(
                    entity_id = %entity_id,
                    behavior_id = %behavior_id,
                    "Restored behavior state"
                );
                any_restored = true;
            }
        }

        Ok(any_restored)
    }

    /// Clear persisted state for an entity's behaviors.
    pub fn clear_persisted_state(&self, entity_id: Uuid) -> Result<bool, PersistenceError> {
        let Some(store) = &self.state_store else {
            return Ok(false);
        };

        // Find all behaviors for this entity and clear each one
        let behavior_ids: Vec<Uuid> = self.behaviors.read()
            .values()
            .filter(|b| b.entity_id == entity_id)
            .map(|b| b.id)
            .collect();

        let mut any_deleted = false;
        for behavior_id in behavior_ids {
            if store.delete("behavior", &behavior_id.to_string())? {
                any_deleted = true;
            }
        }

        Ok(any_deleted)
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

        // Check if we should use a debug engine
        let debug_engine: Option<Engine> = self.debug_controller.as_ref().and_then(|dc| {
            // Find debug sessions that have breakpoints for this script AND match our target
            let sessions = dc.sessions_for_script_with_target(&behavior.script_path, &self.debug_target);
            if let Some(&session_id) = sessions.first() {
                // Create entity context for the debugger
                let entity_ctx = EntityContext {
                    entity_type: behavior.entity_type.as_str().to_string(),
                    entity_id: behavior.entity_id,
                    entity_name: format!("{}:{}", behavior.entity_type.as_str(), behavior.entity_id),
                    sector_id: behavior.sector_id,
                };

                // Create a debug-enabled engine
                let base_engine = self.engine.create_engine_with_bindings();
                dc.create_debug_engine(session_id, base_engine, &behavior.script_path, Some(entity_ctx))
            } else {
                None
            }
        });

        // Call the hook (with debug engine if available)
        let result = if let Some(ref dbg_engine) = debug_engine {
            tracing::debug!(
                behavior_id = %behavior.id,
                script = %behavior.script_path,
                hook = hook_name,
                "Executing with debug engine"
            );
            self.engine.call_function_dynamic_with_engine(
                dbg_engine,
                &behavior.script_path,
                hook_name,
                (ctx.to_dynamic(),),
            )
        } else {
            self.engine.call_function_dynamic(
                &behavior.script_path,
                hook_name,
                (ctx.to_dynamic(),),
            )
        };

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
