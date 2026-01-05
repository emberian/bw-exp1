//! Action handler registry
//!
//! Stores registered action handlers that scripts can define.

use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

/// A registered action handler.
#[derive(Debug, Clone)]
pub struct ActionHandler {
    /// Unique ID for this handler registration
    pub id: Uuid,
    /// Action name (e.g., "dock", "accept_mission", "attack")
    pub action: String,
    /// Script file containing the handler
    pub script_path: String,
    /// Function name to call (receives ctx, params)
    pub handler_function: String,
    /// Requirements that must be met before action can execute
    pub requirements: Vec<ActionRequirement>,
    /// Whether this handler is currently enabled
    pub enabled: bool,
}

impl ActionHandler {
    /// Create a new action handler.
    pub fn new(
        action: impl Into<String>,
        script_path: impl Into<String>,
        handler_function: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            action: action.into(),
            script_path: script_path.into(),
            handler_function: handler_function.into(),
            requirements: Vec::new(),
            enabled: true,
        }
    }

    /// Add requirements to this handler.
    pub fn with_requirements(mut self, requirements: Vec<ActionRequirement>) -> Self {
        self.requirements = requirements;
        self
    }

    /// Add a single requirement.
    pub fn with_requirement(mut self, requirement: ActionRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }
}

/// Requirements that must be met before an action can execute.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionRequirement {
    /// Player must be authenticated (always true for valid sessions)
    Authenticated,
    /// Player's ship must be in one of these statuses
    ShipStatus(Vec<String>),
    /// Player must be in the same sector as their target
    SameSector,
    /// Player must be docked at a station
    MustBeDocked,
    /// Player must NOT be docked
    MustBeUndocked,
    /// Player must NOT be in combat
    NotInCombat,
    /// Custom validation (function name in the handler script)
    Custom(String),
}

/// Registry of action handlers.
pub struct ActionRegistry {
    /// Handlers indexed by action name
    handlers: RwLock<HashMap<String, ActionHandler>>,
    /// Handlers by script path for cleanup
    by_script: RwLock<HashMap<String, Vec<String>>>,
}

impl ActionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            by_script: RwLock::new(HashMap::new()),
        }
    }

    /// Register an action handler.
    ///
    /// Returns the previous handler if one was already registered for this action.
    pub fn register(&self, handler: ActionHandler) -> Option<ActionHandler> {
        let action = handler.action.clone();
        let script_path = handler.script_path.clone();

        tracing::debug!(
            action = %action,
            script = %script_path,
            function = %handler.handler_function,
            "Registering action handler"
        );

        // Track by script for cleanup
        self.by_script.write()
            .entry(script_path)
            .or_default()
            .push(action.clone());

        // Insert handler
        self.handlers.write().insert(action, handler)
    }

    /// Unregister an action handler.
    pub fn unregister(&self, action: &str) -> Option<ActionHandler> {
        let handler = self.handlers.write().remove(action);

        if let Some(ref h) = handler {
            // Remove from script index
            if let Some(actions) = self.by_script.write().get_mut(&h.script_path) {
                actions.retain(|a| a != action);
            }
            tracing::debug!(action = %action, "Action handler unregistered");
        }

        handler
    }

    /// Unregister all handlers from a specific script.
    pub fn unregister_for_script(&self, script_path: &str) {
        let actions: Vec<String> = self.by_script.write()
            .remove(script_path)
            .unwrap_or_default();

        let mut handlers = self.handlers.write();
        for action in actions {
            handlers.remove(&action);
        }

        tracing::debug!(script = %script_path, "All action handlers for script unregistered");
    }

    /// Get a handler by action name.
    pub fn get(&self, action: &str) -> Option<ActionHandler> {
        self.handlers.read().get(action).cloned()
    }

    /// Check if an action is registered.
    pub fn is_registered(&self, action: &str) -> bool {
        self.handlers.read().contains_key(action)
    }

    /// Enable or disable a handler.
    pub fn set_enabled(&self, action: &str, enabled: bool) {
        if let Some(handler) = self.handlers.write().get_mut(action) {
            handler.enabled = enabled;
        }
    }

    /// Get count of registered handlers.
    pub fn count(&self) -> usize {
        self.handlers.read().len()
    }

    /// List all registered action names.
    pub fn list_actions(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }

    /// List all handlers.
    pub fn list_all(&self) -> Vec<ActionHandler> {
        self.handlers.read().values().cloned().collect()
    }

    /// List handlers for a script.
    pub fn list_for_script(&self, script_path: &str) -> Vec<ActionHandler> {
        let actions = self.by_script.read()
            .get(script_path)
            .cloned()
            .unwrap_or_default();

        let handlers = self.handlers.read();
        actions.iter()
            .filter_map(|a| handlers.get(a).cloned())
            .collect()
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_unregister() {
        let registry = ActionRegistry::new();

        let handler = ActionHandler::new("dock", "station.rhai", "handle_dock");
        registry.register(handler);

        assert!(registry.is_registered("dock"));
        assert_eq!(registry.count(), 1);

        let h = registry.get("dock").unwrap();
        assert_eq!(h.action, "dock");
        assert_eq!(h.script_path, "station.rhai");
        assert_eq!(h.handler_function, "handle_dock");

        registry.unregister("dock");
        assert!(!registry.is_registered("dock"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_unregister_for_script() {
        let registry = ActionRegistry::new();

        registry.register(ActionHandler::new("dock", "station.rhai", "handle_dock"));
        registry.register(ActionHandler::new("undock", "station.rhai", "handle_undock"));
        registry.register(ActionHandler::new("attack", "combat.rhai", "handle_attack"));

        assert_eq!(registry.count(), 3);

        registry.unregister_for_script("station.rhai");

        assert_eq!(registry.count(), 1);
        assert!(!registry.is_registered("dock"));
        assert!(!registry.is_registered("undock"));
        assert!(registry.is_registered("attack"));
    }

    #[test]
    fn test_requirements() {
        let handler = ActionHandler::new("dock", "station.rhai", "handle_dock")
            .with_requirements(vec![
                ActionRequirement::Authenticated,
                ActionRequirement::ShipStatus(vec!["idle".into()]),
            ]);

        assert_eq!(handler.requirements.len(), 2);
    }
}
