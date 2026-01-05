//! Unified Script Execution Context
//!
//! Consolidates all thread-local state needed during script execution into
//! a single `ScriptExecutionContext` with RAII-based setup/teardown via
//! `ExecutionGuard`.
//!
//! # Thread Safety
//!
//! The thread-local pattern is safe because:
//! 1. Rhai script execution is synchronous (no await points during execution)
//! 2. The `ExecutionGuard` prevents re-entrant script execution
//! 3. Callers must hold appropriate locks before executing scripts
//!
//! # Usage
//!
//! ```ignore
//! let context = ScriptExecutionContext::new(accessor)
//!     .with_script_path("ai/pirate.rhai")
//!     .with_owner_entity(ship_id)
//!     .with_sector(sector_id)
//!     .with_tick(game_tick);
//!
//! let _guard = ExecutionGuard::enter(context)?;
//! // Execute script - guard handles cleanup and mutation application on drop
//! engine.eval_ast(&ast)?;
//! ```

use std::cell::RefCell;
use std::sync::Arc;
use uuid::Uuid;
use thiserror::Error;

use crate::state::{StateAccessor, WatchRegistry};
use crate::events::EventRegistry;
use crate::actions::ActionRegistry;
use crate::effects::EffectDispatcher;
use crate::persistence::ScriptStateStore;

/// Error during script execution context setup.
#[derive(Error, Debug, Clone)]
pub enum ExecutionError {
    #[error("Re-entrant script execution detected - a script is already running on this thread")]
    ReentrantExecution,
}

/// Unified execution context containing all state needed during script execution.
///
/// This consolidates thread-locals that were previously scattered across:
/// - `state_api.rs` (CURRENT_ACCESSOR, SCRIPT_EXECUTING, CURRENT_WATCH_REGISTRY, CURRENT_SCRIPT_PATH)
/// - `event_api.rs` (CURRENT_REGISTRY, CURRENT_CONTEXT)
/// - `persistence/bindings.rs` (CURRENT_STORE)
/// - `errors.rs` (CURRENT_SCRIPT, CURRENT_TICK)
#[derive(Clone)]
pub struct ScriptExecutionContext {
    /// State accessor for reading game state and queueing mutations.
    pub accessor: Arc<StateAccessor>,

    /// Watch registry for property watches (optional).
    pub watch_registry: Option<Arc<WatchRegistry>>,

    /// Event registry for event subscriptions (optional).
    pub event_registry: Option<Arc<EventRegistry>>,

    /// Action registry for action handlers (optional).
    pub action_registry: Option<Arc<ActionRegistry>>,

    /// Effect dispatcher for combat effects (optional).
    pub effect_dispatcher: Option<Arc<EffectDispatcher>>,

    /// Persistence store for script state (optional).
    pub persistence_store: Option<Arc<dyn ScriptStateStore>>,

    /// Path to the currently executing script.
    pub script_path: String,

    /// The entity this script is attached to (for behavior scripts).
    pub owner_entity_id: Option<Uuid>,

    /// The sector where the script is executing.
    pub sector_id: Option<Uuid>,

    /// Current game tick for error timestamps.
    pub tick: u64,
}

impl ScriptExecutionContext {
    /// Create a new context with just the required accessor.
    pub fn new(accessor: Arc<StateAccessor>) -> Self {
        Self {
            accessor,
            watch_registry: None,
            event_registry: None,
            action_registry: None,
            effect_dispatcher: None,
            persistence_store: None,
            script_path: String::new(),
            owner_entity_id: None,
            sector_id: None,
            tick: 0,
        }
    }

    /// Set the script path.
    pub fn with_script_path(mut self, path: impl Into<String>) -> Self {
        self.script_path = path.into();
        self
    }

    /// Set the owner entity ID.
    pub fn with_owner_entity(mut self, entity_id: Uuid) -> Self {
        self.owner_entity_id = Some(entity_id);
        self
    }

    /// Set the sector ID.
    pub fn with_sector(mut self, sector_id: Uuid) -> Self {
        self.sector_id = Some(sector_id);
        self
    }

    /// Set the current tick.
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    /// Set the watch registry.
    pub fn with_watch_registry(mut self, registry: Arc<WatchRegistry>) -> Self {
        self.watch_registry = Some(registry);
        self
    }

    /// Set the event registry.
    pub fn with_event_registry(mut self, registry: Arc<EventRegistry>) -> Self {
        self.event_registry = Some(registry);
        self
    }

    /// Set the action registry.
    pub fn with_action_registry(mut self, registry: Arc<ActionRegistry>) -> Self {
        self.action_registry = Some(registry);
        self
    }

    /// Set the effect dispatcher.
    pub fn with_effect_dispatcher(mut self, dispatcher: Arc<EffectDispatcher>) -> Self {
        self.effect_dispatcher = Some(dispatcher);
        self
    }

    /// Set the persistence store.
    pub fn with_persistence_store(mut self, store: Arc<dyn ScriptStateStore>) -> Self {
        self.persistence_store = Some(store);
        self
    }
}

thread_local! {
    /// The current execution context for this thread.
    static CURRENT_CONTEXT: RefCell<Option<ScriptExecutionContext>> = const { RefCell::new(None) };
}

/// RAII guard that manages execution context lifetime.
///
/// When created via `enter()`, sets up the context. When dropped, cleans up
/// and applies any pending mutations.
pub struct ExecutionGuard {
    /// Marker to prevent manual construction.
    _private: (),
}

impl ExecutionGuard {
    /// Enter script execution with the given context.
    ///
    /// # Errors
    ///
    /// Returns `ExecutionError::ReentrantExecution` if a script is already
    /// executing on this thread.
    pub fn enter(context: ScriptExecutionContext) -> Result<Self, ExecutionError> {
        CURRENT_CONTEXT.with(|cell| {
            let mut ctx = cell.borrow_mut();
            if ctx.is_some() {
                return Err(ExecutionError::ReentrantExecution);
            }

            // Set up error context (script path and tick)
            crate::errors::set_current_script(&context.script_path);
            crate::errors::set_current_tick(context.tick);

            *ctx = Some(context);
            Ok(Self { _private: () })
        })
    }
}

impl Drop for ExecutionGuard {
    fn drop(&mut self) {
        CURRENT_CONTEXT.with(|cell| {
            if let Some(ctx) = cell.borrow_mut().take() {
                // Apply pending mutations
                let _ = ctx.accessor.apply_pending_mutations();

                // Clear error context
                crate::errors::clear_current_script();
            }
        });
    }
}

/// Access the current execution context.
///
/// Returns `None` if not currently executing a script.
pub fn with_context<T>(f: impl FnOnce(&ScriptExecutionContext) -> T) -> Option<T> {
    CURRENT_CONTEXT.with(|cell| {
        cell.borrow().as_ref().map(f)
    })
}

/// Access the current execution context mutably.
///
/// Returns `None` if not currently executing a script.
pub fn with_context_mut<T>(f: impl FnOnce(&mut ScriptExecutionContext) -> T) -> Option<T> {
    CURRENT_CONTEXT.with(|cell| {
        cell.borrow_mut().as_mut().map(f)
    })
}

/// Check if a script is currently executing.
pub fn is_executing() -> bool {
    CURRENT_CONTEXT.with(|cell| cell.borrow().is_some())
}

/// Get the current script path.
pub fn current_script_path() -> Option<String> {
    with_context(|ctx| ctx.script_path.clone())
}

/// Get the current owner entity ID.
pub fn current_owner_entity() -> Option<Uuid> {
    with_context(|ctx| ctx.owner_entity_id).flatten()
}

/// Get the current sector ID.
pub fn current_sector() -> Option<Uuid> {
    with_context(|ctx| ctx.sector_id).flatten()
}

/// Get the current tick.
pub fn current_tick() -> u64 {
    with_context(|ctx| ctx.tick).unwrap_or(0)
}

// ============================================================================
// Legacy compatibility functions
// ============================================================================
// These bridge the old API to the new unified context system.
// They will be removed once all callers are migrated.

/// Get the current state accessor.
///
/// # Deprecated
/// Use `with_context(|ctx| ...)` instead.
pub fn with_accessor<T>(f: impl FnOnce(&StateAccessor) -> T) -> Option<T> {
    with_context(|ctx| f(&ctx.accessor))
}

/// Get the current watch registry.
pub fn with_watch_registry<T>(f: impl FnOnce(&WatchRegistry) -> T) -> Option<T> {
    with_context(|ctx| ctx.watch_registry.as_ref().map(|r| f(r))).flatten()
}

/// Get the current event registry.
pub fn with_event_registry<T>(f: impl FnOnce(&EventRegistry) -> T) -> Option<T> {
    with_context(|ctx| ctx.event_registry.as_ref().map(|r| f(r))).flatten()
}

/// Get the current action registry.
pub fn with_action_registry<T>(f: impl FnOnce(&ActionRegistry) -> T) -> Option<T> {
    with_context(|ctx| ctx.action_registry.as_ref().map(|r| f(r))).flatten()
}

/// Get the current effect dispatcher.
pub fn with_effect_dispatcher<T>(f: impl FnOnce(&EffectDispatcher) -> T) -> Option<T> {
    with_context(|ctx| ctx.effect_dispatcher.as_ref().map(|d| f(d))).flatten()
}

/// Get the current persistence store.
pub fn with_persistence_store<T>(f: impl FnOnce(&dyn ScriptStateStore) -> T) -> Option<T> {
    with_context(|ctx| ctx.persistence_store.as_ref().map(|s| f(s.as_ref()))).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateProvider;
    use bw_core::models::Position;

    struct MockProvider;
    impl StateProvider for MockProvider {
        fn get_ship(&self, _: Uuid) -> Option<crate::state::ShipSnapshot> { None }
        fn get_ships_in_sector(&self, _: Uuid) -> Vec<crate::state::ShipSnapshot> { vec![] }
        fn get_ships_in_range(&self, _: Uuid, _: Position, _: f64) -> Vec<crate::state::ShipSnapshot> { vec![] }
        fn get_player(&self, _: Uuid) -> Option<crate::state::PlayerSnapshot> { None }
        fn get_sector(&self, _: Uuid) -> Option<crate::state::SectorSnapshot> { None }
        fn apply_mutations(&self, _: Vec<crate::state::StateMutation>) -> Vec<crate::state::MutationResult> { vec![] }
    }

    #[test]
    fn test_execution_guard_basic() {
        let accessor = Arc::new(StateAccessor::new(Arc::new(MockProvider)));
        let context = ScriptExecutionContext::new(accessor)
            .with_script_path("test.rhai")
            .with_tick(42);

        assert!(!is_executing());

        {
            let _guard = ExecutionGuard::enter(context).unwrap();
            assert!(is_executing());
            assert_eq!(current_script_path(), Some("test.rhai".to_string()));
            assert_eq!(current_tick(), 42);
        }

        assert!(!is_executing());
    }

    #[test]
    fn test_reentrant_execution_prevented() {
        let accessor = Arc::new(StateAccessor::new(Arc::new(MockProvider)));
        let context1 = ScriptExecutionContext::new(accessor.clone());
        let context2 = ScriptExecutionContext::new(accessor);

        let _guard = ExecutionGuard::enter(context1).unwrap();

        // Attempting to enter again should fail
        let result = ExecutionGuard::enter(context2);
        assert!(matches!(result, Err(ExecutionError::ReentrantExecution)));
    }

    #[test]
    fn test_with_context() {
        let accessor = Arc::new(StateAccessor::new(Arc::new(MockProvider)));
        let entity_id = Uuid::new_v4();
        let context = ScriptExecutionContext::new(accessor)
            .with_owner_entity(entity_id);

        let _guard = ExecutionGuard::enter(context).unwrap();

        let result = with_context(|ctx| ctx.owner_entity_id);
        assert_eq!(result, Some(Some(entity_id)));
    }
}
