//! Action dispatcher
//!
//! Dispatches client actions to registered script handlers.
//! Uses the generic handler infrastructure with action-specific requirements validation.

use std::sync::Arc;
use rhai::{Dynamic, Map};
use uuid::Uuid;

use crate::engine::ScriptEngine;
use bw_game::state::{StateAccessor, AccessPermissions, StateMutation};
use crate::context::{ScriptExecutionContext, ExecutionGuard};
use crate::handlers::HandlerContext;

use super::{ActionRegistry, ActionHandler, ActionRequirement};

/// Context passed to action handlers.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// The player executing this action
    pub player_id: Uuid,
    /// The player's ship
    pub ship_id: Uuid,
    /// The sector the player is in
    pub sector_id: Uuid,
    /// The action being executed
    pub action: String,
    /// Parameters for the action (from client)
    pub params: serde_json::Value,
}

impl ActionContext {
    /// Convert to a Rhai map for passing to scripts.
    pub fn to_rhai_map(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("player_id".into(), self.player_id.to_string().into());
        map.insert("ship_id".into(), self.ship_id.to_string().into());
        map.insert("sector_id".into(), self.sector_id.to_string().into());
        map.insert("action".into(), self.action.clone().into());
        Dynamic::from(map)
    }
}

/// Implement generic HandlerContext for ActionContext.
impl HandlerContext for ActionContext {
    fn to_dynamic(&self) -> Dynamic {
        self.to_rhai_map()
    }

    fn sector_id(&self) -> Option<Uuid> {
        Some(self.sector_id)
    }

    fn owner_entity_id(&self) -> Option<Uuid> {
        Some(self.ship_id)
    }
}

/// Result from dispatching an action.
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Optional response data from the handler
    pub data: Option<serde_json::Value>,
    /// State mutations collected from the handler
    pub mutations: Vec<StateMutation>,
}

impl ActionResult {
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
    pub fn success_with_data(data: serde_json::Value) -> Self {
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

/// Dispatches actions to script handlers.
pub struct ActionDispatcher {
    registry: Arc<ActionRegistry>,
    engine: Arc<ScriptEngine>,
    state_accessor: Option<Arc<StateAccessor>>,
}

impl ActionDispatcher {
    /// Create a new dispatcher.
    pub fn new(registry: Arc<ActionRegistry>, engine: Arc<ScriptEngine>) -> Self {
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
    pub fn registry(&self) -> &Arc<ActionRegistry> {
        &self.registry
    }

    /// Dispatch an action to its handler.
    pub fn dispatch(&self, ctx: ActionContext) -> ActionResult {
        // Look up the handler
        let handler = match self.registry.get(&ctx.action) {
            Some(h) => h,
            None => {
                tracing::debug!(action = %ctx.action, "Action not found in registry");
                return ActionResult::error(format!("Unknown action: {}", ctx.action));
            }
        };

        // Check if handler is enabled
        if !handler.enabled {
            return ActionResult::error(format!("Action '{}' is currently disabled", ctx.action));
        }

        // Validate requirements
        if let Err(e) = self.validate_requirements(&handler, &ctx) {
            return ActionResult::error(e);
        }

        // Execute the handler
        match self.execute_handler(&handler, &ctx) {
            Ok((result, mutations)) => result.with_mutations(mutations),
            Err(e) => {
                tracing::warn!(
                    action = %ctx.action,
                    error = %e,
                    "Action handler failed"
                );
                ActionResult::error(e)
            }
        }
    }

    /// Validate that requirements are met.
    fn validate_requirements(&self, handler: &ActionHandler, ctx: &ActionContext) -> Result<(), String> {
        for req in &handler.requirements {
            match req {
                ActionRequirement::Authenticated => {
                    // Always true for valid sessions - player_id is set
                }
                ActionRequirement::ShipStatus(allowed) => {
                    // Check ship status via state accessor
                    if let Some(ref accessor) = self.state_accessor {
                        if let Ok(Some(ship)) = accessor.get_ship(ctx.ship_id) {
                            if !allowed.iter().any(|s| s == &ship.status) {
                                return Err(format!(
                                    "Action requires ship status {:?}, but ship is '{}'",
                                    allowed, ship.status
                                ));
                            }
                        }
                    }
                }
                ActionRequirement::MustBeDocked => {
                    if let Some(ref accessor) = self.state_accessor {
                        if let Ok(Some(ship)) = accessor.get_ship(ctx.ship_id) {
                            if ship.status != "docked" {
                                return Err("Must be docked to perform this action".to_string());
                            }
                        }
                    }
                }
                ActionRequirement::MustBeUndocked => {
                    if let Some(ref accessor) = self.state_accessor {
                        if let Ok(Some(ship)) = accessor.get_ship(ctx.ship_id) {
                            if ship.status == "docked" {
                                return Err("Cannot perform this action while docked".to_string());
                            }
                        }
                    }
                }
                ActionRequirement::NotInCombat => {
                    if let Some(ref accessor) = self.state_accessor {
                        if let Ok(Some(ship)) = accessor.get_ship(ctx.ship_id) {
                            if ship.status == "in_combat" {
                                return Err("Cannot perform this action during combat".to_string());
                            }
                        }
                    }
                }
                ActionRequirement::SameSector => {
                    // This requires target info which should be in params
                    // Validation deferred to script
                }
                ActionRequirement::Custom(_) => {
                    // Custom validation is handled in the script
                }
            }
        }

        Ok(())
    }

    /// Execute the handler script function.
    fn execute_handler(
        &self,
        handler: &ActionHandler,
        ctx: &ActionContext,
    ) -> Result<(ActionResult, Vec<StateMutation>), String> {
        // Check script exists
        if !self.engine.has_script(&handler.script_path) {
            return Err(format!("Script not found: {}", handler.script_path));
        }

        // Set up execution context
        let _guard = if let Some(ref accessor) = self.state_accessor {
            // Player actions are trusted
            let perms = AccessPermissions::trusted();
            accessor.set_permissions(perms);
            accessor.set_context_sector(ctx.sector_id);

            let exec_ctx = ScriptExecutionContext::new(accessor.clone())
                .with_script_path(&handler.script_path)
                .with_owner_entity(ctx.ship_id)
                .with_sector(ctx.sector_id);

            match ExecutionGuard::enter(exec_ctx) {
                Ok(guard) => Some(guard),
                Err(e) => {
                    tracing::warn!("Failed to enter execution context for action: {}", e);
                    return Err(format!("Execution context error: {}", e));
                }
            }
        } else {
            None
        };

        // Convert params to Dynamic
        let params_dynamic = json_to_dynamic(&ctx.params);

        // Call the handler function
        let result = self.engine.call_function_dynamic(
            &handler.script_path,
            &handler.handler_function,
            (ctx.to_dynamic(), params_dynamic),
        );

        // Collect mutations before guard drops
        let mutations = if let Some(ref accessor) = self.state_accessor {
            accessor.take_mutations()
        } else {
            Vec::new()
        };

        // Guard drops here automatically

        match result {
            Ok(return_val) => {
                // Parse the return value from the script
                let action_result = parse_action_result(return_val);
                Ok((action_result, mutations))
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Parse the return value from an action handler.
///
/// Expected format:
/// ```rhai
/// #{ success: true, data: #{ ... } }
/// // or
/// #{ success: false, error: "message" }
/// ```
fn parse_action_result(value: Dynamic) -> ActionResult {
    // If the script returns unit/nothing, assume success
    if value.is_unit() {
        return ActionResult::success();
    }

    // If it returns a boolean, use that as success
    if let Some(b) = value.clone().try_cast::<bool>() {
        return if b {
            ActionResult::success()
        } else {
            ActionResult::error("Action failed")
        };
    }

    // Try to parse as a map
    if let Some(map) = value.clone().try_cast::<Map>() {
        let success = map.get("success")
            .and_then(|v| v.clone().try_cast::<bool>())
            .unwrap_or(true);

        let error = map.get("error")
            .and_then(|v| v.clone().try_cast::<String>());

        let data = map.get("data")
            .map(|v| dynamic_to_json(v.clone()));

        return ActionResult {
            success,
            error,
            data,
            mutations: Vec::new(),
        };
    }

    // Unknown return type - assume success
    ActionResult::success()
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

/// Convert a Rhai Dynamic to serde_json::Value.
fn dynamic_to_json(value: Dynamic) -> serde_json::Value {
    if value.is_unit() {
        return serde_json::Value::Null;
    }
    if let Some(b) = value.clone().try_cast::<bool>() {
        return serde_json::Value::Bool(b);
    }
    if let Some(i) = value.clone().try_cast::<i64>() {
        return serde_json::json!(i);
    }
    if let Some(f) = value.clone().try_cast::<f64>() {
        return serde_json::json!(f);
    }
    if let Some(s) = value.clone().try_cast::<String>() {
        return serde_json::Value::String(s);
    }
    if let Some(arr) = value.clone().try_cast::<Vec<Dynamic>>() {
        return serde_json::Value::Array(arr.into_iter().map(dynamic_to_json).collect());
    }
    if let Some(map) = value.clone().try_cast::<Map>() {
        let obj: serde_json::Map<String, serde_json::Value> = map
            .into_iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_json(v)))
            .collect();
        return serde_json::Value::Object(obj);
    }

    // Fallback: try to get string representation
    serde_json::Value::String(format!("{:?}", value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_context_to_dynamic() {
        let ctx = ActionContext {
            player_id: Uuid::new_v4(),
            ship_id: Uuid::new_v4(),
            sector_id: Uuid::new_v4(),
            action: "test".to_string(),
            params: serde_json::json!({}),
        };

        let dynamic = ctx.to_dynamic();
        assert!(dynamic.is::<Map>());

        let map = dynamic.cast::<Map>();
        assert!(map.contains_key("player_id"));
        assert!(map.contains_key("ship_id"));
        assert!(map.contains_key("sector_id"));
        assert!(map.contains_key("action"));
    }

    #[test]
    fn test_action_result_error() {
        let result = ActionResult::error("test error");
        assert!(!result.success);
        assert_eq!(result.error, Some("test error".to_string()));
    }

    #[test]
    fn test_parse_action_result_unit() {
        let result = parse_action_result(Dynamic::UNIT);
        assert!(result.success);
    }

    #[test]
    fn test_parse_action_result_bool() {
        let result = parse_action_result(Dynamic::from(true));
        assert!(result.success);

        let result = parse_action_result(Dynamic::from(false));
        assert!(!result.success);
    }

    #[test]
    fn test_parse_action_result_map() {
        let mut map = Map::new();
        map.insert("success".into(), Dynamic::from(true));
        map.insert("error".into(), Dynamic::UNIT);

        let result = parse_action_result(Dynamic::from(map));
        assert!(result.success);
    }

    #[test]
    fn test_json_to_dynamic_and_back() {
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true,
            "items": [1, 2, 3]
        });

        let dynamic = json_to_dynamic(&json);
        let back = dynamic_to_json(dynamic);

        assert_eq!(json, back);
    }
}
