//! Action API bindings for Rhai
//!
//! Exposes action registration functions to scripts.
//! Allows scripts to register handlers for client actions.

use rhai::{Engine, Array, Map, Dynamic};

use crate::actions::{ActionRegistry, ActionHandler, ActionRequirement};
use crate::context::{current_script_path, with_action_registry};

/// Access action registry from the unified execution context.
fn with_registry<T, F: FnOnce(&ActionRegistry) -> T>(f: F) -> Option<T> {
    with_action_registry(f)
}

/// Parse a requirement from script representation.
fn parse_requirement(value: Dynamic) -> Option<ActionRequirement> {
    // String requirement: "authenticated", "must_be_docked", etc.
    if let Some(s) = value.clone().try_cast::<String>() {
        return match s.to_lowercase().as_str() {
            "authenticated" => Some(ActionRequirement::Authenticated),
            "must_be_docked" | "docked" => Some(ActionRequirement::MustBeDocked),
            "must_be_undocked" | "undocked" => Some(ActionRequirement::MustBeUndocked),
            "not_in_combat" => Some(ActionRequirement::NotInCombat),
            "same_sector" => Some(ActionRequirement::SameSector),
            _ => {
                // Assume it's a custom validator function name
                Some(ActionRequirement::Custom(s))
            }
        };
    }

    // Map requirement: #{ ship_status: ["idle", "docked"] }
    if let Some(map) = value.clone().try_cast::<Map>() {
        if let Some(statuses) = map.get("ship_status") {
            if let Some(arr) = statuses.clone().try_cast::<Array>() {
                let status_list: Vec<String> = arr
                    .into_iter()
                    .filter_map(|v| v.try_cast::<String>())
                    .collect();
                if !status_list.is_empty() {
                    return Some(ActionRequirement::ShipStatus(status_list));
                }
            }
        }
    }

    None
}

/// Register action API functions with the engine.
pub fn register(engine: &mut Engine) {
    // register_action(action: String, handler_fn: String) -> bool
    // Register a simple action handler
    engine.register_fn("register_action", |action: String, handler_fn: String| -> bool {
        let script_path = match current_script_path() {
            Some(p) => p,
            None => {
                tracing::warn!(script = true, "register_action called outside script context");
                return false;
            }
        };

        with_registry(|registry| {
            let handler = ActionHandler::new(action.clone(), script_path, handler_fn);
            registry.register(handler);
            tracing::debug!(script = true, action = %action, "Action registered");
            true
        }).unwrap_or(false)
    });

    // register_action_with_requirements(action: String, handler_fn: String, requirements: Array) -> bool
    // Register an action handler with requirements
    engine.register_fn("register_action_with_requirements",
        |action: String, handler_fn: String, requirements: Array| -> bool {
            let script_path = match current_script_path() {
                Some(p) => p,
                None => {
                    tracing::warn!(script = true, "register_action_with_requirements called outside script context");
                    return false;
                }
            };

            let parsed_reqs: Vec<ActionRequirement> = requirements
                .into_iter()
                .filter_map(parse_requirement)
                .collect();

            with_registry(|registry| {
                let handler = ActionHandler::new(action.clone(), script_path, handler_fn)
                    .with_requirements(parsed_reqs);
                registry.register(handler);
                tracing::debug!(script = true, action = %action, "Action registered with requirements");
                true
            }).unwrap_or(false)
        }
    );

    // unregister_action(action: String) -> bool
    // Unregister an action handler
    engine.register_fn("unregister_action", |action: String| -> bool {
        with_registry(|registry| {
            registry.unregister(&action).is_some()
        }).unwrap_or(false)
    });

    // is_action_registered(action: String) -> bool
    // Check if an action is registered
    engine.register_fn("is_action_registered", |action: String| -> bool {
        with_registry(|registry| {
            registry.is_registered(&action)
        }).unwrap_or(false)
    });

    // list_actions() -> Array
    // List all registered action names
    engine.register_fn("list_actions", || -> Array {
        with_registry(|registry| {
            registry.list_actions()
                .into_iter()
                .map(Dynamic::from)
                .collect()
        }).unwrap_or_default()
    });

    // set_action_enabled(action: String, enabled: bool) -> bool
    // Enable or disable an action handler
    engine.register_fn("set_action_enabled", |action: String, enabled: bool| -> bool {
        with_registry(|registry| {
            if registry.is_registered(&action) {
                registry.set_enabled(&action, enabled);
                true
            } else {
                false
            }
        }).unwrap_or(false)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_requirement_string() {
        assert_eq!(
            parse_requirement(Dynamic::from("authenticated")),
            Some(ActionRequirement::Authenticated)
        );
        assert_eq!(
            parse_requirement(Dynamic::from("must_be_docked")),
            Some(ActionRequirement::MustBeDocked)
        );
        assert_eq!(
            parse_requirement(Dynamic::from("not_in_combat")),
            Some(ActionRequirement::NotInCombat)
        );
    }

    #[test]
    fn test_parse_requirement_custom() {
        assert_eq!(
            parse_requirement(Dynamic::from("my_custom_validator")),
            Some(ActionRequirement::Custom("my_custom_validator".to_string()))
        );
    }

    #[test]
    fn test_parse_requirement_map() {
        let mut map = Map::new();
        let statuses: Array = vec![
            Dynamic::from("idle".to_string()),
            Dynamic::from("docked".to_string()),
        ];
        map.insert("ship_status".into(), Dynamic::from(statuses));

        let req = parse_requirement(Dynamic::from(map));
        assert!(matches!(req, Some(ActionRequirement::ShipStatus(_))));

        if let Some(ActionRequirement::ShipStatus(list)) = req {
            assert_eq!(list, vec!["idle".to_string(), "docked".to_string()]);
        }
    }
}
