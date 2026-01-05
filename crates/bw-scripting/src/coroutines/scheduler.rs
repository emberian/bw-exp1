//! Coroutine scheduler
//!
//! Manages the lifecycle of coroutines, tracking which are ready to run,
//! waiting for ticks, or waiting for events.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use uuid::Uuid;
use rhai::{Dynamic, Map, Scope};

use crate::engine::{ScriptEngine, ScriptError};
use crate::state::{StateAccessor, AccessPermissions};
use crate::bindings::state_api::{set_current_accessor, clear_current_accessor};

use super::{
    Coroutine, CoroutineState, CoroutineExecResult, CoroutineTickResult,
    YieldRequest, YieldType, take_yield_request, clear_yield_request,
};

/// Manages coroutine lifecycle and scheduling.
pub struct CoroutineScheduler {
    /// All active coroutines
    coroutines: RwLock<HashMap<Uuid, Coroutine>>,
    /// Coroutines waiting for specific ticks, indexed by resume tick
    tick_waiters: RwLock<HashMap<u64, Vec<Uuid>>>,
    /// Coroutines waiting for specific events, indexed by event type
    event_waiters: RwLock<HashMap<String, Vec<Uuid>>>,
    /// Queue of coroutines ready to run this tick
    ready_queue: Mutex<VecDeque<Uuid>>,
    /// Script engine for execution
    engine: Arc<ScriptEngine>,
    /// State accessor for scripts
    state_accessor: Option<Arc<StateAccessor>>,
    /// Maximum coroutines to process per tick (to avoid starvation)
    max_per_tick: usize,
}

impl CoroutineScheduler {
    /// Create a new scheduler.
    pub fn new(engine: Arc<ScriptEngine>) -> Self {
        Self {
            coroutines: RwLock::new(HashMap::new()),
            tick_waiters: RwLock::new(HashMap::new()),
            event_waiters: RwLock::new(HashMap::new()),
            ready_queue: Mutex::new(VecDeque::new()),
            engine,
            state_accessor: None,
            max_per_tick: 100,
        }
    }

    /// Set the state accessor for script execution.
    pub fn set_state_accessor(&mut self, accessor: Arc<StateAccessor>) {
        self.state_accessor = Some(accessor);
    }

    /// Spawn a new coroutine.
    pub fn spawn(
        &self,
        script_path: &str,
        function_name: &str,
        current_tick: u64,
    ) -> Result<Uuid, ScriptError> {
        // Verify script exists
        if !self.engine.has_script(script_path) {
            return Err(ScriptError::NotFound(script_path.to_string()));
        }

        let coroutine = Coroutine::new(script_path, function_name, current_tick);
        let id = coroutine.id;

        self.coroutines.write().insert(id, coroutine);
        self.ready_queue.lock().push_back(id);

        tracing::debug!(coroutine_id = %id, script = script_path, function = function_name, "Spawned coroutine");

        Ok(id)
    }

    /// Spawn a coroutine with owner and sector context.
    pub fn spawn_with_context(
        &self,
        script_path: &str,
        function_name: &str,
        current_tick: u64,
        owner_entity_id: Option<Uuid>,
        sector_id: Option<Uuid>,
    ) -> Result<Uuid, ScriptError> {
        if !self.engine.has_script(script_path) {
            return Err(ScriptError::NotFound(script_path.to_string()));
        }

        let mut coroutine = Coroutine::new(script_path, function_name, current_tick);
        coroutine.owner_entity_id = owner_entity_id;
        coroutine.sector_id = sector_id;

        let id = coroutine.id;

        self.coroutines.write().insert(id, coroutine);
        self.ready_queue.lock().push_back(id);

        Ok(id)
    }

    /// Cancel a coroutine.
    pub fn cancel(&self, coroutine_id: Uuid) -> bool {
        let mut coroutines = self.coroutines.write();
        if let Some(coroutine) = coroutines.remove(&coroutine_id) {
            // Remove from tick waiters
            if let Some(resume_at) = coroutine.resume_at {
                if let Some(waiters) = self.tick_waiters.write().get_mut(&resume_at) {
                    waiters.retain(|id| *id != coroutine_id);
                }
            }

            // Remove from event waiters
            if let CoroutineState::WaitingForEvent(event_type) = &coroutine.state {
                if let Some(waiters) = self.event_waiters.write().get_mut(event_type) {
                    waiters.retain(|id| *id != coroutine_id);
                }
            }

            tracing::debug!(coroutine_id = %coroutine_id, "Cancelled coroutine");
            true
        } else {
            false
        }
    }

    /// Cancel all coroutines for an entity.
    pub fn cancel_for_entity(&self, entity_id: Uuid) {
        let ids_to_cancel: Vec<Uuid> = self.coroutines.read()
            .iter()
            .filter(|(_, c)| c.owner_entity_id == Some(entity_id))
            .map(|(id, _)| *id)
            .collect();

        for id in ids_to_cancel {
            self.cancel(id);
        }
    }

    /// Process one tick of coroutine scheduling.
    pub fn tick(&self, current_tick: u64) -> CoroutineTickResult {
        // Move tick waiters that are ready to the ready queue
        self.wake_tick_waiters(current_tick);

        // Move next-frame waiters to ready queue
        self.wake_next_frame_waiters();

        // Process ready coroutines (up to max_per_tick)
        let mut completed = Vec::new();
        let mut failed = Vec::new();
        let mut processed = 0;

        while processed < self.max_per_tick {
            let coroutine_id = {
                let mut queue = self.ready_queue.lock();
                queue.pop_front()
            };

            let Some(id) = coroutine_id else {
                break;
            };

            match self.execute_coroutine(id, current_tick) {
                Ok(CoroutineExecResult::Completed(_)) => {
                    completed.push(id);
                    self.coroutines.write().remove(&id);
                }
                Ok(CoroutineExecResult::Yielded(request)) => {
                    self.handle_yield(id, request, current_tick);
                }
                Ok(CoroutineExecResult::Failed(error)) => {
                    failed.push((id, error));
                    self.coroutines.write().remove(&id);
                }
                Err(e) => {
                    failed.push((id, e.to_string()));
                    self.coroutines.write().remove(&id);
                }
            }

            processed += 1;
        }

        let pending_count = self.coroutines.read().len();

        CoroutineTickResult {
            completed,
            failed,
            pending_count,
        }
    }

    /// Notify the scheduler of a game event (may wake event waiters).
    pub fn notify_event(&self, event_type: &str, event_data: Map) {
        let waiters = {
            let mut event_waiters = self.event_waiters.write();
            event_waiters.remove(event_type).unwrap_or_default()
        };

        if waiters.is_empty() {
            return;
        }

        tracing::debug!(event_type, waiter_count = waiters.len(), "Waking event waiters");

        let mut coroutines = self.coroutines.write();
        let mut ready_queue = self.ready_queue.lock();

        for id in waiters {
            if let Some(coroutine) = coroutines.get_mut(&id) {
                coroutine.state = CoroutineState::Ready;
                coroutine.resume_value = Some(Dynamic::from(event_data.clone()));
                ready_queue.push_back(id);
            }
        }
    }

    /// Get count of active coroutines.
    pub fn active_count(&self) -> usize {
        self.coroutines.read().len()
    }

    /// Get count of coroutines for an entity.
    pub fn count_for_entity(&self, entity_id: Uuid) -> usize {
        self.coroutines.read()
            .values()
            .filter(|c| c.owner_entity_id == Some(entity_id))
            .count()
    }

    /// List all coroutines.
    pub fn list_all(&self) -> Vec<Coroutine> {
        self.coroutines.read().values().cloned().collect()
    }

    // === Private helpers ===

    fn wake_tick_waiters(&self, current_tick: u64) {
        let mut tick_waiters = self.tick_waiters.write();
        let mut coroutines = self.coroutines.write();
        let mut ready_queue = self.ready_queue.lock();

        // Find all ticks <= current_tick
        let due_ticks: Vec<u64> = tick_waiters.keys()
            .filter(|t| **t <= current_tick)
            .copied()
            .collect();

        for tick in due_ticks {
            if let Some(waiters) = tick_waiters.remove(&tick) {
                for id in waiters {
                    if let Some(coroutine) = coroutines.get_mut(&id) {
                        coroutine.state = CoroutineState::Ready;
                        ready_queue.push_back(id);
                    }
                }
            }
        }
    }

    fn wake_next_frame_waiters(&self) {
        let mut coroutines = self.coroutines.write();
        let mut ready_queue = self.ready_queue.lock();

        let next_frame_ids: Vec<Uuid> = coroutines.iter()
            .filter(|(_, c)| c.state == CoroutineState::WaitingForNextFrame)
            .map(|(id, _)| *id)
            .collect();

        for id in next_frame_ids {
            if let Some(coroutine) = coroutines.get_mut(&id) {
                coroutine.state = CoroutineState::Ready;
                ready_queue.push_back(id);
            }
        }
    }

    fn execute_coroutine(&self, id: Uuid, _current_tick: u64) -> Result<CoroutineExecResult, ScriptError> {
        // Get coroutine info (but don't hold lock during execution)
        let (script_path, function_name, local_vars, resume_value, owner_id, sector_id) = {
            let mut coroutines = self.coroutines.write();
            let coroutine = coroutines.get_mut(&id)
                .ok_or_else(|| ScriptError::NotFound(format!("Coroutine {} not found", id)))?;

            coroutine.state = CoroutineState::Running;

            (
                coroutine.script_path.clone(),
                coroutine.function_name.clone(),
                coroutine.local_vars.clone(),
                coroutine.resume_value.take(),
                coroutine.owner_entity_id,
                coroutine.sector_id,
            )
        };

        // Set up state accessor if available
        if let Some(ref accessor) = self.state_accessor {
            let perms = if let Some(entity_id) = owner_id {
                if let Some(sector_id) = sector_id {
                    AccessPermissions::npc_behavior(entity_id, sector_id)
                } else {
                    AccessPermissions::trusted()
                }
            } else {
                AccessPermissions::trusted()
            };

            accessor.set_permissions(perms);
            if let Some(sector_id) = sector_id {
                accessor.set_context_sector(sector_id);
            }
            set_current_accessor(accessor.clone());
        }

        // Clear any stale yield request
        clear_yield_request();

        // Build scope with local vars and resume value
        let mut scope = Scope::new();
        for (key, value) in local_vars {
            scope.push_dynamic(key.to_string(), value);
        }
        if let Some(resume_val) = resume_value {
            scope.push_dynamic("__resume_value", resume_val);
        }

        // Execute the function
        let result = self.engine.call_function_dynamic(
            &script_path,
            &function_name,
            (), // No additional args for coroutine resume
        );

        // Clear state accessor
        clear_current_accessor();

        // Check for yield request
        if let Some(yield_req) = take_yield_request() {
            return Ok(CoroutineExecResult::Yielded(yield_req));
        }

        match result {
            Ok(value) => Ok(CoroutineExecResult::Completed(value)),
            Err(e) => Ok(CoroutineExecResult::Failed(e.to_string())),
        }
    }

    fn handle_yield(&self, id: Uuid, request: YieldRequest, current_tick: u64) {
        let mut coroutines = self.coroutines.write();
        let Some(coroutine) = coroutines.get_mut(&id) else {
            return;
        };

        match request.yield_type {
            YieldType::Ticks(ticks) => {
                let resume_tick = current_tick + ticks;
                coroutine.state = CoroutineState::WaitingForTicks;
                coroutine.resume_at = Some(resume_tick);

                self.tick_waiters.write()
                    .entry(resume_tick)
                    .or_default()
                    .push(id);
            }
            YieldType::Seconds(secs) => {
                // Convert seconds to ticks (10 TPS)
                let ticks = (secs * 10.0).ceil() as u64;
                let resume_tick = current_tick + ticks;
                coroutine.state = CoroutineState::WaitingForTicks;
                coroutine.resume_at = Some(resume_tick);

                self.tick_waiters.write()
                    .entry(resume_tick)
                    .or_default()
                    .push(id);
            }
            YieldType::NextFrame => {
                coroutine.state = CoroutineState::WaitingForNextFrame;
            }
            YieldType::Event(event_type) => {
                coroutine.state = CoroutineState::WaitingForEvent(event_type.clone());

                self.event_waiters.write()
                    .entry(event_type)
                    .or_default()
                    .push(id);
            }
            YieldType::Schedule { delay_ticks, callback } => {
                // Schedule is fire-and-forget: spawn a new coroutine for the callback
                // and mark this yield as completed
                let _ = self.spawn_with_context(
                    &coroutine.script_path,
                    &callback,
                    current_tick + delay_ticks,
                    coroutine.owner_entity_id,
                    coroutine.sector_id,
                );

                // This coroutine continues immediately (schedule doesn't pause)
                coroutine.state = CoroutineState::Ready;
                self.ready_queue.lock().push_back(id);
            }
        }

        // Store any yield data for later use
        if !request.data.is_empty() {
            coroutine.local_vars.extend(request.data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yield_type_ticks() {
        let req = YieldRequest::ticks(10);
        assert!(matches!(req.yield_type, YieldType::Ticks(10)));
    }

    #[test]
    fn test_yield_type_seconds() {
        let req = YieldRequest::seconds(2.5);
        assert!(matches!(req.yield_type, YieldType::Seconds(s) if (s - 2.5).abs() < 0.001));
    }
}
