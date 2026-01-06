//! Script validators for action and definition scripts.
//!
//! Provides comprehensive validation including:
//! - Action script validation (E001-E900, W001-W960)
//! - Definition script validation (E500, W500-W501)
//! - Contract-based validation for mission/behavior scripts

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rhai::{AST, ASTNode, Dynamic, Engine, Expr};

use crate::schema::{ActionSchemaRegistry, DefinitionSchema, DefinitionSchemaRegistry};

use super::analysis::{
    analyze_ast, analyze_function, archetype_category, AstAnalysis, ArchetypeRegistry,
};
use super::api_functions::{get_api_function, is_known_event_type, CTX_KEYS};
use super::traits::{
    ScriptValidation, ScriptValidator, ValidationError, ValidationReport, ValidationWarning,
};
use super::types::InferredType;

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

impl ScriptValidation for ActionScriptValidation {
    type Error = ActionValidationError;
    type Warning = ActionValidationWarning;

    fn path(&self) -> &str {
        &self.path
    }
    fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    fn errors(&self) -> &[Self::Error] {
        &self.errors
    }
    fn warnings(&self) -> &[Self::Warning] {
        &self.warnings
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

impl ValidationError for ActionValidationError {
    fn code(&self) -> &str {
        self.code
    }
    fn message(&self) -> &str {
        &self.message
    }
    fn line(&self) -> Option<usize> {
        self.line
    }
}

/// A validation warning (script may have issues)
#[derive(Debug, Clone)]
pub struct ActionValidationWarning {
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
}

impl ValidationWarning for ActionValidationWarning {
    fn code(&self) -> &str {
        self.code
    }
    fn message(&self) -> &str {
        &self.message
    }
    fn line(&self) -> Option<usize> {
        self.line
    }
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
                    let line_info = warn.line.map(|l| format!(" (line {})", l)).unwrap_or_default();
                    output.push_str(&format!("  WARN  [{}]{}: {}\n", warn.code, line_info, warn.message));
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

impl ValidationReport for ActionValidationReport {
    type Validation = ActionScriptValidation;

    fn scripts(&self) -> &[Self::Validation] {
        &self.scripts
    }
    fn total_errors(&self) -> usize {
        self.total_errors
    }
    fn total_warnings(&self) -> usize {
        self.total_warnings
    }
    fn format_report(&self) -> String {
        ActionValidationReport::format_report(self)
    }
}

/// Validator for action scripts
pub struct ActionScriptValidator {
    engine: Engine,
    action_schema: Option<ActionSchemaRegistry>,
    archetype_registry: Option<ArchetypeRegistry>,
}

impl ActionScriptValidator {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Increase expression depth limits to handle complex scripts
        engine.set_max_expr_depths(128, 128);

        Self::register_stub_api(&mut engine);
        Self { engine, action_schema: None, archetype_registry: None }
    }

    /// Create a validator with an action schema registry for param validation.
    pub fn with_schema(mut self, schema: ActionSchemaRegistry) -> Self {
        self.action_schema = Some(schema);
        self
    }

    /// Create a validator with an archetype registry for W900 validation.
    pub fn with_archetype_registry(mut self, registry: ArchetypeRegistry) -> Self {
        self.archetype_registry = Some(registry);
        self
    }

    /// Try to load the default action schema from scripts/schemas/actions.toml.
    /// Returns self unchanged if the file doesn't exist or fails to parse.
    pub fn with_default_schema(self) -> Self {
        // Try common paths relative to cargo manifest or current dir
        let paths = [
            "scripts/schemas/actions.toml",
            "../scripts/schemas/actions.toml",
            "../../scripts/schemas/actions.toml",
        ];

        for path in paths {
            if let Ok(schema) = ActionSchemaRegistry::load_from_file(path) {
                return self.with_schema(schema);
            }
        }

        self
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
                    line: None,
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

            // === Advanced Analysis ===
            // Use AST-based analysis for accurate validation
            let ast_analysis = analyze_ast(&ast, &content);

            // Check API function argument counts
            self.check_api_calls(&ast_analysis, &mut errors, &mut warnings);

            // Check context key accesses
            self.check_ctx_keys(&ast_analysis, &mut warnings);

            // Check variable flow and param access in each handler
            for action in &registered_actions {
                if functions.contains(&action.handler) {
                    // AST-based comprehensive analysis (E301, E600, W100, W200, W401, W402, W700, E700, etc.)
                    self.check_handler_with_ast(&ast, &action.handler, &mut errors, &mut warnings);
                    // Schema-based param validation
                    self.check_param_keys(&content, &action.name, &action.handler, &mut warnings);
                }
            }

            // Also check init() for variable issues
            if functions.contains(&"init".to_string()) {
                self.check_handler_with_ast(&ast, "init", &mut errors, &mut warnings);
            }

            // === Event Subscription Validation ===
            self.check_event_subscriptions(&ast_analysis, &functions, &mut errors, &mut warnings);

            // === Dynamic Key Warnings ===
            self.check_dynamic_keys(&ast_analysis, &mut warnings);
        }

        ActionScriptValidation {
            path: path_str,
            errors,
            warnings,
            functions,
            registered_actions,
        }
    }

    /// Validate API function calls have correct argument counts.
    fn check_api_calls(
        &self,
        analysis: &AstAnalysis,
        errors: &mut Vec<ActionValidationError>,
        _warnings: &mut Vec<ActionValidationWarning>,
    ) {
        // Track which functions we've seen (for deduplication)
        let mut seen_fns = HashSet::new();

        for (fn_name, arg_count, pos) in &analysis.fn_calls {
            // Skip if we've already reported this function
            if seen_fns.contains(fn_name) {
                continue;
            }

            if let Some(api_fn) = get_api_function(fn_name) {
                // Note: arg_count from text analysis is heuristic, so only warn if it's way off
                // The text analysis doesn't count args accurately, so we skip this check
                // when using text-based analysis (arg_count == 0 is our sentinel)
                if *arg_count > 0 && *arg_count != api_fn.arg_count {
                    errors.push(ActionValidationError {
                        code: "E200",
                        message: format!(
                            "Function '{}' expects {} arguments, got {}",
                            fn_name, api_fn.arg_count, arg_count
                        ),
                        line: pos.and_then(|p| p.line()),
                    });
                }
                seen_fns.insert(fn_name.clone());
            }
        }
    }

    /// Validate context key accesses use known keys.
    fn check_ctx_keys(
        &self,
        analysis: &AstAnalysis,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        for (var, key, pos) in &analysis.index_accesses {
            // Check ctx["key"] accesses
            if var == "ctx" && !CTX_KEYS.contains(&key.as_str()) {
                warnings.push(ActionValidationWarning {
                    code: "W400",
                    message: format!(
                        "Unknown context key '{}'. Valid keys: {:?}",
                        key, CTX_KEYS
                    ),
                    line: pos.and_then(|p| p.line()),
                });
            }
        }
    }

    /// Validate event subscriptions have valid handlers and event types.
    fn check_event_subscriptions(
        &self,
        analysis: &AstAnalysis,
        functions: &[String],
        errors: &mut Vec<ActionValidationError>,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        for sub in &analysis.event_subscriptions {
            // E800: Handler function doesn't exist
            if !functions.contains(&sub.handler_fn) {
                errors.push(ActionValidationError {
                    code: "E800",
                    message: format!(
                        "Event subscription references handler '{}()' which doesn't exist",
                        sub.handler_fn
                    ),
                    line: sub.position.and_then(|p| p.line()),
                });
            }

            // W800: Unknown event type
            // Skip if event_type is "*" (dynamic array)
            if sub.event_type != "*" && !is_known_event_type(&sub.event_type) {
                warnings.push(ActionValidationWarning {
                    code: "W800",
                    message: format!(
                        "Unknown event type '{}'. Check spelling or verify it's a custom event",
                        sub.event_type
                    ),
                    line: sub.position.and_then(|p| p.line()),
                });
            }
        }
    }

    /// Warn about dynamic key accesses that cannot be validated at compile time.
    fn check_dynamic_keys(
        &self,
        analysis: &AstAnalysis,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        // Only warn for ctx and params - dynamic keys on other maps might be intentional
        let important_maps = ["ctx", "params"];

        for (var_name, pos) in &analysis.dynamic_key_accesses {
            if important_maps.contains(&var_name.as_str()) {
                warnings.push(ActionValidationWarning {
                    code: "W960",
                    message: format!(
                        "Dynamic key access on '{}' cannot be validated at compile time. \
                         Consider using a literal string key if possible",
                        var_name
                    ),
                    line: pos.and_then(|p| p.line()),
                });
            }
        }
    }

    /// Validate params["key"] accesses against action schema.
    fn check_param_keys(
        &self,
        content: &str,
        action_name: &str,
        handler_name: &str,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        let Some(schema) = &self.action_schema else {
            return; // No schema loaded, skip validation
        };

        let Some(action_schema) = schema.get(action_name) else {
            return; // Unknown action, skip (might be custom)
        };

        // Extract handler function body and find params["key"] accesses
        let fn_pattern = format!("fn {}(", handler_name);
        let Some(fn_start) = content.find(&fn_pattern) else {
            return;
        };

        let params_start = fn_start + fn_pattern.len();
        let Some(params_end) = content[params_start..].find(')') else {
            return;
        };

        let body_start = params_start + params_end + 1;
        let body_end = content[body_start..].find("\nfn ").unwrap_or(content.len() - body_start);
        let func_body = &content[body_start..body_start + body_end];

        // Find params["key"] accesses in this handler
        let params_re = regex::Regex::new(r#"params\s*\[\s*"([^"]+)"\s*\]"#).ok();
        if let Some(re) = params_re {
            for cap in re.captures_iter(func_body) {
                if let Some(key) = cap.get(1) {
                    let key_str = key.as_str();
                    if !action_schema.has_param(key_str) {
                        let valid_params: Vec<_> = action_schema.params.keys().collect();
                        warnings.push(ActionValidationWarning {
                            code: "E100",
                            message: format!(
                                "In '{}()': Unknown param '{}' for action '{}'. Valid params: {:?}",
                                handler_name, key_str, action_name, valid_params
                            ),
                            line: None,
                        });
                    }
                }
            }
        }

        // Check if required params are accessed
        for param_name in action_schema.required_params() {
            let access_pattern = format!(r#"params\s*\[\s*"{}"\s*\]"#, regex::escape(param_name));
            if regex::Regex::new(&access_pattern).ok()
                .is_none_or(|re| !re.is_match(func_body))
            {
                warnings.push(ActionValidationWarning {
                    code: "W101",
                    message: format!(
                        "In '{}()': Required param '{}' for action '{}' is not accessed",
                        handler_name, param_name, action_name
                    ),
                    line: None,
                });
            }
        }
    }

    /// Check handler using AST-based FunctionAnalyzer for comprehensive analysis.
    /// This provides E301 (return path completeness) checking.
    fn check_handler_with_ast(
        &self,
        ast: &AST,
        handler_name: &str,
        errors: &mut Vec<ActionValidationError>,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        let analysis = analyze_function(ast, handler_name);

        // E301: Handler may exit without returning
        // Skip for init() - it's a setup function that doesn't need to return
        if !analysis.all_paths_return && handler_name != "init" {
            // Make this a warning rather than error - some handlers may legitimately
            // have paths that don't return (e.g., early exit conditions handled elsewhere)
            warnings.push(ActionValidationWarning {
                code: "W301",
                message: format!(
                    "Handler '{}()' may exit without returning a value on all code paths",
                    handler_name
                ),
                line: None,
            });
        }

        // Check for return statements without success field
        // (supplement the existing check_handler_returns)
        for keys in &analysis.return_map_keys {
            if !keys.contains(&"success".to_string()) {
                warnings.push(ActionValidationWarning {
                    code: "W003",
                    message: format!(
                        "Handler '{}()' return map is missing 'success' field",
                        handler_name
                    ),
                    line: None,
                });
            }
        }

        // W302: Check return field types
        for fields in &analysis.return_map_fields {
            for field in fields {
                if field.name == "success" && field.value_type != InferredType::Bool {
                    warnings.push(ActionValidationWarning {
                        code: "W302",
                        message: format!(
                            "In '{}()': Return field 'success' should be Bool, got {:?}",
                            handler_name, field.value_type
                        ),
                        line: field.position.and_then(|p| p.line()),
                    });
                }
                if field.name == "error" && field.value_type != InferredType::String && field.value_type != InferredType::Dynamic {
                    warnings.push(ActionValidationWarning {
                        code: "W302",
                        message: format!(
                            "In '{}()': Return field 'error' should be String, got {:?}",
                            handler_name, field.value_type
                        ),
                        line: field.position.and_then(|p| p.line()),
                    });
                }
            }
        }

        // AST-based undefined variable detection (more accurate than text-based)
        for (var_name, pos) in &analysis.undefined_vars {
            errors.push(ActionValidationError {
                code: "E600",
                message: format!(
                    "In '{}()': Use of undefined variable '{}'",
                    handler_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // AST-based unused variable detection
        for (var_name, pos) in &analysis.unused_vars {
            warnings.push(ActionValidationWarning {
                code: "W401",
                message: format!(
                    "In '{}()': Variable '{}' is declared but never used",
                    handler_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // AST-based unchecked nullable detection
        for (var_name, pos) in &analysis.unchecked_nullables {
            warnings.push(ActionValidationWarning {
                code: "W100",
                message: format!(
                    "In '{}()': Accessing potentially null '{}' without check",
                    handler_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // AST-based dead code detection
        for pos in &analysis.dead_code {
            warnings.push(ActionValidationWarning {
                code: "W200",
                message: format!(
                    "In '{}()': Unreachable code detected",
                    handler_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // AST-based shadowing detection
        for (var_name, pos) in &analysis.shadowed_vars {
            warnings.push(ActionValidationWarning {
                code: "W402",
                message: format!(
                    "In '{}()': Variable '{}' shadows a variable in outer scope",
                    handler_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // E700: Type mismatches in binary operations
        for (msg, pos) in &analysis.type_mismatches {
            errors.push(ActionValidationError {
                code: "E700",
                message: format!(
                    "In '{}()': Type error - {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // E701: Index type mismatches (e.g., arr[floor(x)] where floor returns Float)
        for (msg, pos) in &analysis.index_type_errors {
            errors.push(ActionValidationError {
                code: "E701",
                message: format!(
                    "In '{}()': Index type error - {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // E702: Boolean operator requires Bool operand
        for (msg, pos) in &analysis.boolean_op_errors {
            errors.push(ActionValidationError {
                code: "E702",
                message: format!(
                    "In '{}()': Boolean operator error - {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // W700: Implicit type coercions
        for (msg, pos) in &analysis.implicit_coercions {
            warnings.push(ActionValidationWarning {
                code: "W700",
                message: format!(
                    "In '{}()': {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // E900: Invalid archetype IDs (empty string)
        for (msg, pos) in &analysis.invalid_archetype_ids {
            errors.push(ActionValidationError {
                code: "E900",
                message: format!(
                    "In '{}()': Invalid archetype ID - {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // W900: Unknown archetype IDs (check against registry if available)
        if let Some(registry) = &self.archetype_registry {
            for (fn_name, id, pos) in &analysis.archetype_lookups {
                if let Some(category) = archetype_category(fn_name)
                    && !registry.contains(category, id) {
                        warnings.push(ActionValidationWarning {
                            code: "W900",
                            message: format!(
                                "In '{}()': Archetype ID '{}' not found in {} definitions",
                                handler_name, id, category
                            ),
                            line: pos.and_then(|p| p.line()),
                        });
                    }
            }
        }

        // E501: Invalid nested field accesses (field doesn't exist in schema)
        for (msg, pos) in &analysis.invalid_nested_accesses {
            errors.push(ActionValidationError {
                code: "E501",
                message: format!(
                    "In '{}()': {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // W502: Dynamic nested accesses that can't be validated
        for (msg, pos) in &analysis.dynamic_nested_accesses {
            warnings.push(ActionValidationWarning {
                code: "W502",
                message: format!(
                    "In '{}()': {}",
                    handler_name, msg
                ),
                line: pos.and_then(|p| p.line()),
            });
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
                    line: None,
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
                    line: None,
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

        // Cross-script validation (E950/W950)
        self.check_cross_script_dependencies(&mut scripts);

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

    /// Check for cross-script issues like duplicate action registrations.
    fn check_cross_script_dependencies(&self, scripts: &mut [ActionScriptValidation]) {
        // Build map of action_name -> (script_path, handler_name)
        let mut action_registry: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for script in scripts.iter() {
            for action in &script.registered_actions {
                action_registry
                    .entry(action.name.clone())
                    .or_default()
                    .push((script.path.clone(), action.handler.clone()));
            }
        }

        // W950: Duplicate action registrations
        for (action_name, registrations) in &action_registry {
            if registrations.len() > 1 {
                // Multiple scripts register the same action - warn all of them
                let providers: Vec<String> = registrations.iter()
                    .map(|(path, _)| {
                        // Extract just the filename for readability
                        Path::new(path)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(path)
                            .to_string()
                    })
                    .collect();

                let warning_msg = format!(
                    "Action '{}' is registered in multiple scripts: {}",
                    action_name,
                    providers.join(", ")
                );

                // Add warning to the first script that registers it
                // (to avoid cluttering all scripts with the same warning)
                if let Some((first_path, _)) = registrations.first() {
                    if let Some(script) = scripts.iter_mut().find(|s| &s.path == first_path) {
                        script.warnings.push(ActionValidationWarning {
                            code: "W950",
                            message: warning_msg,
                            line: None,
                        });
                    }
                }
            }
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

impl ScriptValidator for ActionScriptValidator {
    type Validation = ActionScriptValidation;
    type Report = ActionValidationReport;

    fn validate_file(&self, path: &Path) -> Self::Validation {
        ActionScriptValidator::validate_file(self, path)
    }

    fn validate_directory(&self, dir: &Path) -> Self::Report {
        ActionScriptValidator::validate_directory(self, dir)
    }
}

// ============================================================================
// Definition Script Validator
// ============================================================================

/// Result of validating a single definition script
#[derive(Debug, Clone)]
pub struct DefinitionScriptValidation {
    pub path: String,
    pub errors: Vec<DefinitionValidationError>,
    pub warnings: Vec<DefinitionValidationWarning>,
    pub definitions: Vec<DefinitionInfo>,
}

impl DefinitionScriptValidation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl ScriptValidation for DefinitionScriptValidation {
    type Error = DefinitionValidationError;
    type Warning = DefinitionValidationWarning;

    fn path(&self) -> &str {
        &self.path
    }
    fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    fn errors(&self) -> &[Self::Error] {
        &self.errors
    }
    fn warnings(&self) -> &[Self::Warning] {
        &self.warnings
    }
}

/// Information about a definition found in a script
#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    pub id: String,
    pub function: String,
    pub fields: Vec<String>,
}

/// A validation error for definition scripts
#[derive(Debug, Clone)]
pub struct DefinitionValidationError {
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
}

impl ValidationError for DefinitionValidationError {
    fn code(&self) -> &str {
        self.code
    }
    fn message(&self) -> &str {
        &self.message
    }
    fn line(&self) -> Option<usize> {
        self.line
    }
}

/// A validation warning for definition scripts
#[derive(Debug, Clone)]
pub struct DefinitionValidationWarning {
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
}

impl ValidationWarning for DefinitionValidationWarning {
    fn code(&self) -> &str {
        self.code
    }
    fn message(&self) -> &str {
        &self.message
    }
    fn line(&self) -> Option<usize> {
        self.line
    }
}

/// Result of validating all definition scripts
#[derive(Debug)]
pub struct DefinitionValidationReport {
    pub scripts: Vec<DefinitionScriptValidation>,
    pub total_errors: usize,
    pub total_warnings: usize,
}

impl DefinitionValidationReport {
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
                    let line_info = warn.line.map(|l| format!(" (line {})", l)).unwrap_or_default();
                    output.push_str(&format!("  WARN  [{}]{}: {}\n", warn.code, line_info, warn.message));
                }
            }
        }

        output.push_str(&format!(
            "\nDefinition Validation: {} errors, {} warnings across {} scripts\n",
            self.total_errors, self.total_warnings, self.scripts.len()
        ));

        output
    }
}

impl ValidationReport for DefinitionValidationReport {
    type Validation = DefinitionScriptValidation;

    fn scripts(&self) -> &[Self::Validation] {
        &self.scripts
    }
    fn total_errors(&self) -> usize {
        self.total_errors
    }
    fn total_warnings(&self) -> usize {
        self.total_warnings
    }
    fn format_report(&self) -> String {
        DefinitionValidationReport::format_report(self)
    }
}

/// Validator for definition scripts (ships.rhai, weapons.rhai, etc.)
pub struct DefinitionScriptValidator {
    engine: Engine,
    schema_registry: DefinitionSchemaRegistry,
}

impl DefinitionScriptValidator {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(128, 128);
        Self::register_stub_api(&mut engine);
        Self {
            engine,
            schema_registry: DefinitionSchemaRegistry::with_builtins(),
        }
    }

    /// Create with a custom schema registry.
    pub fn with_schema_registry(schema_registry: DefinitionSchemaRegistry) -> Self {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(128, 128);
        Self::register_stub_api(&mut engine);
        Self { engine, schema_registry }
    }

    /// Register stub API functions for parsing
    fn register_stub_api(engine: &mut Engine) {
        // Just enough to compile definition scripts
        engine.register_fn("merge", |_: Dynamic, _: Dynamic| Dynamic::UNIT);
        engine.register_fn("scale_stats", |_: Dynamic, _: Dynamic| Dynamic::UNIT);
    }

    /// Validate a single definition script file
    pub fn validate_file(&self, path: &Path) -> DefinitionScriptValidation {
        let path_str = path.display().to_string();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut definitions = Vec::new();

        // Read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(DefinitionValidationError {
                    code: "E001",
                    message: format!("Failed to read file: {}", e),
                    line: None,
                });
                return DefinitionScriptValidation {
                    path: path_str,
                    errors,
                    warnings,
                    definitions,
                };
            }
        };

        // Parse script
        let ast = match self.engine.compile(&content) {
            Ok(ast) => ast,
            Err(e) => {
                errors.push(DefinitionValidationError {
                    code: "E002",
                    message: format!("Syntax error: {}", e),
                    line: e.position().line(),
                });
                return DefinitionScriptValidation {
                    path: path_str,
                    errors,
                    warnings,
                    definitions,
                };
            }
        };

        // Infer definition type from path
        let def_type = self.schema_registry.infer_type_from_path(&path_str);
        let schema = def_type.and_then(|t| self.schema_registry.get(t));

        // Check if this is a definition script
        let is_definition_script = path_str.contains("definitions/") || path_str.contains("definitions\\");

        if !is_definition_script {
            return DefinitionScriptValidation {
                path: path_str,
                errors,
                warnings,
                definitions,
            };
        }

        // Find export function (all_ships, all_weapons, etc.)
        let export_fn_names = ["all_ships", "all_weapons", "all_effects", "all_cargo",
                               "all_abilities", "all_factions", "player_ships"];

        let mut has_export_fn = false;
        for fn_def in ast.iter_functions() {
            if export_fn_names.contains(&fn_def.name.to_string().as_str()) {
                has_export_fn = true;
                break;
            }
        }

        if !has_export_fn && def_type.is_some() {
            // base.rhai doesn't need export functions
            if !path_str.contains("base.rhai") {
                warnings.push(DefinitionValidationWarning {
                    code: "W500",
                    message: format!(
                        "Definition script missing export function (e.g., all_{}s())",
                        def_type.unwrap_or("item")
                    ),
                    line: None,
                });
            }
        }

        // Analyze map literals in functions
        if let Some(schema) = schema {
            self.validate_definitions(&ast, &content, schema, &mut errors, &mut warnings, &mut definitions);
        }

        DefinitionScriptValidation {
            path: path_str,
            errors,
            warnings,
            definitions,
        }
    }

    /// Validate definition map literals against schema
    fn validate_definitions(
        &self,
        ast: &AST,
        content: &str,
        schema: &DefinitionSchema,
        errors: &mut Vec<DefinitionValidationError>,
        warnings: &mut Vec<DefinitionValidationWarning>,
        definitions: &mut Vec<DefinitionInfo>,
    ) {
        for fn_def in ast.iter_fn_def() {
            // Skip helper functions like merge, scale_stats
            let fn_name = fn_def.name.to_string();
            let fn_name_str = fn_name.as_str();

            if matches!(fn_name_str, "merge" | "scale_stats" | "railgun" | "missile" | "laser" | "point_defense") {
                continue;
            }
            // Skip export functions
            if fn_name_str.starts_with("all_") || fn_name_str.starts_with("player_") {
                continue;
            }
            // Skip base template functions
            if fn_name_str.ends_with("_base") {
                continue;
            }

            // Find map literals in this function
            let map_keys = self.extract_map_keys_from_function(fn_def, content);

            if !map_keys.is_empty() {
                // Validate each map
                for keys in &map_keys {
                    // Check for required fields
                    for required_field in schema.required_fields() {
                        if !keys.contains(&required_field.to_string()) {
                            errors.push(DefinitionValidationError {
                                code: "E500",
                                message: format!(
                                    "In '{}()': Missing required field '{}' for {}",
                                    fn_name_str, required_field, schema.name
                                ),
                                line: None,
                            });
                        }
                    }

                    // Check for unknown fields
                    for key in keys {
                        if !schema.has_field(key) {
                            warnings.push(DefinitionValidationWarning {
                                code: "W501",
                                message: format!(
                                    "In '{}()': Unknown field '{}' for {}",
                                    fn_name_str, key, schema.name
                                ),
                                line: None,
                            });
                        }
                    }

                    // Extract ID for tracking
                    let id = keys.iter()
                        .find(|k| *k == "id")
                        .map(|_| fn_name.clone())
                        .unwrap_or_else(|| format!("{}_unknown", fn_name_str));

                    definitions.push(DefinitionInfo {
                        id,
                        function: fn_name.clone(),
                        fields: keys.clone(),
                    });
                }
            }
        }
    }

    /// Extract map keys from a function definition using AST walking.
    ///
    /// Map literals can appear as either:
    /// - `Expr::Map` - raw AST form
    /// - `Expr::DynamicConstant` - optimized form (map is pre-evaluated into a Dynamic)
    fn extract_map_keys_from_function(&self, fn_def: &rhai::ScriptFuncDef, _content: &str) -> Vec<Vec<String>> {
        let mut all_keys = Vec::new();

        // Walk all statements to find map literals
        let mut path = Vec::new();
        for stmt in fn_def.body.statements() {
            stmt.walk(&mut path, &mut |nodes| {
                if let Some(ASTNode::Expr(expr)) = nodes.last() {
                    self.extract_map_keys_from_expr(expr, &mut all_keys);
                }
                true
            });
        }

        all_keys
    }

    /// Extract map keys from an expression, handling both Expr::Map and Expr::DynamicConstant.
    fn extract_map_keys_from_expr(&self, expr: &Expr, all_keys: &mut Vec<Vec<String>>) {
        match expr {
            // Raw map literal (before optimization)
            Expr::Map(boxed, _) => {
                let keys: Vec<String> = boxed.0.iter()
                    .map(|(ident, _)| ident.name.to_string())
                    .collect();
                if !keys.is_empty() {
                    all_keys.push(keys);
                }
            }
            // Optimized constant - map has been evaluated into a Dynamic
            Expr::DynamicConstant(boxed_dyn, _) => {
                // Try to extract map keys from the Dynamic value
                if let Some(map) = boxed_dyn.read_lock::<rhai::Map>() {
                    let keys: Vec<String> = map.keys().map(|k| k.to_string()).collect();
                    if !keys.is_empty() {
                        all_keys.push(keys);
                    }
                }
            }
            _ => {}
        }
    }

    /// Validate all definition scripts in a directory
    pub fn validate_directory(&self, dir: &Path) -> DefinitionValidationReport {
        let mut scripts = Vec::new();
        let mut total_errors = 0;
        let mut total_warnings = 0;

        self.validate_dir_recursive(dir, &mut scripts);

        for script in &scripts {
            total_errors += script.errors.len();
            total_warnings += script.warnings.len();
        }

        DefinitionValidationReport {
            scripts,
            total_errors,
            total_warnings,
        }
    }

    fn validate_dir_recursive(&self, dir: &Path, scripts: &mut Vec<DefinitionScriptValidation>) {
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

impl Default for DefinitionScriptValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptValidator for DefinitionScriptValidator {
    type Validation = DefinitionScriptValidation;
    type Report = DefinitionValidationReport;

    fn validate_file(&self, path: &Path) -> Self::Validation {
        DefinitionScriptValidator::validate_file(self, path)
    }

    fn validate_directory(&self, dir: &Path) -> Self::Report {
        DefinitionScriptValidator::validate_directory(self, dir)
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
                if let Some(expected_params) = spec.param_count
                    && f.params.len() != expected_params {
                        result.wrong_params.push((spec.clone(), f.params.len()));
                    }
            }
        }
    }

    // Check optional functions for parameter mismatches (warnings only)
    for spec in &contract.optional {
        if let Some(f) = functions.iter().find(|f| f.name == spec.name)
            && let Some(expected_params) = spec.param_count
                && f.params.len() != expected_params {
                    result.warnings.push(format!(
                        "Optional function '{}' has {} parameters, expected {} ({})",
                        spec.name, f.params.len(), expected_params, spec.description
                    ));
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
