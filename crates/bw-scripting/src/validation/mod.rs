//! Script validation
//!
//! Validates scripts have required functions before runtime execution.
//! Also provides comprehensive action script validation to catch errors early.
//!
//! # Validation Layers
//!
//! 1. **Syntax** - Rhai parser checks
//! 2. **Structure** - Required functions (init, handlers)
//! 3. **API Calls** - Function argument count validation
//! 4. **Variable Flow** - Undefined vars, unused vars, shadowing
//! 5. **Null Safety** - Detecting use of potentially null values
//! 6. **Return Paths** - Ensuring handlers return on all paths
//!
//! # Module Organization
//!
//! - `api_functions` - API function signatures and return types
//! - `types` - Core types for type inference, scopes, and analysis results
//! - `schema` - Object schemas for nested access validation
//! - `analysis` - AST walking and function analysis
//! - `validators` - Action and definition script validators
//! - `dependency` - Cross-script dependency analysis
//! - `traits` - Shared traits for validators
//! - `error_codes` - Error code taxonomy and categorization

mod api_functions;
mod analysis;
mod builtin_functions;
mod dependency;
mod error_codes;
mod schema;
mod traits;
mod types;
mod validators;

// Re-export public API

// From api_functions
pub use api_functions::{
    ApiFn, ReturnType, API_FUNCTIONS, CTX_KEYS, KNOWN_EVENT_TYPES,
    EventSubscriptionInfo, VarInfo,
    get_api_function, is_known_event_type,
};

// From types
pub use types::{
    InferredType, BinaryOpRule, BINARY_OP_RULES, BINARY_OPERATORS,
    IndexRule, INDEX_RULES,
    NullGuardInfo, VarState, Scope, ReturnMapField, FunctionAnalysis,
    is_binary_operator, check_binary_op, is_implicit_coercion,
    check_index_op, is_indexable, check_boolean_operand, check_logical_not,
};

// From builtin_functions
pub use builtin_functions::{
    BuiltinFn, BUILTIN_FUNCTIONS,
    get_builtin_function, get_builtin_method, get_builtin,
};

// From schema
pub use schema::{
    NestedFieldType, NestedFieldSchema, ObjectSchema, AccessSegment, AccessPath,
    SHIP_SCHEMA, PLAYER_SCHEMA, CARGO_ITEM_SCHEMA, INSTALLED_UPGRADE_SCHEMA,
    POSITION_SCHEMA, WEAPON_SCHEMA, LOCATION_SCHEMA, SECTOR_SCHEMA,
    ADJACENT_SECTOR_SCHEMA, COMBAT_ENGAGEMENT_SCHEMA, SQUADRON_SCHEMA,
    SQUADRON_INVITE_SCHEMA, MISSION_SCHEMA,
    get_object_schema, api_return_schema,
};

// From analysis
pub use analysis::{
    AstAnalysis, ArchetypeRegistry,
    ARCHETYPE_LOOKUP_FNS,
    analyze_ast, analyze_function, extract_string_literal,
    is_archetype_lookup_fn, archetype_category, is_builtin_var, infer_dynamic_type,
};

// From validators
pub use validators::{
    // Action validation
    ActionScriptValidation, RegisteredAction, ActionValidationError, ActionValidationWarning,
    ActionValidationReport, ActionScriptValidator,
    // Definition validation
    DefinitionScriptValidation, DefinitionInfo, DefinitionValidationError, DefinitionValidationWarning,
    DefinitionValidationReport, DefinitionScriptValidator,
    // Contract validation
    ScriptContract, FunctionSpec, ValidationResult,
    contracts, validate_script, infer_contract,
};

// From dependency
pub use dependency::{
    ScriptActionInfo, ScriptDependencyGraph,
};

// From traits
pub use traits::{
    ValidationError, ValidationWarning, ScriptValidation, ValidationReport,
    ScriptValidator, collect_rhai_files,
};

// From error_codes
pub use error_codes::{ErrorCategory, is_error, is_warning};

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Engine;

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

    // ========================================================================
    // API Function Signature Tests
    // ========================================================================

    #[test]
    fn test_api_function_lookup() {
        // Test that API functions are properly registered
        assert!(get_api_function("query_ship").is_some());
        assert!(get_api_function("send_notification").is_some());
        assert!(get_api_function("nonexistent_function").is_none());
    }

    #[test]
    fn test_api_function_arg_counts() {
        // Verify some key functions have correct arg counts
        assert_eq!(get_api_function("query_ship").unwrap().arg_count, 1);
        assert_eq!(get_api_function("modify_ship").unwrap().arg_count, 2);
        assert_eq!(get_api_function("send_notification").unwrap().arg_count, 2);
        assert_eq!(get_api_function("spawn_npc").unwrap().arg_count, 1);
    }

    #[test]
    fn test_api_function_return_types() {
        assert_eq!(get_api_function("query_ship").unwrap().returns, ReturnType::Map);
        assert_eq!(get_api_function("query_sector").unwrap().returns, ReturnType::Nullable);
        assert_eq!(get_api_function("modify_ship").unwrap().returns, ReturnType::Unit);
        assert_eq!(get_api_function("send_notification").unwrap().returns, ReturnType::Bool);
    }

    // ========================================================================
    // Binary Operation Type Checking Tests
    // ========================================================================

    #[test]
    fn test_binary_op_rules() {
        // Int + Int = Int
        assert_eq!(
            check_binary_op("+", &InferredType::Int, &InferredType::Int).unwrap(),
            InferredType::Int
        );

        // Float + Float = Float
        assert_eq!(
            check_binary_op("+", &InferredType::Float, &InferredType::Float).unwrap(),
            InferredType::Float
        );

        // Int + Float = Float (coercion)
        assert_eq!(
            check_binary_op("+", &InferredType::Int, &InferredType::Float).unwrap(),
            InferredType::Float
        );

        // String + Int = String
        assert_eq!(
            check_binary_op("+", &InferredType::String, &InferredType::Int).unwrap(),
            InferredType::String
        );

        // Comparison returns Bool
        assert_eq!(
            check_binary_op("<", &InferredType::Int, &InferredType::Int).unwrap(),
            InferredType::Bool
        );
    }

    #[test]
    fn test_binary_op_type_errors() {
        // Array + Int should fail
        assert!(check_binary_op("+", &InferredType::Array, &InferredType::Int).is_err());

        // Map - Bool should fail
        assert!(check_binary_op("-", &InferredType::Map, &InferredType::Bool).is_err());
    }

    #[test]
    fn test_implicit_coercion_detection() {
        assert!(is_implicit_coercion(&InferredType::Int, &InferredType::Float));
        assert!(is_implicit_coercion(&InferredType::Float, &InferredType::Int));
        assert!(!is_implicit_coercion(&InferredType::Int, &InferredType::Int));
        assert!(!is_implicit_coercion(&InferredType::String, &InferredType::Int));
    }

    // ========================================================================
    // Schema Validation Tests
    // ========================================================================

    #[test]
    fn test_ship_schema_fields() {
        let schema = get_object_schema("Ship").unwrap();
        assert!(schema.get_field("id").is_some());
        assert!(schema.get_field("name").is_some());
        assert!(schema.get_field("hull").is_some());
        assert!(schema.get_field("cargo").is_some());
        assert!(schema.get_field("nonexistent").is_none());
    }

    #[test]
    fn test_player_schema_fields() {
        let schema = get_object_schema("Player").unwrap();
        assert!(schema.get_field("id").is_some());
        assert!(schema.get_field("username").is_some());
        assert!(schema.get_field("credits").is_some());
        assert!(schema.get_field("reputation").is_some());
    }

    #[test]
    fn test_api_return_schema() {
        assert_eq!(api_return_schema("query_ship"), Some("Ship"));
        assert_eq!(api_return_schema("query_player"), Some("Player"));
        assert_eq!(api_return_schema("query_sector"), Some("Sector"));
        assert_eq!(api_return_schema("unknown_function"), None);
    }

    // ========================================================================
    // Event Type Validation Tests
    // ========================================================================

    #[test]
    fn test_known_event_types() {
        assert!(is_known_event_type("ShipDestroyed"));
        assert!(is_known_event_type("MissionCompleted"));
        assert!(is_known_event_type("CombatStarted"));
        assert!(!is_known_event_type("NonexistentEvent"));
    }

    // ========================================================================
    // AST Analysis Tests
    // ========================================================================

    #[test]
    fn test_analyze_ast_function_calls() {
        let engine = Engine::new();
        let ast = engine.compile(r#"
            fn test() {
                let x = query_ship("ship1");
                send_notification("player1", "Hello");
            }
        "#).unwrap();

        let analysis = analyze_ast(&ast, "");

        assert!(analysis.fn_calls.iter().any(|(name, _, _)| name == "query_ship"));
        assert!(analysis.fn_calls.iter().any(|(name, _, _)| name == "send_notification"));
    }

    #[test]
    fn test_analyze_ast_variable_tracking() {
        let engine = Engine::new();
        let ast = engine.compile(r#"
            fn test() {
                let x = 1;
                let y = x + 2;
            }
        "#).unwrap();

        let analysis = analyze_ast(&ast, "");

        // Should have variable definitions
        assert!(analysis.var_defs.iter().any(|(name, _, _)| name == "x"));
        assert!(analysis.var_defs.iter().any(|(name, _, _)| name == "y"));

        // Should track variable uses
        assert!(analysis.var_uses.iter().any(|(name, _)| name == "x"));
    }

    // ========================================================================
    // Archetype Registry Tests
    // ========================================================================

    #[test]
    fn test_archetype_registry() {
        let mut registry = ArchetypeRegistry::new();
        registry.insert("ships", "fighter".to_string());
        registry.insert("weapons", "laser".to_string());

        assert!(registry.contains("ships", "fighter"));
        assert!(registry.contains("weapons", "laser"));
        assert!(!registry.contains("ships", "nonexistent"));
        assert!(!registry.contains("invalid_category", "anything"));
    }

    #[test]
    fn test_archetype_category_mapping() {
        assert_eq!(archetype_category("get_ship_def"), Some("ships"));
        assert_eq!(archetype_category("get_weapon"), Some("weapons"));
        assert_eq!(archetype_category("get_faction"), Some("factions"));
        assert_eq!(archetype_category("unknown_fn"), None);
    }

    // ========================================================================
    // ScriptValidator Trait Tests
    // ========================================================================

    #[test]
    fn test_validator_trait_action() {
        // Test that ActionScriptValidator implements ScriptValidator
        fn assert_validator<V: ScriptValidator>(_v: &V) {}

        let validator = ActionScriptValidator::new();
        assert_validator(&validator);

        // Validate through trait interface
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("trait_test.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}
fn handle_test(ctx, params) {
    #{ success: true }
}
"#).unwrap();

        let result: ActionScriptValidation = <ActionScriptValidator as ScriptValidator>::validate_file(&validator, &test_file);
        std::fs::remove_file(&test_file).ok();

        assert!(ScriptValidation::is_valid(&result));
        assert!(ScriptValidation::errors(&result).is_empty());
        assert!(!ScriptValidation::path(&result).is_empty());
    }

    #[test]
    fn test_validator_trait_definition() {
        // Test that DefinitionScriptValidator implements ScriptValidator
        fn assert_validator<V: ScriptValidator>(_v: &V) {}

        let validator = DefinitionScriptValidator::new();
        assert_validator(&validator);
    }

    #[test]
    fn test_validation_error_trait() {
        let error = ActionValidationError {
            code: "E001",
            message: "Test error".to_string(),
            line: Some(10),
        };

        assert_eq!(ValidationError::code(&error), "E001");
        assert_eq!(ValidationError::message(&error), "Test error");
        assert_eq!(ValidationError::line(&error), Some(10));
    }

    #[test]
    fn test_validation_warning_trait() {
        let warning = ActionValidationWarning {
            code: "W001",
            message: "Test warning".to_string(),
            line: None,
        };

        assert_eq!(ValidationWarning::code(&warning), "W001");
        assert_eq!(ValidationWarning::message(&warning), "Test warning");
        assert_eq!(ValidationWarning::line(&warning), None);
    }

    #[test]
    fn test_collect_rhai_files() {
        let temp_dir = std::env::temp_dir().join("validation_test");
        let sub_dir = temp_dir.join("subdir");
        std::fs::create_dir_all(&sub_dir).ok();
        std::fs::write(temp_dir.join("test1.rhai"), "// test").ok();
        std::fs::write(temp_dir.join("test2.txt"), "// not rhai").ok();
        std::fs::write(sub_dir.join("test3.rhai"), "// nested").ok();

        let files = collect_rhai_files(&temp_dir);

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();

        // Should find both .rhai files but not .txt
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("test1.rhai")));
        assert!(files.iter().any(|p| p.ends_with("test3.rhai")));
    }
}
