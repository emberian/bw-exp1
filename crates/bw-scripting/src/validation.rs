//! Script validation
//!
//! Validates scripts have required functions before runtime execution.

use rhai::{AST, Engine};

/// Contract defining expected functions for a script type.
#[derive(Debug, Clone)]
pub struct ScriptContract {
    /// Functions that must be present
    pub required: Vec<FunctionSpec>,
    /// Functions that are optional but recognized
    pub optional: Vec<FunctionSpec>,
}

/// Specification of an expected function.
#[derive(Debug, Clone)]
pub struct FunctionSpec {
    /// Function name
    pub name: String,
    /// Expected number of parameters (None = any)
    pub param_count: Option<usize>,
    /// Description for error messages
    pub description: String,
}

impl FunctionSpec {
    /// Create a new function specification.
    pub fn new(name: impl Into<String>, param_count: Option<usize>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_count,
            description: description.into(),
        }
    }

    /// Create a spec requiring exact parameter count.
    pub fn with_params(name: impl Into<String>, params: usize, description: impl Into<String>) -> Self {
        Self::new(name, Some(params), description)
    }

    /// Create a spec with any parameter count.
    pub fn any_params(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(name, None, description)
    }
}

/// Result of validating a script.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Script name that was validated
    pub script: String,
    /// Missing required functions
    pub missing_required: Vec<FunctionSpec>,
    /// Functions with wrong parameter counts
    pub wrong_params: Vec<(FunctionSpec, usize)>,
    /// Warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a new empty result.
    pub fn new(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            missing_required: Vec::new(),
            wrong_params: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        self.missing_required.is_empty() && self.wrong_params.is_empty()
    }

    /// Get a formatted error message.
    pub fn error_message(&self) -> Option<String> {
        if self.is_valid() {
            return None;
        }

        let mut msg = format!("Script '{}' validation failed:", self.script);

        for func in &self.missing_required {
            msg.push_str(&format!("\n  - Missing required function: {} ({})", func.name, func.description));
        }

        for (func, actual) in &self.wrong_params {
            let expected = func.param_count.unwrap_or(0);
            msg.push_str(&format!(
                "\n  - Function '{}' has {} parameters, expected {} ({})",
                func.name, actual, expected, func.description
            ));
        }

        Some(msg)
    }
}

/// Predefined contracts for different script types.
pub mod contracts {
    use super::*;

    /// Contract for mission scripts.
    pub fn mission() -> ScriptContract {
        ScriptContract {
            required: vec![
                FunctionSpec::with_params("on_start", 1, "Called when mission begins, receives context"),
                FunctionSpec::with_params("on_choice", 2, "Called when player makes a choice, receives context and choice_id"),
            ],
            optional: vec![
                FunctionSpec::with_params("on_combat_resolved", 2, "Called after combat ends, receives context and combat_result"),
                FunctionSpec::with_params("on_tick", 2, "Called each tick for timed missions, receives context and elapsed_seconds"),
            ],
        }
    }

    /// Contract for behavior scripts.
    pub fn behavior() -> ScriptContract {
        ScriptContract {
            required: vec![
                // on_spawn is strongly recommended but we'll make on_update the only requirement
                // since some behaviors might only respond to events
            ],
            optional: vec![
                FunctionSpec::with_params("on_spawn", 1, "Called when behavior attaches, receives context"),
                FunctionSpec::with_params("on_update", 1, "Called each tick, receives context"),
                FunctionSpec::with_params("on_destroy", 1, "Called when behavior detaches, receives context"),
                FunctionSpec::with_params("on_event", 2, "Called for game events, receives context and event"),
            ],
        }
    }

    /// Contract for combat AI scripts.
    pub fn combat_ai() -> ScriptContract {
        ScriptContract {
            required: vec![],
            optional: vec![
                FunctionSpec::with_params("pirate_combat_ai", 3, "AI for pirate NPCs"),
                FunctionSpec::with_params("sera_combat_ai", 3, "AI for Sera NPCs"),
                FunctionSpec::with_params("civilian_combat_ai", 3, "AI for civilian NPCs"),
                FunctionSpec::with_params("military_combat_ai", 3, "AI for military NPCs"),
                FunctionSpec::with_params("drone_combat_ai", 3, "AI for drone NPCs"),
            ],
        }
    }

    /// Minimal contract (no requirements).
    pub fn minimal() -> ScriptContract {
        ScriptContract {
            required: vec![],
            optional: vec![],
        }
    }
}

/// Validate a script against a contract.
pub fn validate_script(
    _engine: &Engine,
    ast: &AST,
    contract: &ScriptContract,
    script_name: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new(script_name);

    // Get all function definitions from the AST
    let functions: Vec<_> = ast.iter_functions().collect();

    // Check required functions
    for spec in &contract.required {
        let found = functions.iter().find(|f| f.name == spec.name);

        match found {
            None => {
                result.missing_required.push(spec.clone());
            }
            Some(f) => {
                if let Some(expected_params) = spec.param_count {
                    if f.params.len() != expected_params {
                        result.wrong_params.push((spec.clone(), f.params.len()));
                    }
                }
            }
        }
    }

    // Check optional functions for parameter mismatches (warnings only)
    for spec in &contract.optional {
        if let Some(f) = functions.iter().find(|f| f.name == spec.name) {
            if let Some(expected_params) = spec.param_count {
                if f.params.len() != expected_params {
                    result.warnings.push(format!(
                        "Optional function '{}' has {} parameters, expected {} ({})",
                        spec.name, f.params.len(), expected_params, spec.description
                    ));
                }
            }
        }
    }

    result
}

/// Infer the contract type from a script path.
pub fn infer_contract(script_path: &str) -> ScriptContract {
    if script_path.contains("missions/") {
        contracts::mission()
    } else if script_path.contains("behaviors/") || script_path.contains("ai/") {
        contracts::behavior()
    } else if script_path.contains("combat/") {
        contracts::combat_ai()
    } else {
        contracts::minimal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_spec() {
        let spec = FunctionSpec::with_params("on_start", 1, "Test function");
        assert_eq!(spec.name, "on_start");
        assert_eq!(spec.param_count, Some(1));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new("test.rhai");
        assert!(result.is_valid());

        result.missing_required.push(FunctionSpec::with_params("on_start", 1, "Test"));
        assert!(!result.is_valid());
        assert!(result.error_message().is_some());
    }

    #[test]
    fn test_infer_contract() {
        let mission_contract = infer_contract("missions/random/pirate.rhai");
        assert!(!mission_contract.required.is_empty());

        let behavior_contract = infer_contract("ai/npc_behaviors.rhai");
        assert!(behavior_contract.required.is_empty()); // Behaviors have no strict requirements
    }
}
