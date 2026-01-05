//! Generic Handler Infrastructure
//!
//! Provides reusable registry and dispatcher patterns for script-based handlers.
//! Used by both the Action system (player inputs) and Effect system (combat mechanics).
//!
//! # Architecture
//!
//! ```text
//! HandlerRegistry<C>   - Maps handler names to Handler<C>
//! HandlerDispatcher<C> - Looks up handlers and executes them
//! HandlerResult        - Standardized result with success/error/data/mutations
//! ```
//!
//! # Usage
//!
//! Specialize with a context type:
//! - `HandlerDispatcher<ActionContext>` for player actions
//! - `HandlerDispatcher<EffectContext>` for combat effects

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Dynamic, Map};
use uuid::Uuid;

use crate::engine::ScriptEngine;
use bw_game::state::{StateAccessor, StateMutation};
use crate::context::{ScriptExecutionContext, ExecutionGuard};

/// Trait for handler contexts.
///
/// Contexts carry the data needed for handler execution:
/// - ActionContext: player_id, ship_id, sector_id, action name
/// - EffectContext: source_id, target_id, damage, weapon_id
pub trait HandlerContext: Clone + Send + Sync + 'static {
    /// Convert to Rhai Dynamic for passing to script handlers.
    fn to_dynamic(&self) -> Dynamic;

    /// Get the sector ID for this context (used for state accessor).
    fn sector_id(&self) -> Option<Uuid> {
        None
    }

    /// Get the owner entity ID for this context (used for permissions).
    fn owner_entity_id(&self) -> Option<Uuid> {
        None
    }
}

/// Trait for built-in Rust handler implementations.
pub trait HandlerFn<C: HandlerContext>: Send + Sync {
    /// Execute the handler with the given context and parameters.
    fn execute(&self, ctx: &C, params: &Dynamic) -> HandlerResult;
}

/// A registered handler.
#[derive(Clone)]
pub struct Handler<C: HandlerContext> {
    /// Unique ID for this handler registration.
    pub id: Uuid,
    /// Handler name (e.g., "dock", "shield_pierce").
    pub name: String,
    /// The handler implementation.
    pub handler_type: HandlerType<C>,
    /// Whether this handler is currently enabled.
    pub enabled: bool,
}

impl<C: HandlerContext> Handler<C> {
    /// Create a new script-based handler.
    pub fn script(
        name: impl Into<String>,
        script_path: impl Into<String>,
        function_name: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            handler_type: HandlerType::Script {
                script_path: script_path.into(),
                function_name: function_name.into(),
            },
            enabled: true,
        }
    }

    /// Create a new built-in Rust handler.
    pub fn builtin(name: impl Into<String>, handler: Arc<dyn HandlerFn<C>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            handler_type: HandlerType::Builtin(handler),
            enabled: true,
        }
    }
}

/// The type of handler implementation.
#[derive(Clone)]
pub enum HandlerType<C: HandlerContext> {
    /// Built-in Rust handler.
    Builtin(Arc<dyn HandlerFn<C>>),
    /// Script-defined handler.
    Script {
        script_path: String,
        function_name: String,
    },
}

/// Result from executing a handler.
#[derive(Debug, Clone, Default)]
pub struct HandlerResult {
    /// Whether the handler succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Optional response data.
    pub data: Option<Dynamic>,
    /// State mutations collected during execution.
    pub mutations: Vec<StateMutation>,
}

impl HandlerResult {
    /// Create a success result.
    pub fn success() -> Self {
        Self {
            success: true,
            error: None,
            data: None,
            mutations: Vec::new(),
        }
    }

    /// Create a success result with data.
    pub fn success_with_data(data: Dynamic) -> Self {
        Self {
            success: true,
            error: None,
            data: Some(data),
            mutations: Vec::new(),
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(message.into()),
            data: None,
            mutations: Vec::new(),
        }
    }

    /// Add mutations to the result.
    pub fn with_mutations(mut self, mutations: Vec<StateMutation>) -> Self {
        self.mutations = mutations;
        self
    }
}

/// Generic registry mapping handler names to handlers.
pub struct HandlerRegistry<C: HandlerContext> {
    /// Handlers indexed by name.
    handlers: RwLock<HashMap<String, Handler<C>>>,
    /// Handlers by script path for cleanup.
    by_script: RwLock<HashMap<String, Vec<String>>>,
    /// Phantom data for the context type.
    _phantom: PhantomData<C>,
}

impl<C: HandlerContext> HandlerRegistry<C> {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            by_script: RwLock::new(HashMap::new()),
            _phantom: PhantomData,
        }
    }

    /// Register a handler.
    ///
    /// Returns the previous handler if one was already registered with this name.
    pub fn register(&self, handler: Handler<C>) -> Option<Handler<C>> {
        let name = handler.name.clone();

        // Track script handlers for cleanup
        if let HandlerType::Script { ref script_path, .. } = handler.handler_type {
            self.by_script.write()
                .entry(script_path.clone())
                .or_default()
                .push(name.clone());
        }

        tracing::debug!(name = %name, "Handler registered");
        self.handlers.write().insert(name, handler)
    }

    /// Unregister a handler by name.
    pub fn unregister(&self, name: &str) -> Option<Handler<C>> {
        let handler = self.handlers.write().remove(name);

        if let Some(ref h) = handler {
            if let HandlerType::Script { ref script_path, .. } = h.handler_type
                && let Some(names) = self.by_script.write().get_mut(script_path) {
                    names.retain(|n| n != name);
                }
            tracing::debug!(name = %name, "Handler unregistered");
        }

        handler
    }

    /// Unregister all handlers from a specific script.
    pub fn unregister_for_script(&self, script_path: &str) {
        let names: Vec<String> = self.by_script.write()
            .remove(script_path)
            .unwrap_or_default();

        let mut handlers = self.handlers.write();
        for name in names {
            handlers.remove(&name);
        }

        tracing::debug!(script = %script_path, "All handlers for script unregistered");
    }

    /// Get a handler by name.
    pub fn get(&self, name: &str) -> Option<Handler<C>> {
        self.handlers.read().get(name).cloned()
    }

    /// Check if a handler is registered.
    pub fn is_registered(&self, name: &str) -> bool {
        self.handlers.read().contains_key(name)
    }

    /// Enable or disable a handler.
    pub fn set_enabled(&self, name: &str, enabled: bool) {
        if let Some(handler) = self.handlers.write().get_mut(name) {
            handler.enabled = enabled;
        }
    }

    /// Get count of registered handlers.
    pub fn count(&self) -> usize {
        self.handlers.read().len()
    }

    /// List all registered handler names.
    pub fn list_names(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }

    /// List all handlers.
    pub fn list_all(&self) -> Vec<Handler<C>> {
        self.handlers.read().values().cloned().collect()
    }
}

impl<C: HandlerContext> Default for HandlerRegistry<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic dispatcher that looks up and executes handlers.
pub struct HandlerDispatcher<C: HandlerContext> {
    /// The handler registry.
    registry: Arc<HandlerRegistry<C>>,
    /// Script engine for executing script handlers.
    engine: Arc<ScriptEngine>,
    /// State accessor for script execution context.
    state_accessor: Option<Arc<StateAccessor>>,
}

impl<C: HandlerContext> HandlerDispatcher<C> {
    /// Create a new dispatcher.
    pub fn new(registry: Arc<HandlerRegistry<C>>, engine: Arc<ScriptEngine>) -> Self {
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

    /// Get a reference to the registry.
    pub fn registry(&self) -> &Arc<HandlerRegistry<C>> {
        &self.registry
    }

    /// Dispatch to a handler by name.
    pub fn dispatch(&self, name: &str, ctx: &C, params: &Dynamic) -> HandlerResult {
        // Look up the handler
        let handler = match self.registry.get(name) {
            Some(h) => h,
            None => {
                tracing::debug!(name = %name, "Handler not found");
                return HandlerResult::error(format!("Unknown handler: {}", name));
            }
        };

        // Check if handler is enabled
        if !handler.enabled {
            return HandlerResult::error(format!("Handler '{}' is currently disabled", name));
        }

        // Execute based on handler type
        match self.execute_handler(&handler, ctx, params) {
            Ok((result, mutations)) => result.with_mutations(mutations),
            Err(e) => {
                tracing::warn!(name = %name, error = %e, "Handler execution failed");
                HandlerResult::error(e)
            }
        }
    }

    /// Execute a handler.
    fn execute_handler(
        &self,
        handler: &Handler<C>,
        ctx: &C,
        params: &Dynamic,
    ) -> Result<(HandlerResult, Vec<StateMutation>), String> {
        match &handler.handler_type {
            HandlerType::Builtin(handler_fn) => {
                // Built-in handlers execute directly
                let result = handler_fn.execute(ctx, params);
                Ok((result, Vec::new()))
            }
            HandlerType::Script { script_path, function_name } => {
                self.execute_script_handler(script_path, function_name, ctx, params)
            }
        }
    }

    /// Execute a script-based handler.
    fn execute_script_handler(
        &self,
        script_path: &str,
        function_name: &str,
        ctx: &C,
        params: &Dynamic,
    ) -> Result<(HandlerResult, Vec<StateMutation>), String> {
        // Check script exists
        if !self.engine.has_script(script_path) {
            return Err(format!("Script not found: {}", script_path));
        }

        // Set up execution context
        let _guard = if let Some(ref accessor) = self.state_accessor {
            let mut exec_ctx = ScriptExecutionContext::new(accessor.clone())
                .with_script_path(script_path);

            if let Some(sector_id) = ctx.sector_id() {
                exec_ctx = exec_ctx.with_sector(sector_id);
                accessor.set_context_sector(sector_id);
            }

            if let Some(entity_id) = ctx.owner_entity_id() {
                exec_ctx = exec_ctx.with_owner_entity(entity_id);
            }

            match ExecutionGuard::enter(exec_ctx) {
                Ok(guard) => Some(guard),
                Err(e) => {
                    tracing::warn!("Failed to enter execution context: {}", e);
                    return Err(format!("Execution context error: {}", e));
                }
            }
        } else {
            None
        };

        // Call the handler function
        let result = self.engine.call_function_dynamic(
            script_path,
            function_name,
            (ctx.to_dynamic(), params.clone()),
        );

        // Collect mutations before guard drops
        let mutations = if let Some(ref accessor) = self.state_accessor {
            accessor.take_mutations()
        } else {
            Vec::new()
        };

        // Parse the result
        match result {
            Ok(return_val) => {
                let handler_result = parse_handler_result(return_val);
                Ok((handler_result, mutations))
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Parse the return value from a script handler.
///
/// Accepts:
/// - `()` or nothing -> success
/// - `true`/`false` -> success/failure
/// - `#{ success: bool, error: String, data: ... }` -> full result
fn parse_handler_result(value: Dynamic) -> HandlerResult {
    // Unit -> success
    if value.is_unit() {
        return HandlerResult::success();
    }

    // Boolean -> success/failure
    if let Some(b) = value.clone().try_cast::<bool>() {
        return if b {
            HandlerResult::success()
        } else {
            HandlerResult::error("Handler returned false")
        };
    }

    // Map -> parse fields
    if let Some(map) = value.clone().try_cast::<Map>() {
        let success = map.get("success")
            .and_then(|v| v.clone().try_cast::<bool>())
            .unwrap_or(true);

        let error = map.get("error")
            .and_then(|v| v.clone().try_cast::<String>());

        let data = map.get("data").cloned();

        return HandlerResult {
            success,
            error,
            data,
            mutations: Vec::new(),
        };
    }

    // Unknown -> assume success with data
    HandlerResult::success_with_data(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestContext {
        value: i32,
    }

    impl HandlerContext for TestContext {
        fn to_dynamic(&self) -> Dynamic {
            let mut map = Map::new();
            map.insert("value".into(), Dynamic::from(self.value));
            Dynamic::from(map)
        }
    }

    struct TestHandler;

    impl HandlerFn<TestContext> for TestHandler {
        fn execute(&self, ctx: &TestContext, _params: &Dynamic) -> HandlerResult {
            HandlerResult::success_with_data(Dynamic::from(ctx.value * 2))
        }
    }

    #[test]
    fn test_registry_register_unregister() {
        let registry = HandlerRegistry::<TestContext>::new();

        let handler = Handler::builtin("test", Arc::new(TestHandler));
        registry.register(handler);

        assert!(registry.is_registered("test"));
        assert_eq!(registry.count(), 1);

        registry.unregister("test");
        assert!(!registry.is_registered("test"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_handler_result_parsing() {
        // Unit -> success
        assert!(parse_handler_result(Dynamic::UNIT).success);

        // true -> success
        assert!(parse_handler_result(Dynamic::from(true)).success);

        // false -> error
        assert!(!parse_handler_result(Dynamic::from(false)).success);

        // Map with success: true
        let mut map = Map::new();
        map.insert("success".into(), Dynamic::from(true));
        assert!(parse_handler_result(Dynamic::from(map)).success);

        // Map with success: false and error
        let mut map = Map::new();
        map.insert("success".into(), Dynamic::from(false));
        map.insert("error".into(), Dynamic::from("test error"));
        let result = parse_handler_result(Dynamic::from(map));
        assert!(!result.success);
        assert_eq!(result.error, Some("test error".to_string()));
    }
}
