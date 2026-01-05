//! Script validation
//!
//! Validates scripts have required functions before runtime execution.
//! Also provides comprehensive action script validation to catch errors early.

use std::path::Path;

use rhai::{AST, Dynamic, Engine};

// ============================================================================
// Action Script Validator
// ============================================================================

/// Result of validating a single action script
#[derive(Debug, Clone)]
pub struct ActionScriptValidation {
    pub path: String,
    pub errors: Vec<ActionValidationError>,
    pub warnings: Vec<ActionValidationWarning>,
    pub functions: Vec<String>,
    pub registered_actions: Vec<RegisteredAction>,
}

impl ActionScriptValidation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// A registered action found in a script
#[derive(Debug, Clone)]
pub struct RegisteredAction {
    pub name: String,
    pub handler: String,
    pub requirements: Vec<String>,
}

/// A validation error (script won't work correctly)
#[derive(Debug, Clone)]
pub struct ActionValidationError {
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
}

/// A validation warning (script may have issues)
#[derive(Debug, Clone)]
pub struct ActionValidationWarning {
    pub code: &'static str,
    pub message: String,
}

/// Result of validating all scripts
#[derive(Debug)]
pub struct ActionValidationReport {
    pub scripts: Vec<ActionScriptValidation>,
    pub total_errors: usize,
    pub total_warnings: usize,
}

impl ActionValidationReport {
    pub fn is_valid(&self) -> bool {
        self.total_errors == 0
    }

    pub fn format_report(&self) -> String {
        let mut output = String::new();

        for script in &self.scripts {
            if !script.errors.is_empty() || !script.warnings.is_empty() {
                output.push_str(&format!("\n=== {} ===\n", script.path));

                for err in &script.errors {
                    let line_info = err.line.map(|l| format!(" (line {})", l)).unwrap_or_default();
                    output.push_str(&format!("  ERROR [{}]{}: {}\n", err.code, line_info, err.message));
                }

                for warn in &script.warnings {
                    output.push_str(&format!("  WARN  [{}]: {}\n", warn.code, warn.message));
                }
            }
        }

        output.push_str(&format!(
            "\nValidation: {} errors, {} warnings across {} scripts\n",
            self.total_errors, self.total_warnings, self.scripts.len()
        ));

        output
    }
}

/// Validator for action scripts
pub struct ActionScriptValidator {
    engine: Engine,
}

impl ActionScriptValidator {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Increase expression depth limits to handle complex scripts
        engine.set_max_expr_depths(128, 128);

        Self::register_stub_api(&mut engine);
        Self { engine }
    }

    /// Register stub API functions so parsing succeeds
    fn register_stub_api(engine: &mut Engine) {
        // Query functions
        engine.register_fn("query_ship", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("query_player", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("query_sector", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("query_location", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("query_mission", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("query_squadron", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("query_combat_engagement", |_: Dynamic| Dynamic::UNIT);

        // Modification functions
        engine.register_fn("modify_ship", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("modify_player", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("modify_mission", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("modify_squadron", |_: Dynamic, _: Dynamic| {});

        // Notifications
        engine.register_fn("send_notification", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("broadcast_to_sector", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("broadcast_to_squadron", |_: Dynamic, _: Dynamic| {});

        // Action registration
        engine.register_fn("register_action", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("register_action_with_requirements", |_: Dynamic, _: Dynamic, _: Dynamic| {});
        engine.register_fn("unregister_action", |_: Dynamic| {});

        // Events
        engine.register_fn("emit_event", |_: Dynamic, _: Dynamic| {});

        // Combat
        engine.register_fn("create_combat_engagement", |_: Dynamic, _: Dynamic, _: Dynamic| Dynamic::UNIT);
        engine.register_fn("end_combat_engagement", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("set_weapon_cooldown", |_: Dynamic, _: Dynamic, _: Dynamic| {});

        // Squadron
        engine.register_fn("create_squadron", |_: Dynamic, _: Dynamic, _: Dynamic| Dynamic::UNIT);
        engine.register_fn("disband_squadron", |_: Dynamic| {});
        engine.register_fn("add_to_squadron", |_: Dynamic, _: Dynamic, _: Dynamic| {});
        engine.register_fn("remove_from_squadron", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("create_squadron_invite", |_: Dynamic, _: Dynamic, _: Dynamic| Dynamic::UNIT);
        engine.register_fn("get_squadron_invite", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("delete_squadron_invite", |_: Dynamic| {});
        engine.register_fn("send_squadron_invite", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("update_squadron_leader", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("squadron_tag_exists", |_: Dynamic| false);

        // Alliance
        engine.register_fn("create_alliance_proposal", |_: Dynamic, _: Dynamic| Dynamic::UNIT);
        engine.register_fn("get_alliance_proposal", |_: Dynamic| Dynamic::UNIT);
        engine.register_fn("delete_alliance_proposal", |_: Dynamic| {});
        engine.register_fn("send_alliance_proposal", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("create_alliance", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("are_squadrons_allied", |_: Dynamic, _: Dynamic| false);
        engine.register_fn("are_squadrons_at_war", |_: Dynamic, _: Dynamic| false);
        engine.register_fn("declare_war", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("create_peace_proposal", |_: Dynamic, _: Dynamic| {});

        // Missions
        engine.register_fn("add_player_mission", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("remove_player_mission", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("record_mission_choice", |_: Dynamic, _: Dynamic| {});

        // Hail
        engine.register_fn("send_hail", |_: Dynamic, _: Dynamic| {});

        // Utility
        engine.register_fn("distance_between", |_: Dynamic, _: Dynamic| 0.0_f64);
        engine.register_fn("current_tick", || 0_i64);
        engine.register_fn("random_float", || 0.5_f64);
        engine.register_fn("random_int", |_: i64, _: i64| 0_i64);
        engine.register_fn("log_info", |_: Dynamic| {});
        engine.register_fn("log_warning", |_: Dynamic| {});
        engine.register_fn("log_error", |_: Dynamic| {});

        // Credits
        engine.register_fn("spend_credits", |_: Dynamic, _: Dynamic| {});
        engine.register_fn("add_credits", |_: Dynamic, _: Dynamic| {});
    }

    /// Validate a single action script file
    pub fn validate_file(&self, path: &Path) -> ActionScriptValidation {
        let path_str = path.display().to_string();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut functions = Vec::new();
        let mut registered_actions = Vec::new();

        // Read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(ActionValidationError {
                    code: "E001",
                    message: format!("Failed to read file: {}", e),
                    line: None,
                });
                return ActionScriptValidation {
                    path: path_str,
                    errors,
                    warnings,
                    functions,
                    registered_actions,
                };
            }
        };

        // Parse script
        let ast = match self.engine.compile(&content) {
            Ok(ast) => ast,
            Err(e) => {
                errors.push(ActionValidationError {
                    code: "E002",
                    message: format!("Syntax error: {}", e),
                    line: e.position().line(),
                });
                return ActionScriptValidation {
                    path: path_str,
                    errors,
                    warnings,
                    functions,
                    registered_actions,
                };
            }
        };

        // Extract function names
        for fn_def in ast.iter_functions() {
            functions.push(fn_def.name.to_string());
        }

        // Check if this is an action script
        let is_action_script = path_str.contains("actions/") || path_str.contains("actions\\");

        if is_action_script {
            // Must have init() function
            if !functions.contains(&"init".to_string()) {
                errors.push(ActionValidationError {
                    code: "E003",
                    message: "Action scripts must have an init() function".to_string(),
                    line: None,
                });
            }

            // Extract registered actions
            registered_actions = self.extract_registered_actions(&content);

            if registered_actions.is_empty() && functions.contains(&"init".to_string()) {
                warnings.push(ActionValidationWarning {
                    code: "W001",
                    message: "init() exists but no actions are registered".to_string(),
                });
            }

            // Verify handlers exist
            for action in &registered_actions {
                if !functions.contains(&action.handler) {
                    errors.push(ActionValidationError {
                        code: "E004",
                        message: format!(
                            "Action '{}' references handler '{}()' which doesn't exist",
                            action.name, action.handler
                        ),
                        line: None,
                    });
                }
            }

            // Check handler signatures (should have ctx and params)
            for action in &registered_actions {
                if functions.contains(&action.handler) {
                    // Find the function in AST and check params
                    for fn_def in ast.iter_functions() {
                        if fn_def.name == action.handler {
                            if fn_def.params.len() != 2 {
                                errors.push(ActionValidationError {
                                    code: "E005",
                                    message: format!(
                                        "Handler '{}()' should have 2 parameters (ctx, params), has {}",
                                        action.handler, fn_def.params.len()
                                    ),
                                    line: None,
                                });
                            }
                            break;
                        }
                    }
                }
            }

            // Check for proper return format in handlers
            for action in &registered_actions {
                if functions.contains(&action.handler) {
                    self.check_handler_returns(&content, &action.handler, &mut warnings);
                }
            }
        }

        ActionScriptValidation {
            path: path_str,
            errors,
            warnings,
            functions,
            registered_actions,
        }
    }

    /// Extract registered actions from script content
    fn extract_registered_actions(&self, content: &str) -> Vec<RegisteredAction> {
        let mut actions = Vec::new();

        // Match register_action("name", "handler")
        let re1 = regex::Regex::new(r#"register_action\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)"#).ok();
        // Match register_action_with_requirements("name", "handler", [...])
        let re2 = regex::Regex::new(r#"register_action_with_requirements\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)""#).ok();

        if let Some(re) = re1 {
            for cap in re.captures_iter(content) {
                actions.push(RegisteredAction {
                    name: cap[1].to_string(),
                    handler: cap[2].to_string(),
                    requirements: vec![],
                });
            }
        }

        if let Some(re) = re2 {
            for cap in re.captures_iter(content) {
                // Don't duplicate if already found
                if !actions.iter().any(|a| a.name == cap[1]) {
                    actions.push(RegisteredAction {
                        name: cap[1].to_string(),
                        handler: cap[2].to_string(),
                        requirements: vec!["(with requirements)".to_string()],
                    });
                }
            }
        }

        actions
    }

    /// Check that handler returns proper result format
    fn check_handler_returns(&self, content: &str, handler: &str, warnings: &mut Vec<ActionValidationWarning>) {
        // Find function body (heuristic)
        let fn_start = format!("fn {}(", handler);
        if let Some(start_idx) = content.find(&fn_start) {
            let after_fn = &content[start_idx..];
            // Find matching closing brace (simplified)
            let end_idx = after_fn.find("\nfn ").unwrap_or(after_fn.len());
            let func_body = &after_fn[..end_idx];

            // Check for success field in return
            if !func_body.contains("success:") && !func_body.contains("success :") {
                warnings.push(ActionValidationWarning {
                    code: "W002",
                    message: format!(
                        "Handler '{}()' may not return proper result (missing 'success:' field)",
                        handler
                    ),
                });
            }

            // Check for error handling
            if !func_body.contains("success: false") && !func_body.contains("success : false") {
                warnings.push(ActionValidationWarning {
                    code: "W003",
                    message: format!(
                        "Handler '{}()' has no error paths (never returns success: false)",
                        handler
                    ),
                });
            }
        }
    }

    /// Validate all scripts in a directory (recursive)
    pub fn validate_directory(&self, dir: &Path) -> ActionValidationReport {
        let mut scripts = Vec::new();
        let mut total_errors = 0;
        let mut total_warnings = 0;

        self.validate_dir_recursive(dir, &mut scripts);

        for script in &scripts {
            total_errors += script.errors.len();
            total_warnings += script.warnings.len();
        }

        ActionValidationReport {
            scripts,
            total_errors,
            total_warnings,
        }
    }

    fn validate_dir_recursive(&self, dir: &Path, scripts: &mut Vec<ActionScriptValidation>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.validate_dir_recursive(&path, scripts);
                } else if path.extension().map(|e| e == "rhai").unwrap_or(false) {
                    scripts.push(self.validate_file(&path));
                }
            }
        }
    }
}

impl Default for ActionScriptValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Original Contract-Based Validation (for other script types)
// ============================================================================

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

    // ========================================================================
    // Action Script Validator Tests
    // ========================================================================

    #[test]
    fn test_action_validator_syntax_error() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_syntax.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, "fn broken( { }").unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "E002"));
    }

    #[test]
    fn test_action_validator_missing_init() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_no_init.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn handle_something(ctx, params) {
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "E003"));
    }

    #[test]
    fn test_action_validator_missing_handler() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_missing_handler.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_nonexistent");
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "E004"));
    }

    #[test]
    fn test_action_validator_wrong_params() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_wrong_params.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx) {
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "E005"));
    }

    #[test]
    fn test_action_validator_valid_script() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_valid.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    if params["value"] == () {
        return #{ success: false, error: "value required" };
    }
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        assert!(result.is_valid(), "Errors: {:?}", result.errors);
    }

    /// This test validates ALL action scripts in scripts/actions/
    #[test]
    fn test_validate_all_action_scripts() {
        let validator = ActionScriptValidator::new();

        // Get scripts/actions directory relative to workspace root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let actions_dir = std::path::PathBuf::from(manifest_dir)
            .parent().unwrap()
            .parent().unwrap()
            .join("scripts")
            .join("actions");

        if !actions_dir.exists() {
            eprintln!("Actions directory not found at {:?}, skipping", actions_dir);
            return;
        }

        let report = validator.validate_directory(&actions_dir);

        // Print report for visibility
        if !report.is_valid() || report.total_warnings > 0 {
            eprintln!("{}", report.format_report());
        }

        // Fail on errors
        assert!(
            report.is_valid(),
            "Action script validation failed with {} errors. See output above.",
            report.total_errors
        );

        // Log success with stats
        let total_actions: usize = report.scripts.iter()
            .map(|s| s.registered_actions.len())
            .sum();
        eprintln!(
            "Validated {} action scripts, {} registered actions, {} warnings",
            report.scripts.len(),
            total_actions,
            report.total_warnings
        );
    }

    /// This test validates ALL scripts in the project (informational, warnings allowed)
    #[test]
    fn test_validate_all_scripts() {
        let validator = ActionScriptValidator::new();

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let scripts_dir = std::path::PathBuf::from(manifest_dir)
            .parent().unwrap()
            .parent().unwrap()
            .join("scripts");

        if !scripts_dir.exists() {
            eprintln!("Scripts directory not found at {:?}, skipping", scripts_dir);
            return;
        }

        let report = validator.validate_directory(&scripts_dir);

        // Print report (informational)
        eprintln!("{}", report.format_report());

        // Note: This test is informational - it doesn't fail on errors in non-action scripts
        // as those may have different requirements. Action script validation is strict.
        eprintln!(
            "Total: {} scripts, {} errors, {} warnings",
            report.scripts.len(),
            report.total_errors,
            report.total_warnings
        );
    }
}
