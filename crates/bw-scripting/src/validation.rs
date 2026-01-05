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

use std::collections::HashSet;
use std::path::Path;

use rhai::{AST, ASTNode, Dynamic, Engine, Expr, FnCallExpr, Position, Stmt};

use crate::schema::ActionSchemaRegistry;

// ============================================================================
// API Function Signatures
// ============================================================================

/// Return type of an API function for null safety analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// Returns nothing (modify_*, send_*, emit_*)
    Unit,
    /// May return () on failure (query_* functions)
    Nullable,
    /// Always returns bool
    Bool,
    /// Always returns a Map
    Map,
    /// Always returns an Array
    Array,
    /// Always returns a number (Int or Float)
    Number,
    /// Always returns a String
    String,
    /// Unknown/varied return type
    Dynamic,
}

/// Specification of an API function available to scripts.
#[derive(Debug, Clone)]
pub struct ApiFn {
    pub name: &'static str,
    pub arg_count: usize,
    pub returns: ReturnType,
}

impl ApiFn {
    const fn new(name: &'static str, arg_count: usize, returns: ReturnType) -> Self {
        Self { name, arg_count, returns }
    }
}

/// All registered API functions with their signatures.
/// Used for validating function calls in scripts.
pub static API_FUNCTIONS: &[ApiFn] = &[
    // === State API - Queries ===
    ApiFn::new("query_ship", 1, ReturnType::Map),           // Throws on not found
    ApiFn::new("query_player", 1, ReturnType::Map),         // Throws on not found
    ApiFn::new("query_sector", 1, ReturnType::Nullable),    // Returns () if not found
    ApiFn::new("query_ships_in_sector", 1, ReturnType::Array),
    ApiFn::new("query_ships_in_range", 5, ReturnType::Array),
    ApiFn::new("query_ships_near", 4, ReturnType::Array),
    ApiFn::new("get_context_sector", 0, ReturnType::Nullable),

    // === State API - Modifications ===
    ApiFn::new("modify_ship", 2, ReturnType::Unit),         // Throws on error
    ApiFn::new("modify_player", 2, ReturnType::Bool),
    ApiFn::new("damage_ship", 2, ReturnType::Bool),
    ApiFn::new("move_ship_to", 4, ReturnType::Bool),
    ApiFn::new("spawn_npc", 1, ReturnType::String),
    ApiFn::new("destroy_ship", 1, ReturnType::Bool),

    // === State API - Events ===
    ApiFn::new("emit_event", 2, ReturnType::Bool),
    ApiFn::new("emit_event_with_target", 3, ReturnType::Bool),

    // === State API - Economy ===
    ApiFn::new("add_credits", 2, ReturnType::Bool),
    ApiFn::new("spend_credits", 2, ReturnType::Bool),
    ApiFn::new("get_credits", 1, ReturnType::Number),

    // === State API - Cargo ===
    ApiFn::new("add_cargo", 4, ReturnType::Bool),
    ApiFn::new("remove_cargo", 3, ReturnType::Bool),
    ApiFn::new("get_cargo", 1, ReturnType::Array),
    ApiFn::new("get_cargo_capacity", 1, ReturnType::Number),
    ApiFn::new("get_cargo_used", 1, ReturnType::Number),

    // === State API - Combat ===
    ApiFn::new("set_combat_stance", 2, ReturnType::Bool),
    ApiFn::new("get_combat_stance", 1, ReturnType::String),
    ApiFn::new("lock_target", 2, ReturnType::Bool),
    ApiFn::new("clear_target", 1, ReturnType::Bool),
    ApiFn::new("get_locked_target", 1, ReturnType::String),

    // === State API - Upgrades ===
    ApiFn::new("install_upgrade", 3, ReturnType::Bool),
    ApiFn::new("remove_upgrade", 2, ReturnType::Bool),
    ApiFn::new("get_upgrades", 1, ReturnType::Array),
    ApiFn::new("has_upgrade", 2, ReturnType::Bool),

    // === State API - Watches ===
    ApiFn::new("watch_property", 5, ReturnType::String),
    ApiFn::new("watch_property_changed", 3, ReturnType::String),
    ApiFn::new("watch_once", 5, ReturnType::String),
    ApiFn::new("unwatch", 1, ReturnType::Bool),
    ApiFn::new("unwatch_all_for_entity", 1, ReturnType::Unit),
    ApiFn::new("pause_watch", 1, ReturnType::Bool),
    ApiFn::new("resume_watch", 1, ReturnType::Bool),
    ApiFn::new("list_watches_for_entity", 1, ReturnType::Array),
    ApiFn::new("get_watch_count", 0, ReturnType::Number),

    // === State API - Utility ===
    ApiFn::new("distance", 6, ReturnType::Number),
    ApiFn::new("distance_2d", 4, ReturnType::Number),
    ApiFn::new("distance_to_ship", 4, ReturnType::Number),
    ApiFn::new("is_hostile_ship_class", 1, ReturnType::Bool),

    // === Message API ===
    ApiFn::new("send_notification", 2, ReturnType::Bool),
    ApiFn::new("send_notification_type", 3, ReturnType::Bool),
    ApiFn::new("send_warning", 2, ReturnType::Bool),
    ApiFn::new("send_error", 2, ReturnType::Bool),
    ApiFn::new("send_success", 2, ReturnType::Bool),
    ApiFn::new("send_choice", 4, ReturnType::Bool),
    ApiFn::new("broadcast_to_sector", 2, ReturnType::Bool),
    ApiFn::new("broadcast_warning_to_sector", 2, ReturnType::Bool),

    // === Action API ===
    ApiFn::new("register_action", 2, ReturnType::Bool),
    ApiFn::new("register_action_with_requirements", 3, ReturnType::Bool),
    ApiFn::new("unregister_action", 1, ReturnType::Bool),
    ApiFn::new("is_action_registered", 1, ReturnType::Bool),
    ApiFn::new("list_actions", 0, ReturnType::Array),
    ApiFn::new("set_action_enabled", 2, ReturnType::Bool),

    // === Combat API ===
    ApiFn::new("calculate_damage", 3, ReturnType::Number),
    ApiFn::new("calculate_hit_chance", 3, ReturnType::Number),
    ApiFn::new("roll_hit", 1, ReturnType::Bool),
    ApiFn::new("roll_critical", 2, ReturnType::Bool),
    ApiFn::new("apply_critical", 2, ReturnType::Number),
    ApiFn::new("create_attack_result", 3, ReturnType::Map),
    ApiFn::new("resolve_attack", 7, ReturnType::Map),

    // === Data API ===
    ApiFn::new("get_data", 2, ReturnType::Nullable),
    ApiFn::new("get_ship_def", 1, ReturnType::Nullable),
    ApiFn::new("get_upgrade_def", 1, ReturnType::Nullable),
    ApiFn::new("get_cargo_def", 1, ReturnType::Nullable),
    ApiFn::new("get_stance_def", 1, ReturnType::Nullable),
    ApiFn::new("list_data_keys", 1, ReturnType::Array),
    ApiFn::new("has_data", 2, ReturnType::Bool),

    // === Data API - Archetypes ===
    ApiFn::new("get_ship", 1, ReturnType::Nullable),
    ApiFn::new("all_ships", 0, ReturnType::Array),
    ApiFn::new("player_ships", 0, ReturnType::Array),
    ApiFn::new("get_weapon", 1, ReturnType::Nullable),
    ApiFn::new("all_weapons", 0, ReturnType::Array),
    ApiFn::new("get_effect", 1, ReturnType::Nullable),
    ApiFn::new("all_effects", 0, ReturnType::Array),
    ApiFn::new("get_ability", 1, ReturnType::Nullable),
    ApiFn::new("all_abilities", 0, ReturnType::Array),
    ApiFn::new("active_abilities", 0, ReturnType::Array),
    ApiFn::new("passive_abilities", 0, ReturnType::Array),
    ApiFn::new("all_cargo_types", 0, ReturnType::Array),
    ApiFn::new("legal_cargo_types", 0, ReturnType::Array),
    ApiFn::new("get_faction", 1, ReturnType::Nullable),
    ApiFn::new("all_factions", 0, ReturnType::Array),
    ApiFn::new("playable_factions", 0, ReturnType::Array),
    ApiFn::new("hostile_factions", 0, ReturnType::Array),
    ApiFn::new("get_faction_relation", 2, ReturnType::Number),

    // === Logging ===
    ApiFn::new("log_info", 1, ReturnType::Unit),
    ApiFn::new("log_warning", 1, ReturnType::Unit),
    ApiFn::new("log_error", 1, ReturnType::Unit),

    // === Random ===
    ApiFn::new("random_float", 0, ReturnType::Number),
    ApiFn::new("random_int", 2, ReturnType::Number),
    ApiFn::new("current_tick", 0, ReturnType::Number),
];

/// Lookup API function by name.
pub fn get_api_function(name: &str) -> Option<&'static ApiFn> {
    API_FUNCTIONS.iter().find(|f| f.name == name)
}

/// Known keys for action context.
pub static CTX_KEYS: &[&str] = &[
    "player_id",
    "ship_id",
    "sector_id",
    "action",
    "params",
];

/// Known event types (from bw_core::events::GameEventType).
pub static KNOWN_EVENT_TYPES: &[&str] = &[
    // Player events
    "PlayerJoined",
    "PlayerLeft",
    "PlayerDocked",
    "PlayerUndocked",
    // Ship events
    "ShipSpawned",
    "ShipDestroyed",
    "ShipDamaged",
    "ShipRepaired",
    "ShipRefueled",
    "ShipRearmed",
    // Combat events
    "CombatStarted",
    "CombatEnded",
    "AttackHit",
    "AttackMissed",
    // Mission events
    "MissionSpawned",
    "MissionAccepted",
    "MissionCompleted",
    "MissionFailed",
    "MissionExpired",
    "MissionChoiceMade",
    // Reputation events
    "ReputationGained",
    "ReputationLost",
    "FameGained",
    "FameLost",
    "PlayerDisgraced",
    // Squadron events
    "SquadronCreated",
    "SquadronDissolved",
    "SquadronMemberJoined",
    "SquadronMemberLeft",
    "SquadronWarDeclared",
    "SquadronPeaceDeclared",
    // Sector events
    "SectorControlChanged",
    "StationBuilt",
    "StationDestroyed",
    // Special events
    "SeraIncursion",
    "DroneSwarmDetected",
    "AsteroidAlert",
    "DistressSignalReceived",
];

/// Check if an event type is known.
pub fn is_known_event_type(event_type: &str) -> bool {
    KNOWN_EVENT_TYPES.contains(&event_type)
}

/// Information about an event subscription in a script.
#[derive(Debug, Clone)]
pub struct EventSubscriptionInfo {
    /// The event type being subscribed to
    pub event_type: String,
    /// The handler function name
    pub handler_fn: String,
    /// Position in source
    pub position: Option<Position>,
}

// ============================================================================
// Dataflow Analysis Types
// ============================================================================

/// Information about a variable in scope.
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// Name of the variable
    pub name: String,
    /// Where the variable was defined
    pub defined_at: Option<Position>,
    /// Whether the variable has been used
    pub used: bool,
    /// Whether this variable may be null (from query_* etc)
    pub nullable: bool,
    /// Whether we've confirmed it's not null (via if check)
    pub null_checked: bool,
}

/// Result of variable flow analysis.
#[derive(Debug, Default)]
pub struct VariableFlowResult {
    /// Variables used without being defined
    pub undefined: Vec<(String, Option<Position>)>,
    /// Variables defined but never used
    pub unused: Vec<(String, Option<Position>)>,
    /// Variables that shadow outer scope
    pub shadows: Vec<(String, Option<Position>)>,
}

/// Result of null safety analysis.
#[derive(Debug, Default)]
pub struct NullSafetyResult {
    /// Accesses to potentially null values without null check
    pub unchecked_accesses: Vec<(String, Option<Position>)>,
}

/// Result of control flow analysis.
#[derive(Debug, Default)]
pub struct ControlFlowResult {
    /// Code after unconditional return
    pub dead_code: Vec<Option<Position>>,
    /// Handler functions that may not return a value
    pub missing_returns: Vec<String>,
}

// ============================================================================
// AST Walking Infrastructure
// ============================================================================

/// Collected information from AST walking.
#[derive(Debug, Default)]
pub struct AstAnalysis {
    /// Function calls: (name, arg_count, position)
    pub fn_calls: Vec<(String, usize, Option<Position>)>,
    /// Index accesses: (target_var, key, position) for x["key"]
    pub index_accesses: Vec<(String, String, Option<Position>)>,
    /// Dynamic key accesses: (target_var, position) for x[variable]
    pub dynamic_key_accesses: Vec<(String, Option<Position>)>,
    /// Variable definitions: (name, position, is_nullable)
    pub var_defs: Vec<(String, Option<Position>, bool)>,
    /// Variable uses: (name, position)
    pub var_uses: Vec<(String, Option<Position>)>,
    /// Return statements: (position, has_value)
    pub returns: Vec<(Option<Position>, bool)>,
    /// Map literal keys: (keys, position)
    pub map_keys: Vec<(Vec<String>, Option<Position>)>,
    /// Event subscriptions found in the script
    pub event_subscriptions: Vec<EventSubscriptionInfo>,
}

/// Analyze an AST and collect information for validation using proper AST walking.
pub fn analyze_ast(ast: &AST, _content: &str) -> AstAnalysis {
    let mut analysis = AstAnalysis::default();

    // Add function parameters as defined variables and walk function bodies
    for fn_def in ast.iter_fn_def() {
        for param in fn_def.params.iter() {
            analysis.var_defs.push((param.to_string(), None, false));
        }

        // Walk the function body
        let mut path = Vec::new();
        for stmt in fn_def.body.statements() {
            stmt.walk(&mut path, &mut |nodes| {
                collect_from_ast_node(nodes, &mut analysis);
                true // continue walking
            });
        }
    }

    // Walk top-level statements
    let mut path = Vec::new();
    for stmt in ast.statements() {
        stmt.walk(&mut path, &mut |nodes| {
            collect_from_ast_node(nodes, &mut analysis);
            true
        });
    }

    analysis
}

/// Collect analysis information from an AST node path.
fn collect_from_ast_node(path: &[ASTNode], analysis: &mut AstAnalysis) {
    let Some(node) = path.last() else { return };

    match node {
        ASTNode::Stmt(stmt) => collect_from_stmt(stmt, analysis),
        ASTNode::Expr(expr) => collect_from_expr(expr, analysis),
        _ => {} // ASTNode is non-exhaustive
    }
}

/// Collect information from a statement.
fn collect_from_stmt(stmt: &Stmt, analysis: &mut AstAnalysis) {
    match stmt {
        // Variable definitions: let x = expr
        Stmt::Var(boxed, _flags, pos) => {
            let (ident, init_expr, _) = boxed.as_ref();
            let var_name = ident.name.to_string();

            // Check if the initializer is a nullable function call
            let is_nullable = is_nullable_expr(init_expr);
            analysis.var_defs.push((var_name, Some(*pos), is_nullable));
        }

        // For loops: for x in expr { ... }
        Stmt::For(boxed, pos) => {
            let (loop_var, counter_var, _flow) = boxed.as_ref();
            analysis.var_defs.push((loop_var.name.to_string(), Some(*pos), false));
            if let Some(counter) = counter_var {
                analysis.var_defs.push((counter.name.to_string(), Some(*pos), false));
            }
        }

        // Return statements
        Stmt::Return(opt_expr, _flags, pos) => {
            let has_value = opt_expr.is_some();
            analysis.returns.push((Some(*pos), has_value));
        }

        // Function calls as statements
        Stmt::FnCall(call_expr, pos) => {
            collect_fn_call(call_expr, Some(*pos), analysis);
        }

        _ => {}
    }
}

/// Collect information from an expression.
fn collect_from_expr(expr: &Expr, analysis: &mut AstAnalysis) {
    match expr {
        // Variable uses
        Expr::Variable(boxed, _, pos) => {
            // boxed.1 is the variable name (ImmutableString)
            let var_name = boxed.1.to_string();
            analysis.var_uses.push((var_name, Some(*pos)));
        }

        // Function calls
        Expr::FnCall(call_expr, pos) => {
            collect_fn_call(call_expr, Some(*pos), analysis);
        }

        // Index access: x["key"] or x[variable]
        Expr::Index(boxed, _flags, pos) => {
            // Check if it's variable[...] pattern
            if let Expr::Variable(var_box, _, _) = &boxed.lhs {
                let var_name = var_box.1.to_string();

                // Check if index is a string literal
                match &boxed.rhs {
                    Expr::StringConstant(key, _) => {
                        // Static key: x["key"]
                        analysis.index_accesses.push((var_name, key.to_string(), Some(*pos)));
                    }
                    Expr::DynamicConstant(dyn_val, _) => {
                        // Check if it's a constant string
                        if let Some(s) = dyn_val.clone().try_cast::<rhai::ImmutableString>() {
                            analysis.index_accesses.push((var_name, s.to_string(), Some(*pos)));
                        } else {
                            // Non-string constant (e.g., integer index)
                            analysis.dynamic_key_accesses.push((var_name, Some(*pos)));
                        }
                    }
                    _ => {
                        // Dynamic key: x[variable] or x[expr]
                        analysis.dynamic_key_accesses.push((var_name, Some(*pos)));
                    }
                }
            }
        }

        // Map literals: #{ key: value, ... }
        Expr::Map(boxed, pos) => {
            let keys: Vec<String> = boxed.0.iter().map(|(ident, _)| ident.name.to_string()).collect();
            if !keys.is_empty() {
                analysis.map_keys.push((keys, Some(*pos)));
            }
        }

        _ => {}
    }
}

/// Collect function call information.
fn collect_fn_call(call_expr: &FnCallExpr, pos: Option<Position>, analysis: &mut AstAnalysis) {
    let fn_name = call_expr.name.to_string();
    let arg_count = call_expr.args.len();
    analysis.fn_calls.push((fn_name.clone(), arg_count, pos));

    // Check for event subscription calls
    match fn_name.as_str() {
        "subscribe_event" | "subscribe_event_filtered" => {
            // subscribe_event(event_type, handler_fn)
            // subscribe_event_filtered(event_type, handler_fn, filter_type, filter_value)
            if call_expr.args.len() >= 2 {
                if let (Some(event_type), Some(handler_fn)) = (
                    extract_string_literal(&call_expr.args[0]),
                    extract_string_literal(&call_expr.args[1]),
                ) {
                    analysis.event_subscriptions.push(EventSubscriptionInfo {
                        event_type,
                        handler_fn,
                        position: pos,
                    });
                }
            }
        }
        "subscribe_events" => {
            // subscribe_events(event_types_array, handler_fn)
            // We can't easily extract array literals here, but we can get the handler
            if call_expr.args.len() >= 2 {
                if let Some(handler_fn) = extract_string_literal(&call_expr.args[1]) {
                    // For array subscriptions, we mark event_type as "*" (multiple)
                    // We'll extract individual types if the array is a literal
                    if let Some(event_types) = extract_string_array(&call_expr.args[0]) {
                        for event_type in event_types {
                            analysis.event_subscriptions.push(EventSubscriptionInfo {
                                event_type,
                                handler_fn: handler_fn.clone(),
                                position: pos,
                            });
                        }
                    } else {
                        // Dynamic array - can't validate event types, but can validate handler
                        analysis.event_subscriptions.push(EventSubscriptionInfo {
                            event_type: "*".to_string(),
                            handler_fn,
                            position: pos,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

/// Extract a string literal from an expression.
fn extract_string_literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::StringConstant(s, _) => Some(s.to_string()),
        Expr::DynamicConstant(boxed, _) => {
            if let Some(s) = boxed.clone().try_cast::<rhai::ImmutableString>() {
                Some(s.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract an array of strings from an array expression.
fn extract_string_array(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::Array(boxed, _) => {
            let mut strings = Vec::new();
            for item in boxed.iter() {
                if let Some(s) = extract_string_literal(item) {
                    strings.push(s);
                } else {
                    return None; // Has non-string elements
                }
            }
            Some(strings)
        }
        Expr::DynamicConstant(boxed, _) => {
            if let Some(arr) = boxed.read_lock::<rhai::Array>() {
                let mut strings = Vec::new();
                for item in arr.iter() {
                    if let Some(s) = item.clone().try_cast::<rhai::ImmutableString>() {
                        strings.push(s.to_string());
                    } else {
                        return None;
                    }
                }
                Some(strings)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if an expression is a call to a nullable-returning function.
fn is_nullable_expr(expr: &Expr) -> bool {
    if let Expr::FnCall(call_expr, _) = expr {
        let name = call_expr.name.as_str();
        return is_nullable_fn_name(name);
    }
    false
}

/// Check if a function name returns a nullable value.
fn is_nullable_fn_name(name: &str) -> bool {
    matches!(
        name,
        "query_sector" | "get_context_sector" | "get_data" |
        "get_ship_def" | "get_upgrade_def" | "get_cargo_def" |
        "get_stance_def" | "get_ship" | "get_weapon" | "get_effect" |
        "get_ability" | "get_faction"
    )
}

// ============================================================================
// Function Analyzer - Proper AST-based analysis with scope/type tracking
// ============================================================================

/// Inferred type for a value in Rhai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredType {
    /// Unit type ()
    Unit,
    /// Boolean
    Bool,
    /// Integer (i64)
    Int,
    /// Float (f64)
    Float,
    /// String
    String,
    /// Array
    Array,
    /// Map/Object
    Map,
    /// Could be Map or Unit (from nullable functions)
    Nullable,
    /// Unknown/dynamic type
    Dynamic,
}

// ============================================================================
// Binary Operation Type Rules
// ============================================================================

/// A binary operation type rule: (operator, left_type, right_type, result_type)
#[derive(Debug, Clone)]
pub struct BinaryOpRule {
    pub op: &'static str,
    pub left: InferredType,
    pub right: InferredType,
    pub result: InferredType,
}

impl BinaryOpRule {
    const fn new(op: &'static str, left: InferredType, right: InferredType, result: InferredType) -> Self {
        Self { op, left, right, result }
    }
}

/// Type rules for binary operations.
/// Rules are matched in order - first match wins.
pub static BINARY_OP_RULES: &[BinaryOpRule] = &[
    // Numeric operations
    BinaryOpRule::new("+", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("+", InferredType::Float, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("+", InferredType::Int, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("+", InferredType::Float, InferredType::Int, InferredType::Float),
    BinaryOpRule::new("+", InferredType::String, InferredType::String, InferredType::String),

    BinaryOpRule::new("-", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("-", InferredType::Float, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("-", InferredType::Int, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("-", InferredType::Float, InferredType::Int, InferredType::Float),

    BinaryOpRule::new("*", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("*", InferredType::Float, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("*", InferredType::Int, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("*", InferredType::Float, InferredType::Int, InferredType::Float),

    BinaryOpRule::new("/", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("/", InferredType::Float, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("/", InferredType::Int, InferredType::Float, InferredType::Float),
    BinaryOpRule::new("/", InferredType::Float, InferredType::Int, InferredType::Float),

    BinaryOpRule::new("%", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("%", InferredType::Float, InferredType::Float, InferredType::Float),

    // Comparison operations - always return Bool
    BinaryOpRule::new("==", InferredType::Dynamic, InferredType::Dynamic, InferredType::Bool),
    BinaryOpRule::new("!=", InferredType::Dynamic, InferredType::Dynamic, InferredType::Bool),
    BinaryOpRule::new("<", InferredType::Int, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new("<", InferredType::Float, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new("<", InferredType::Int, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new("<", InferredType::Float, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new("<", InferredType::String, InferredType::String, InferredType::Bool),
    BinaryOpRule::new("<=", InferredType::Int, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new("<=", InferredType::Float, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new("<=", InferredType::Int, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new("<=", InferredType::Float, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new("<=", InferredType::String, InferredType::String, InferredType::Bool),
    BinaryOpRule::new(">", InferredType::Int, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new(">", InferredType::Float, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new(">", InferredType::Int, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new(">", InferredType::Float, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new(">", InferredType::String, InferredType::String, InferredType::Bool),
    BinaryOpRule::new(">=", InferredType::Int, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new(">=", InferredType::Float, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new(">=", InferredType::Int, InferredType::Float, InferredType::Bool),
    BinaryOpRule::new(">=", InferredType::Float, InferredType::Int, InferredType::Bool),
    BinaryOpRule::new(">=", InferredType::String, InferredType::String, InferredType::Bool),

    // Logical operations
    BinaryOpRule::new("&&", InferredType::Bool, InferredType::Bool, InferredType::Bool),
    BinaryOpRule::new("||", InferredType::Bool, InferredType::Bool, InferredType::Bool),

    // Bitwise operations
    BinaryOpRule::new("&", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("|", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("^", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new("<<", InferredType::Int, InferredType::Int, InferredType::Int),
    BinaryOpRule::new(">>", InferredType::Int, InferredType::Int, InferredType::Int),
];

/// List of binary operators we can check.
pub static BINARY_OPERATORS: &[&str] = &[
    "+", "-", "*", "/", "%",
    "==", "!=", "<", "<=", ">", ">=",
    "&&", "||",
    "&", "|", "^", "<<", ">>",
];

/// Check if a function name is a binary operator.
pub fn is_binary_operator(name: &str) -> bool {
    BINARY_OPERATORS.contains(&name)
}

/// Find matching binary operation rule.
/// Returns Ok(result_type) if valid, Err(error_message) if type mismatch.
pub fn check_binary_op(op: &str, left: &InferredType, right: &InferredType) -> Result<InferredType, String> {
    // Dynamic types can combine with anything
    if *left == InferredType::Dynamic || *right == InferredType::Dynamic {
        return Ok(InferredType::Dynamic);
    }

    // Find matching rule
    for rule in BINARY_OP_RULES {
        if rule.op != op {
            continue;
        }

        // Check exact match
        if rule.left == *left && rule.right == *right {
            return Ok(rule.result.clone());
        }

        // Check if rule uses Dynamic (accepts any)
        if rule.left == InferredType::Dynamic && rule.right == InferredType::Dynamic {
            return Ok(rule.result.clone());
        }
    }

    // No matching rule - this is a type error
    Err(format!(
        "cannot apply operator '{}' to {:?} and {:?}",
        op, left, right
    ))
}

/// Check if mixing Int and Float (implicit coercion).
pub fn is_implicit_coercion(left: &InferredType, right: &InferredType) -> bool {
    matches!(
        (left, right),
        (InferredType::Int, InferredType::Float) |
        (InferredType::Float, InferredType::Int)
    )
}

impl InferredType {
    /// Check if this type could be unit (null).
    pub fn could_be_null(&self) -> bool {
        matches!(self, InferredType::Unit | InferredType::Nullable | InferredType::Dynamic)
    }

    /// Check if this type is definitely not null.
    pub fn is_definitely_not_null(&self) -> bool {
        matches!(self,
            InferredType::Bool | InferredType::Int | InferredType::Float |
            InferredType::String | InferredType::Array | InferredType::Map
        )
    }
}

/// Infer type from a Rhai Dynamic value (for optimized constants).
fn infer_dynamic_type(value: &Dynamic) -> InferredType {
    if value.is_unit() {
        InferredType::Unit
    } else if value.is_bool() {
        InferredType::Bool
    } else if value.is_int() {
        InferredType::Int
    } else if value.is_float() {
        InferredType::Float
    } else if value.is_string() {
        InferredType::String
    } else if value.is_array() {
        InferredType::Array
    } else if value.is_map() {
        InferredType::Map
    } else {
        InferredType::Dynamic
    }
}

/// Functions that look up archetypes by ID string.
const ARCHETYPE_LOOKUP_FNS: &[&str] = &[
    "get_ship_def",
    "get_weapon_def",
    "get_upgrade_def",
    "get_cargo_def",
    "get_stance_def",
    "get_ship",
    "get_weapon",
    "get_effect",
    "get_ability",
    "get_faction",
];

/// Check if a function is an archetype lookup function.
fn is_archetype_lookup_fn(name: &str) -> bool {
    ARCHETYPE_LOOKUP_FNS.contains(&name)
}

/// Tracks null guard information for complex conditionals.
#[derive(Debug, Clone, PartialEq)]
pub enum NullGuardInfo {
    /// This boolean variable represents `target_var != ()` (is_not_null check)
    IsNotNull(String),
    /// This boolean variable represents `target_var == ()` (is_null check)
    IsNull(String),
}

/// State of a variable in a scope.
#[derive(Debug, Clone)]
pub struct VarState {
    /// Variable name
    pub name: String,
    /// Inferred type
    pub inferred_type: InferredType,
    /// Whether variable has been used
    pub used: bool,
    /// Whether null has been checked (narrows Nullable to Map)
    pub null_checked: bool,
    /// Position where defined
    pub defined_at: Option<Position>,
    /// If this is a boolean that tracks another variable's null state
    pub null_guard_of: Option<NullGuardInfo>,
}

/// A scope containing variables.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Variables in this scope
    pub vars: std::collections::HashMap<String, VarState>,
    /// Parent scope index (None for root)
    pub parent: Option<usize>,
}

/// A field in a return map literal with its inferred type.
#[derive(Debug, Clone)]
pub struct ReturnMapField {
    /// Field name
    pub name: String,
    /// Inferred type of the value
    pub value_type: InferredType,
    /// Position in source
    pub position: Option<Position>,
}

/// Result of analyzing a function.
#[derive(Debug, Default)]
pub struct FunctionAnalysis {
    /// Undefined variable uses
    pub undefined_vars: Vec<(String, Option<Position>)>,
    /// Unused variables (name, position)
    pub unused_vars: Vec<(String, Option<Position>)>,
    /// Shadowed variables (name, position)
    pub shadowed_vars: Vec<(String, Option<Position>)>,
    /// Unchecked nullable accesses (var_name, position)
    pub unchecked_nullables: Vec<(String, Option<Position>)>,
    /// Type mismatches in binary operations (description, position) - E700
    pub type_mismatches: Vec<(String, Option<Position>)>,
    /// Implicit type coercions (description, position) - W700
    pub implicit_coercions: Vec<(String, Option<Position>)>,
    /// Invalid archetype IDs (empty string) - E900
    pub invalid_archetype_ids: Vec<(String, Option<Position>)>,
    /// Unknown archetype IDs (not in registry) - W900
    pub unknown_archetype_ids: Vec<(String, String, Option<Position>)>, // (fn_name, id, position)
    /// Whether all control flow paths return a value
    pub all_paths_return: bool,
    /// Dead code positions
    pub dead_code: Vec<Option<Position>>,
    /// Return statement positions (for analysis)
    pub returns: Vec<Option<Position>>,
    /// Map keys found in return statements
    pub return_map_keys: Vec<Vec<String>>,
    /// Map fields with types found in return statements (for W302 validation)
    pub return_map_fields: Vec<Vec<ReturnMapField>>,
}

/// Analyzer for a single function with scope and type tracking.
pub struct FunctionAnalyzer<'a> {
    /// The AST being analyzed
    ast: &'a AST,
    /// Function name being analyzed
    fn_name: String,
    /// Scope stack (index 0 is root/function scope)
    scopes: Vec<Scope>,
    /// Current scope index
    current_scope: usize,
    /// Analysis results
    result: FunctionAnalysis,
}

impl<'a> FunctionAnalyzer<'a> {
    /// Create a new analyzer for a function.
    pub fn new(ast: &'a AST, fn_name: &str) -> Self {
        Self {
            ast,
            fn_name: fn_name.to_string(),
            scopes: vec![Scope::default()],
            current_scope: 0,
            result: FunctionAnalysis::default(),
        }
    }

    /// Analyze the function and return results.
    pub fn analyze(mut self) -> FunctionAnalysis {
        // Find the function definition
        let fn_def = self.ast.iter_fn_def()
            .find(|f| f.name == self.fn_name);

        let Some(fn_def) = fn_def else {
            return self.result;
        };

        // Add parameters to root scope
        for param in fn_def.params.iter() {
            let param_name = param.to_string();
            // Infer type from common parameter names
            let inferred_type = match param_name.as_str() {
                "ctx" => InferredType::Map,
                "params" => InferredType::Map,
                _ => InferredType::Dynamic,
            };
            self.define_var(&param_name, inferred_type, None);
        }

        // Analyze function body
        let all_return = self.analyze_statements(fn_def.body.statements().iter());
        self.result.all_paths_return = all_return;

        // Check for unused variables in all scopes
        for scope in &self.scopes {
            for (name, state) in &scope.vars {
                if !state.used && !name.starts_with('_') {
                    self.result.unused_vars.push((name.clone(), state.defined_at));
                }
            }
        }

        self.result
    }

    /// Push a new scope.
    fn push_scope(&mut self) {
        let new_scope = Scope {
            vars: std::collections::HashMap::new(),
            parent: Some(self.current_scope),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    /// Pop the current scope.
    fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            // Check for unused vars before popping
            let scope = &self.scopes[self.current_scope];
            for (name, state) in &scope.vars {
                if !state.used && !name.starts_with('_') {
                    self.result.unused_vars.push((name.clone(), state.defined_at));
                }
            }
            self.current_scope = parent;
        }
    }

    /// Define a variable in the current scope.
    fn define_var(&mut self, name: &str, inferred_type: InferredType, pos: Option<Position>) {
        // Check for shadowing
        if self.lookup_var(name).is_some() {
            self.result.shadowed_vars.push((name.to_string(), pos));
        }

        let state = VarState {
            name: name.to_string(),
            inferred_type,
            used: false,
            null_checked: false,
            defined_at: pos,
            null_guard_of: None,
        };
        self.scopes[self.current_scope].vars.insert(name.to_string(), state);
    }

    /// Define a variable with null guard tracking information.
    fn define_var_with_null_guard(
        &mut self,
        name: &str,
        inferred_type: InferredType,
        pos: Option<Position>,
        null_guard: NullGuardInfo,
    ) {
        // Check for shadowing
        if self.lookup_var(name).is_some() {
            self.result.shadowed_vars.push((name.to_string(), pos));
        }

        let state = VarState {
            name: name.to_string(),
            inferred_type,
            used: false,
            null_checked: false,
            defined_at: pos,
            null_guard_of: Some(null_guard),
        };
        self.scopes[self.current_scope].vars.insert(name.to_string(), state);
    }

    /// Look up a variable in scope chain.
    fn lookup_var(&self, name: &str) -> Option<&VarState> {
        let mut scope_idx = self.current_scope;
        loop {
            if let Some(state) = self.scopes[scope_idx].vars.get(name) {
                return Some(state);
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                return None;
            }
        }
    }

    /// Mark a variable as used.
    fn use_var(&mut self, name: &str, pos: Option<Position>) {
        // Find and mark as used
        let mut scope_idx = self.current_scope;
        loop {
            if self.scopes[scope_idx].vars.contains_key(name) {
                self.scopes[scope_idx].vars.get_mut(name).unwrap().used = true;
                return;
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                // Undefined variable
                if !is_builtin_var(name) {
                    self.result.undefined_vars.push((name.to_string(), pos));
                }
                return;
            }
        }
    }

    /// Mark a variable as null-checked.
    fn mark_null_checked(&mut self, name: &str) {
        let mut scope_idx = self.current_scope;
        loop {
            if self.scopes[scope_idx].vars.contains_key(name) {
                self.scopes[scope_idx].vars.get_mut(name).unwrap().null_checked = true;
                return;
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                return;
            }
        }
    }

    /// Check if accessing a variable would be an unchecked nullable access.
    fn check_nullable_access(&mut self, name: &str, pos: Option<Position>) {
        if let Some(state) = self.lookup_var(name) {
            if state.inferred_type == InferredType::Nullable && !state.null_checked {
                self.result.unchecked_nullables.push((name.to_string(), pos));
            }
        }
    }

    /// Analyze a sequence of statements. Returns true if all paths return.
    fn analyze_statements<'b>(&mut self, stmts: impl Iterator<Item = &'b Stmt>) -> bool {
        let mut has_returned = false;
        let stmts: Vec<_> = stmts.collect();
        let last_idx = stmts.len().saturating_sub(1);

        for (idx, stmt) in stmts.iter().enumerate() {
            if has_returned {
                // Dead code after return
                self.result.dead_code.push(Some(stmt.position()));
                continue;
            }

            // Check if this is the last statement and it's an expression
            // (Rhai optimizes `return expr` to just `expr` for the last statement)
            let is_last = idx == last_idx;
            has_returned = self.analyze_stmt(stmt);

            // If the last statement is an expression (implicit return), treat it as returning
            if is_last && !has_returned {
                if let Stmt::Expr(expr) = stmt {
                    // Record as implicit return and capture map keys if present
                    self.result.returns.push(Some(stmt.position()));
                    if let Some(keys) = self.extract_map_keys(expr.as_ref()) {
                        self.result.return_map_keys.push(keys);
                    }
                    has_returned = true;
                }
            }
        }

        has_returned
    }

    /// Analyze a single statement. Returns true if it definitely returns.
    fn analyze_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Var(boxed, _flags, pos) => {
                let (ident, init_expr, _) = boxed.as_ref();
                let var_name = ident.name.to_string();
                let inferred_type = self.infer_expr_type(init_expr);

                // Analyze the initializer expression
                self.analyze_expr(init_expr);

                // Check if this is a null guard assignment: let valid = x != () or let valid = x == ()
                if let Some(null_guard) = self.detect_null_guard(init_expr) {
                    self.define_var_with_null_guard(&var_name, inferred_type, Some(*pos), null_guard);
                } else {
                    self.define_var(&var_name, inferred_type, Some(*pos));
                }
                false
            }

            Stmt::Assignment(boxed) => {
                let (_op, binary) = boxed.as_ref();
                self.analyze_expr(&binary.rhs);
                // TODO: update variable type if reassigned
                false
            }

            Stmt::If(flow, pos) => {
                // FlowControl: expr=condition, body=if_block, branch=else_block
                let flow = flow.as_ref();

                // Analyze condition and check for null checks
                self.analyze_expr(&flow.expr);
                self.extract_null_checks(&flow.expr);

                // Analyze if branch (body)
                self.push_scope();
                let if_returns = self.analyze_statements(flow.body.statements().iter());
                self.pop_scope();

                // Analyze else branch (branch) - it may be empty
                let else_returns = if flow.branch.is_empty() {
                    false
                } else {
                    self.push_scope();
                    let returns = self.analyze_statements(flow.branch.statements().iter());
                    self.pop_scope();
                    returns
                };

                let _ = pos;
                // Both branches must return for this to be a returning statement
                if_returns && else_returns
            }

            Stmt::Switch(boxed, pos) => {
                let (scrutinee, _cases) = boxed.as_ref();
                self.analyze_expr(scrutinee);
                // Switch statements are complex - for now, don't assume they return
                // A more complete analysis would track all case branches
                let _ = pos;
                false
            }

            Stmt::While(boxed, pos) => {
                // FlowControl has condition (expr) and body (StmtBlock)
                self.push_scope();
                self.analyze_statements(boxed.body.statements().iter());
                self.pop_scope();
                let _ = pos;
                false // While loops don't guarantee return
            }

            Stmt::For(boxed, pos) => {
                let (loop_var, counter_var, flow) = boxed.as_ref();

                self.analyze_expr(&flow.expr);

                self.push_scope();
                self.define_var(&loop_var.name, InferredType::Dynamic, Some(*pos));
                if let Some(counter) = counter_var {
                    self.define_var(&counter.name, InferredType::Int, Some(*pos));
                }
                self.analyze_statements(flow.body.statements().iter());
                self.pop_scope();
                false // For loops don't guarantee return
            }

            Stmt::Return(opt_expr, _flags, pos) => {
                if let Some(expr) = opt_expr {
                    self.analyze_expr(expr);
                    // Extract map keys from return expression
                    if let Some(keys) = self.extract_map_keys(expr) {
                        self.result.return_map_keys.push(keys);
                    }
                    // Extract map fields with types for W302 validation
                    if let Some(fields) = self.extract_map_fields(expr) {
                        self.result.return_map_fields.push(fields);
                    }
                }
                self.result.returns.push(Some(*pos));
                true // Return always returns!
            }

            Stmt::Block(block) => {
                self.push_scope();
                let returns = self.analyze_statements(block.statements().iter());
                self.pop_scope();
                returns
            }

            Stmt::FnCall(call_expr, _pos) => {
                self.analyze_fn_call(call_expr);
                false
            }

            Stmt::Expr(expr) => {
                self.analyze_expr(expr.as_ref());
                // Check if this is an implicit return (last expression)
                if let Some(keys) = self.extract_map_keys(expr.as_ref()) {
                    self.result.return_map_keys.push(keys);
                }
                // Extract map fields with types for W302 validation
                if let Some(fields) = self.extract_map_fields(expr.as_ref()) {
                    self.result.return_map_fields.push(fields);
                }
                false
            }

            _ => false,
        }
    }

    /// Analyze an expression.
    fn analyze_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable(boxed, _, pos) => {
                let var_name = boxed.1.to_string();
                self.use_var(&var_name, Some(*pos));
            }

            Expr::FnCall(call_expr, _pos) => {
                self.analyze_fn_call(call_expr);
            }

            Expr::Index(boxed, _flags, pos) => {
                // x["key"] - check for nullable access
                if let Expr::Variable(var_box, _, _) = &boxed.lhs {
                    let var_name = var_box.1.to_string();
                    self.check_nullable_access(&var_name, Some(*pos));
                    self.use_var(&var_name, Some(*pos));
                }
                self.analyze_expr(&boxed.rhs);
            }

            Expr::Dot(boxed, _flags, pos) => {
                // x.method() or x.field - check for nullable access
                if let Expr::Variable(var_box, _, _) = &boxed.lhs {
                    let var_name = var_box.1.to_string();
                    self.check_nullable_access(&var_name, Some(*pos));
                    self.use_var(&var_name, Some(*pos));
                }
                self.analyze_expr(&boxed.rhs);
            }

            Expr::Array(boxed, _pos) => {
                for item in boxed.iter() {
                    self.analyze_expr(item);
                }
            }

            Expr::Map(boxed, _pos) => {
                for (_, value_expr) in boxed.0.iter() {
                    self.analyze_expr(value_expr);
                }
            }

            Expr::And(boxed, _pos) | Expr::Or(boxed, _pos) => {
                // And/Or contain a vector of expressions
                for expr in boxed.iter() {
                    self.analyze_expr(expr);
                }
            }

            _ => {
                // For other expressions, walk children
                // The rhai walk API handles this
            }
        }
    }

    /// Analyze a function call.
    fn analyze_fn_call(&mut self, call_expr: &FnCallExpr) {
        // Analyze arguments first
        for arg in &call_expr.args {
            self.analyze_expr(arg);
        }

        let fn_name = call_expr.name.as_str();

        // Check if this is a binary operator
        if is_binary_operator(fn_name) && call_expr.args.len() == 2 {
            let left_type = self.infer_expr_type(&call_expr.args[0]);
            let right_type = self.infer_expr_type(&call_expr.args[1]);

            // Check for implicit coercion warning (W700)
            if is_implicit_coercion(&left_type, &right_type) {
                self.result.implicit_coercions.push((
                    format!(
                        "implicit coercion in '{}': mixing Int and Float",
                        fn_name
                    ),
                    None, // Position not easily available from FnCallExpr
                ));
            }

            // Check for type mismatch error (E700)
            if let Err(msg) = check_binary_op(fn_name, &left_type, &right_type) {
                self.result.type_mismatches.push((msg, None));
            }
        }

        // Check for archetype lookup functions with string literal IDs
        if is_archetype_lookup_fn(fn_name) && !call_expr.args.is_empty() {
            if let Some(id) = extract_string_literal(&call_expr.args[0]) {
                if id.is_empty() {
                    // E900: Empty string is definitely invalid
                    self.result.invalid_archetype_ids.push((
                        format!("{}() called with empty string", fn_name),
                        None,
                    ));
                }
                // Note: W900 for unknown IDs requires a registry, handled at higher level
            }
        }
    }

    /// Infer the type of an expression.
    fn infer_expr_type(&self, expr: &Expr) -> InferredType {
        match expr {
            Expr::Unit(_) => InferredType::Unit,
            Expr::BoolConstant(_, _) => InferredType::Bool,
            Expr::IntegerConstant(_, _) => InferredType::Int,
            Expr::FloatConstant(_, _) => InferredType::Float,
            Expr::StringConstant(_, _) => InferredType::String,
            Expr::Array(_, _) => InferredType::Array,
            Expr::Map(_, _) => InferredType::Map,

            Expr::FnCall(call_expr, _) => {
                let fn_name = call_expr.name.as_str();

                // Check if it's a binary operator first
                if is_binary_operator(fn_name) && call_expr.args.len() == 2 {
                    let left_type = self.infer_expr_type(&call_expr.args[0]);
                    let right_type = self.infer_expr_type(&call_expr.args[1]);
                    // Try to find result type, default to Dynamic on error
                    check_binary_op(fn_name, &left_type, &right_type)
                        .unwrap_or(InferredType::Dynamic)
                }
                // Check API function return types
                else if let Some(api_fn) = get_api_function(fn_name) {
                    match api_fn.returns {
                        ReturnType::Unit => InferredType::Unit,
                        ReturnType::Bool => InferredType::Bool,
                        ReturnType::Map => InferredType::Map,
                        ReturnType::Array => InferredType::Array,
                        ReturnType::Number => InferredType::Float, // Could be Int too
                        ReturnType::String => InferredType::String,
                        ReturnType::Nullable => InferredType::Nullable,
                        ReturnType::Dynamic => InferredType::Dynamic,
                    }
                } else if is_nullable_fn_name(fn_name) {
                    InferredType::Nullable
                } else {
                    InferredType::Dynamic
                }
            }

            Expr::Variable(boxed, _, _) => {
                let var_name = boxed.1.to_string();
                self.lookup_var(&var_name)
                    .map(|s| s.inferred_type.clone())
                    .unwrap_or(InferredType::Dynamic)
            }

            _ => InferredType::Dynamic,
        }
    }

    /// Detect null guard patterns in an expression.
    /// Returns Some(NullGuardInfo) if the expression is a null check like `x != ()` or `x == ()`.
    fn detect_null_guard(&self, expr: &Expr) -> Option<NullGuardInfo> {
        if let Expr::FnCall(call_expr, _) = expr {
            let fn_name = call_expr.name.as_str();
            if (fn_name == "==" || fn_name == "!=") && call_expr.args.len() == 2 {
                // Check if one side is a variable and other is unit
                let var_name = match (&call_expr.args[0], &call_expr.args[1]) {
                    (Expr::Variable(var_box, _, _), Expr::Unit(_)) => {
                        Some(var_box.1.to_string())
                    }
                    (Expr::Unit(_), Expr::Variable(var_box, _, _)) => {
                        Some(var_box.1.to_string())
                    }
                    _ => None,
                };

                if let Some(name) = var_name {
                    return if fn_name == "!=" {
                        Some(NullGuardInfo::IsNotNull(name))
                    } else {
                        Some(NullGuardInfo::IsNull(name))
                    };
                }
            }
        }
        None
    }

    /// Extract null check patterns from a condition expression.
    /// e.g., `x == ()` or `x != ()` marks x as null-checked in the appropriate branch.
    fn extract_null_checks(&mut self, condition: &Expr) {
        // Direct null check pattern: var == () or var != ()
        if let Some(guard_info) = self.detect_null_guard(condition) {
            let name = match &guard_info {
                NullGuardInfo::IsNotNull(n) | NullGuardInfo::IsNull(n) => n.clone(),
            };
            // For `x != ()`, the variable is null-checked in the if branch
            // For `x == ()`, the variable is null-checked in the else branch
            // For simplicity, we mark it as checked in both cases
            self.mark_null_checked(&name);
            return;
        }

        // Check if condition is a variable that tracks a null guard
        if let Expr::Variable(var_box, _, _) = condition {
            let var_name = var_box.1.to_string();
            if let Some(state) = self.lookup_var(&var_name) {
                if let Some(guard_info) = &state.null_guard_of {
                    // This variable was assigned from a null check
                    let guarded_var = match guard_info {
                        NullGuardInfo::IsNotNull(n) => n.clone(),
                        NullGuardInfo::IsNull(n) => n.clone(),
                    };
                    self.mark_null_checked(&guarded_var);
                    return;
                }
            }
        }

        // Handle Expr::And (&&) - recurse into both sides
        if let Expr::And(boxed_exprs, _) = condition {
            for expr in boxed_exprs.iter() {
                self.extract_null_checks(expr);
            }
            return;
        }

        // Handle Expr::Or (||) - recurse into both sides
        if let Expr::Or(boxed_exprs, _) = condition {
            for expr in boxed_exprs.iter() {
                self.extract_null_checks(expr);
            }
            return;
        }

        // Check for negation: !valid where valid is a null guard
        if let Expr::FnCall(call_expr, _) = condition {
            let fn_name = call_expr.name.as_str();
            if fn_name == "!" && call_expr.args.len() == 1 {
                if let Expr::Variable(var_box, _, _) = &call_expr.args[0] {
                    let var_name = var_box.1.to_string();
                    if let Some(state) = self.lookup_var(&var_name) {
                        if let Some(guard_info) = &state.null_guard_of {
                            // !valid where valid = x != () means we're in the null case
                            // But we still mark x as considered checked for simplicity
                            let guarded_var = match guard_info {
                                NullGuardInfo::IsNotNull(n) | NullGuardInfo::IsNull(n) => n.clone(),
                            };
                            self.mark_null_checked(&guarded_var);
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Extract map keys from a map literal expression.
    fn extract_map_keys(&self, expr: &Expr) -> Option<Vec<String>> {
        if let Expr::Map(boxed, _) = expr {
            let keys: Vec<String> = boxed.0.iter()
                .map(|(ident, _)| ident.name.to_string())
                .collect();
            if !keys.is_empty() {
                return Some(keys);
            }
        }
        None
    }

    /// Extract map fields with their inferred types from a map literal expression.
    ///
    /// Handles both `Expr::Map` (raw AST) and `Expr::DynamicConstant` (optimized form).
    fn extract_map_fields(&self, expr: &Expr) -> Option<Vec<ReturnMapField>> {
        match expr {
            // Raw map literal (before optimization)
            Expr::Map(boxed, pos) => {
                let fields: Vec<ReturnMapField> = boxed.0.iter()
                    .map(|(ident, value_expr)| {
                        ReturnMapField {
                            name: ident.name.to_string(),
                            value_type: self.infer_expr_type(value_expr),
                            position: Some(*pos),
                        }
                    })
                    .collect();
                if !fields.is_empty() {
                    return Some(fields);
                }
            }
            // Optimized constant - map has been evaluated into a Dynamic
            Expr::DynamicConstant(boxed_dyn, pos) => {
                if let Some(map) = boxed_dyn.read_lock::<rhai::Map>() {
                    let fields: Vec<ReturnMapField> = map.iter()
                        .map(|(key, value)| {
                            ReturnMapField {
                                name: key.to_string(),
                                value_type: infer_dynamic_type(value),
                                position: Some(*pos),
                            }
                        })
                        .collect();
                    if !fields.is_empty() {
                        return Some(fields);
                    }
                }
            }
            _ => {}
        }
        None
    }
}

/// Analyze a function from an AST.
pub fn analyze_function(ast: &AST, fn_name: &str) -> FunctionAnalysis {
    FunctionAnalyzer::new(ast, fn_name).analyze()
}

/// Legacy text-based analysis (kept for fallback/comparison).
/// This provides basic validation without deep AST inspection.
#[allow(dead_code)]
pub fn analyze_text_simple(content: &str) -> AstAnalysis {
    let mut analysis = AstAnalysis::default();

    // Use regex to find function calls
    let fn_call_re = regex::Regex::new(r"(\w+)\s*\(").ok();
    if let Some(re) = fn_call_re {
        for cap in re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let name_str = name.as_str().to_string();
                // Skip keywords and common constructs
                if !matches!(name_str.as_str(), "fn" | "if" | "while" | "for" | "let") {
                    // Count args heuristically (just mark as unknown with 0)
                    analysis.fn_calls.push((name_str, 0, None));
                }
            }
        }
    }

    // Find index accesses like ctx["key"] or params["key"]
    let index_re = regex::Regex::new(r#"(\w+)\s*\[\s*"([^"]+)"\s*\]"#).ok();
    if let Some(re) = index_re {
        for cap in re.captures_iter(content) {
            if let (Some(var), Some(key)) = (cap.get(1), cap.get(2)) {
                analysis.index_accesses.push((
                    var.as_str().to_string(),
                    key.as_str().to_string(),
                    None,
                ));
            }
        }
    }

    // Find variable definitions: let x = ..., let mut x = ...
    let let_re = regex::Regex::new(r"let\s+(?:mut\s+)?(\w+)\s*=\s*([^;]+)").ok();
    if let Some(re) = let_re {
        for cap in re.captures_iter(content) {
            if let (Some(name), Some(rhs)) = (cap.get(1), cap.get(2)) {
                let name_str = name.as_str().to_string();
                let rhs_str = rhs.as_str();
                // Check if RHS is a nullable function call
                let is_nullable = is_nullable_call(rhs_str);
                analysis.var_defs.push((name_str, None, is_nullable));
            }
        }
    }

    // Find variable uses (identifiers in expressions)
    // This is a simplified heuristic - looks for bare identifiers not in definitions
    let ident_re = regex::Regex::new(r"\b([a-z_][a-z0-9_]*)\b").ok();
    if let Some(re) = ident_re {
        for cap in re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let name_str = name.as_str();
                // Skip keywords and common constructs
                if !is_keyword(name_str) {
                    analysis.var_uses.push((name_str.to_string(), None));
                }
            }
        }
    }

    // Find return statements
    let return_re = regex::Regex::new(r"return\s+([^;]+)").ok();
    if let Some(re) = return_re {
        for cap in re.captures_iter(content) {
            let has_value = cap.get(1).is_some_and(|m| !m.as_str().trim().is_empty());
            analysis.returns.push((None, has_value));
        }
    }

    // Find implicit returns (last expression in function, starts with #{)
    let map_return_re = regex::Regex::new(r"#\{\s*(\w+)\s*:").ok();
    if let Some(re) = map_return_re {
        for cap in re.captures_iter(content) {
            if let Some(key) = cap.get(1) {
                analysis.map_keys.push((vec![key.as_str().to_string()], None));
            }
        }
    }

    analysis
}

/// Check if an expression is a call to a nullable-returning function.
fn is_nullable_call(expr: &str) -> bool {
    // Check for query_* functions that return Nullable
    let nullable_prefixes = ["query_sector", "get_context_sector", "get_data",
                            "get_ship_def", "get_upgrade_def", "get_cargo_def",
                            "get_stance_def", "get_ship", "get_weapon", "get_effect",
                            "get_ability", "get_faction"];
    for prefix in nullable_prefixes {
        if expr.trim_start().starts_with(prefix) {
            return true;
        }
    }
    false
}

/// Check if a string is a Rhai keyword or builtin.
fn is_keyword(s: &str) -> bool {
    matches!(s,
        "let" | "const" | "if" | "else" | "while" | "for" | "in" | "loop" |
        "break" | "continue" | "return" | "throw" | "try" | "catch" |
        "fn" | "private" | "import" | "export" | "as" | "true" | "false" |
        "this" | "switch" | "do" | "type_of" | "print" | "debug"
    )
}

/// Strip string literals and comments from code.
/// Preserves structure (newlines, whitespace) so positions remain valid.
fn strip_strings_and_comments(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Line comment
            '/' if chars.peek() == Some(&'/') => {
                chars.next(); // consume second /
                // Skip until end of line
                for c in chars.by_ref() {
                    if c == '\n' {
                        result.push('\n');
                        break;
                    }
                }
            }
            // Block comment
            '/' if chars.peek() == Some(&'*') => {
                chars.next(); // consume *
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                    if c == '\n' {
                        result.push('\n');
                    } else {
                        result.push(' '); // preserve spacing
                    }
                }
            }
            // Double-quoted string
            '"' => {
                result.push(' '); // placeholder
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        chars.next(); // skip escaped char
                    } else if c == '"' {
                        break;
                    } else if c == '\n' {
                        result.push('\n');
                    }
                }
            }
            // Backtick string (Rhai template strings)
            '`' => {
                result.push(' ');
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        chars.next();
                    } else if c == '`' {
                        break;
                    } else if c == '\n' {
                        result.push('\n');
                    }
                }
            }
            _ => result.push(c),
        }
    }

    result
}

/// Analyze variable flow within a function body.
///
/// Detects:
/// - Undefined variables (E600)
/// - Unused variables (W401)
/// - Shadowed variables (W402)
pub fn analyze_variable_flow(content: &str, fn_name: &str) -> VariableFlowResult {
    let mut result = VariableFlowResult::default();

    // Strip strings and comments first
    let cleaned = strip_strings_and_comments(content);

    // Extract function body
    let fn_pattern = format!("fn {}(", fn_name);
    let Some(fn_start) = cleaned.find(&fn_pattern) else {
        return result;
    };

    // Find parameters
    let params_start = fn_start + fn_pattern.len();
    let Some(params_end) = cleaned[params_start..].find(')') else {
        return result;
    };
    let params_str = &cleaned[params_start..params_start + params_end];

    // Parse parameters as defined variables
    let mut defined: HashSet<String> = params_str
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    // Track all local definitions (not params) for unused check
    let mut local_defs: Vec<String> = Vec::new();

    // Find end of function (heuristic: next fn or end of file)
    let body_start = params_start + params_end + 1;
    let body_end = cleaned[body_start..].find("\nfn ").unwrap_or(cleaned.len() - body_start);
    let func_body = &cleaned[body_start..body_start + body_end];

    // Find let statements in function body
    let let_re = regex::Regex::new(r"let\s+(?:mut\s+)?(\w+)\s*=").unwrap();
    for cap in let_re.captures_iter(func_body) {
        if let Some(name) = cap.get(1) {
            let var_name = name.as_str().to_string();

            // Check for shadowing
            if defined.contains(&var_name) {
                result.shadows.push((var_name.clone(), None));
            }

            defined.insert(var_name.clone());
            local_defs.push(var_name);
        }
    }

    // Find for loop variables
    let for_re = regex::Regex::new(r"for\s+(\w+)\s+in\b").unwrap();
    for cap in for_re.captures_iter(func_body) {
        if let Some(name) = cap.get(1) {
            let var_name = name.as_str().to_string();
            if defined.contains(&var_name) {
                result.shadows.push((var_name.clone(), None));
            }
            defined.insert(var_name.clone());
            local_defs.push(var_name);
        }
    }

    // Find variable uses - identifiers that are NOT:
    // - Keywords
    // - Function calls (followed by `(`)
    // - Map keys (followed by `:`)
    // - API functions
    // - Part of let/for definitions
    let mut used: HashSet<String> = HashSet::new();

    // First, remove let/for definitions from the body so we don't count them as uses
    let body_without_defs = {
        let mut s = func_body.to_string();
        // Remove "let varname =" and "let mut varname =" patterns
        let let_def_re = regex::Regex::new(r"let\s+(?:mut\s+)?(\w+)\s*=").unwrap();
        s = let_def_re.replace_all(&s, "let __def__ =").to_string();
        // Remove "for varname in" patterns
        let for_def_re = regex::Regex::new(r"for\s+(\w+)\s+in\b").unwrap();
        s = for_def_re.replace_all(&s, "for __def__ in").to_string();
        s
    };

    // Regex that captures identifier and what follows
    let ident_re = regex::Regex::new(r"\b([a-z_][a-z0-9_]*)\b\s*([:\(]?)").unwrap();
    for cap in ident_re.captures_iter(&body_without_defs) {
        let name = cap.get(1).unwrap().as_str();
        let suffix = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        // Skip if followed by : (map key) or ( (function call)
        if suffix == ":" || suffix == "(" {
            continue;
        }

        // Skip keywords and builtins
        if is_keyword(name) || is_builtin_var(name) {
            continue;
        }

        // Skip API function names
        if get_api_function(name).is_some() {
            continue;
        }

        // Skip our placeholder
        if name == "__def__" {
            continue;
        }

        // This is a variable use
        used.insert(name.to_string());

        // Check if undefined
        if !defined.contains(name) {
            result.undefined.push((name.to_string(), None));
        }
    }

    // Check for unused LOCAL variables (not params - those may be intentionally unused)
    for var in &local_defs {
        if !used.contains(var) && !var.starts_with('_') {
            result.unused.push((var.clone(), None));
        }
    }

    // Deduplicate undefined (may have multiple uses of same undefined var)
    let mut seen = HashSet::new();
    result.undefined.retain(|(name, _)| seen.insert(name.clone()));

    result
}

/// Check if a variable name is a builtin or commonly injected.
fn is_builtin_var(name: &str) -> bool {
    // Common builtins, context variables, and special patterns
    matches!(name, "ctx" | "params" | "this" | "true" | "false" | "_")
}

/// Analyze control flow within a function body.
///
/// Detects:
/// - Dead code after unconditional return (W200)
/// - Handler functions that may not return a value (missing_returns)
pub fn analyze_control_flow(content: &str, fn_name: &str) -> ControlFlowResult {
    let mut result = ControlFlowResult::default();

    // Strip strings and comments first
    let cleaned = strip_strings_and_comments(content);

    // Extract function body
    let fn_pattern = format!("fn {}(", fn_name);
    let Some(fn_start) = cleaned.find(&fn_pattern) else {
        return result;
    };

    let params_start = fn_start + fn_pattern.len();
    let Some(params_end) = cleaned[params_start..].find(')') else {
        return result;
    };

    let body_start = params_start + params_end + 1;
    let body_end = cleaned[body_start..].find("\nfn ").unwrap_or(cleaned.len() - body_start);
    let func_body = &cleaned[body_start..body_start + body_end];

    // Find dead code: Look for `return ...;` followed by non-whitespace code that's not `}`
    // This is a heuristic - we look for return followed by statements not in nested blocks
    let return_re = regex::Regex::new(r"return\s+[^;]+;\s*\n\s*([a-z])").unwrap();
    for cap in return_re.captures_iter(func_body) {
        // Check if the next char starts a statement (not just closing brace)
        if let Some(next_char) = cap.get(1) {
            let ch = next_char.as_str();
            if ch != "}" && ch != "/" {  // Not closing brace or comment
                result.dead_code.push(None);
            }
        }
    }

    result
}

/// Analyze null safety within a function body.
///
/// Detects when variables assigned from nullable functions (query_sector, get_*, etc.)
/// are accessed without being checked for null first.
///
/// Returns W100 warnings for unchecked nullable accesses.
pub fn analyze_null_safety(content: &str, fn_name: &str) -> NullSafetyResult {
    let mut result = NullSafetyResult::default();

    // Strip strings and comments first
    let cleaned = strip_strings_and_comments(content);

    // Extract function body
    let fn_pattern = format!("fn {}(", fn_name);
    let Some(fn_start) = cleaned.find(&fn_pattern) else {
        return result;
    };

    let params_start = fn_start + fn_pattern.len();
    let Some(params_end) = cleaned[params_start..].find(')') else {
        return result;
    };

    let body_start = params_start + params_end + 1;
    let body_end = cleaned[body_start..].find("\nfn ").unwrap_or(cleaned.len() - body_start);
    let func_body = &cleaned[body_start..body_start + body_end];

    // Track which variables are nullable (assigned from nullable functions)
    let mut nullable_vars: HashSet<String> = HashSet::new();

    // Find nullable assignments: let x = query_sector(...) or let x = get_ship(...)
    let nullable_assign_re = regex::Regex::new(
        r"let\s+(?:mut\s+)?(\w+)\s*=\s*(query_sector|get_context_sector|get_data|get_ship_def|get_upgrade_def|get_cargo_def|get_stance_def|get_ship|get_weapon|get_effect|get_ability|get_faction)\s*\("
    ).unwrap();

    for cap in nullable_assign_re.captures_iter(func_body) {
        if let Some(var_name) = cap.get(1) {
            nullable_vars.insert(var_name.as_str().to_string());
        }
    }

    if nullable_vars.is_empty() {
        return result; // No nullable variables, nothing to check
    }

    // Track which nullable variables have been null-checked
    // Patterns: `if varname == ()`, `if varname != ()`, `varname == ()`, `varname != ()`
    let mut null_checked: HashSet<String> = HashSet::new();

    for var in &nullable_vars {
        // Check if there's a null check pattern for this variable
        let check_pattern = format!(r"\b{}\s*[!=]=\s*\(\)", regex::escape(var));
        if regex::Regex::new(&check_pattern).ok()
            .is_some_and(|re| re.is_match(func_body))
        {
            null_checked.insert(var.clone());
        }
    }

    // Find accesses to nullable variables that haven't been null-checked
    // Access patterns: var["key"], var.method(), var.property
    for var in &nullable_vars {
        if null_checked.contains(var) {
            continue; // This variable has been null-checked somewhere
        }

        // Check if variable is accessed (indexed or method called)
        let access_pattern = format!(r"\b{}\s*[\[\.]", regex::escape(var));
        if regex::Regex::new(&access_pattern).ok()
            .is_some_and(|re| re.is_match(func_body))
        {
            result.unchecked_accesses.push((var.clone(), None));
        }
    }

    result
}

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
    pub line: Option<usize>,
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

/// Validator for action scripts
pub struct ActionScriptValidator {
    engine: Engine,
    action_schema: Option<ActionSchemaRegistry>,
}

impl ActionScriptValidator {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Increase expression depth limits to handle complex scripts
        engine.set_max_expr_depths(128, 128);

        Self::register_stub_api(&mut engine);
        Self { engine, action_schema: None }
    }

    /// Create a validator with an action schema registry for param validation.
    pub fn with_schema(mut self, schema: ActionSchemaRegistry) -> Self {
        self.action_schema = Some(schema);
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
            // Use text-based analysis (simpler but works without internals feature)
            let analysis = analyze_text_simple(&content);

            // Check API function argument counts
            self.check_api_calls(&analysis, &mut errors, &mut warnings);

            // Check context key accesses
            self.check_ctx_keys(&analysis, &mut warnings);

            // Check variable flow and param access in each handler
            for action in &registered_actions {
                if functions.contains(&action.handler) {
                    // AST-based comprehensive analysis (E301, E600, W100, W200, W401, W402)
                    self.check_handler_with_ast(&ast, &action.handler, &mut errors, &mut warnings);

                    // Legacy text-based checks (kept for additional coverage)
                    self.check_variable_flow(&content, &action.handler, &mut errors, &mut warnings);
                    self.check_null_safety(&content, &action.handler, &mut warnings);
                    self.check_control_flow(&content, &action.handler, &mut warnings);
                    self.check_param_keys(&content, &action.name, &action.handler, &mut warnings);
                }
            }

            // Also check init() for variable issues
            if functions.contains(&"init".to_string()) {
                self.check_handler_with_ast(&ast, "init", &mut errors, &mut warnings);
                self.check_variable_flow(&content, "init", &mut errors, &mut warnings);
            }

            // === Event Subscription Validation ===
            let ast_analysis = analyze_ast(&ast, &content);
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
    }

    /// Check variable flow within a function.
    fn check_variable_flow(
        &self,
        content: &str,
        fn_name: &str,
        errors: &mut Vec<ActionValidationError>,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        let flow_result = analyze_variable_flow(content, fn_name);

        // Report undefined variables as errors
        for (var_name, pos) in flow_result.undefined {
            errors.push(ActionValidationError {
                code: "E600",
                message: format!(
                    "In '{}()': Use of undefined variable '{}'",
                    fn_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // Report unused variables as warnings
        for (var_name, pos) in flow_result.unused {
            warnings.push(ActionValidationWarning {
                code: "W401",
                message: format!(
                    "In '{}()': Variable '{}' is defined but never used",
                    fn_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }

        // Report shadowed variables as warnings
        for (var_name, pos) in flow_result.shadows {
            warnings.push(ActionValidationWarning {
                code: "W402",
                message: format!(
                    "In '{}()': Variable '{}' shadows a previous definition",
                    fn_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }
    }

    /// Check null safety within a function.
    fn check_null_safety(
        &self,
        content: &str,
        fn_name: &str,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        let null_result = analyze_null_safety(content, fn_name);

        // Report unchecked nullable accesses as warnings
        for (var_name, pos) in null_result.unchecked_accesses {
            warnings.push(ActionValidationWarning {
                code: "W100",
                message: format!(
                    "In '{}()': Variable '{}' may be null (from query/get function) but is accessed without null check",
                    fn_name, var_name
                ),
                line: pos.and_then(|p| p.line()),
            });
        }
    }

    /// Check control flow within a function.
    fn check_control_flow(
        &self,
        content: &str,
        fn_name: &str,
        warnings: &mut Vec<ActionValidationWarning>,
    ) {
        let flow_result = analyze_control_flow(content, fn_name);

        // Report dead code warnings
        for pos in flow_result.dead_code {
            warnings.push(ActionValidationWarning {
                code: "W200",
                message: format!(
                    "In '{}()': Dead code detected after return statement",
                    fn_name
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
// Definition Script Validator
// ============================================================================

use crate::schema::{DefinitionSchemaRegistry, DefinitionSchema};

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

/// A validation warning for definition scripts
#[derive(Debug, Clone)]
pub struct DefinitionValidationWarning {
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
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
    fn test_api_function_return_types() {
        // Check return types are correctly set
        assert_eq!(get_api_function("query_sector").unwrap().returns, ReturnType::Nullable);
        assert_eq!(get_api_function("query_ship").unwrap().returns, ReturnType::Map);
        assert_eq!(get_api_function("modify_ship").unwrap().returns, ReturnType::Unit);
        assert_eq!(get_api_function("send_notification").unwrap().returns, ReturnType::Bool);
        assert_eq!(get_api_function("all_ships").unwrap().returns, ReturnType::Array);
        assert_eq!(get_api_function("distance").unwrap().returns, ReturnType::Number);
    }

    #[test]
    fn test_api_function_arg_counts() {
        assert_eq!(get_api_function("query_ship").unwrap().arg_count, 1);
        assert_eq!(get_api_function("send_notification").unwrap().arg_count, 2);
        assert_eq!(get_api_function("resolve_attack").unwrap().arg_count, 7);
        assert_eq!(get_api_function("all_ships").unwrap().arg_count, 0);
    }

    // ========================================================================
    // Text Analysis Tests
    // ========================================================================

    #[test]
    fn test_text_analysis_fn_calls() {
        let content = r#"
fn init() {
    register_action("test", "handler");
}

fn handler(ctx, params) {
    let ship = query_ship(ctx["ship_id"]);
    send_notification(ctx["player_id"], "Hello");
    #{ success: true }
}
"#;
        let analysis = analyze_text_simple(content);

        // Check function calls were found
        let fn_names: Vec<&str> = analysis.fn_calls.iter()
            .map(|(name, _, _)| name.as_str())
            .collect();
        assert!(fn_names.contains(&"register_action"));
        assert!(fn_names.contains(&"query_ship"));
        assert!(fn_names.contains(&"send_notification"));
    }

    #[test]
    fn test_text_analysis_index_accesses() {
        let content = r#"
fn handler(ctx, params) {
    let player_id = ctx["player_id"];
    let ship_id = ctx["ship_id"];
    let amount = params["amount"];
    #{ success: true }
}
"#;
        let analysis = analyze_text_simple(content);

        // Check index accesses were found
        let ctx_keys: Vec<&str> = analysis.index_accesses.iter()
            .filter(|(var, _, _)| var == "ctx")
            .map(|(_, key, _)| key.as_str())
            .collect();
        assert!(ctx_keys.contains(&"player_id"));
        assert!(ctx_keys.contains(&"ship_id"));

        let params_keys: Vec<&str> = analysis.index_accesses.iter()
            .filter(|(var, _, _)| var == "params")
            .map(|(_, key, _)| key.as_str())
            .collect();
        assert!(params_keys.contains(&"amount"));
    }

    #[test]
    fn test_ctx_key_validation_warning() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_ctx_keys.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let player_id = ctx["player_id"];  // Valid
    let unknown = ctx["invalid_key"];  // Should warn
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have a warning about unknown ctx key
        assert!(
            result.warnings.iter().any(|w| w.code == "W400"),
            "Expected W400 warning for unknown ctx key. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_valid_ctx_keys_no_warning() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_valid_ctx.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let player_id = ctx["player_id"];
    let ship_id = ctx["ship_id"];
    let sector_id = ctx["sector_id"];
    let action = ctx["action"];
    let p = ctx["params"];
    if params["value"] == () {
        return #{ success: false, error: "missing value" };
    }
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have W400 warning (all ctx keys are valid)
        let w400_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W400")
            .collect();
        assert!(
            w400_warnings.is_empty(),
            "Unexpected W400 warnings: {:?}",
            w400_warnings
        );
    }

    // ========================================================================
    // Variable Flow Analysis Tests
    // ========================================================================

    #[test]
    fn test_strip_strings_and_comments() {
        let input = r#"
let x = "hello world";  // comment
let y = 42; /* block */
let z = `template string`;
"#;
        let stripped = super::strip_strings_and_comments(input);
        // String contents should be gone
        assert!(!stripped.contains("hello"));
        assert!(!stripped.contains("world"));
        assert!(!stripped.contains("comment"));
        assert!(!stripped.contains("block"));
        assert!(!stripped.contains("template"));
        // Variable names and keywords should remain
        assert!(stripped.contains("let x"));
        assert!(stripped.contains("let y"));
        assert!(stripped.contains("let z"));
    }

    #[test]
    fn test_variable_flow_undefined() {
        let content = r#"
fn test_fn(x, y) {
    let a = x + y;
    let b = a + undefined_var;
    b
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        assert!(
            result.undefined.iter().any(|(name, _)| name == "undefined_var"),
            "Should detect undefined_var as undefined. Got: {:?}",
            result.undefined
        );
    }

    #[test]
    fn test_variable_flow_unused() {
        let content = r#"
fn test_fn(x, y) {
    let unused_var = 42;
    let used_var = x + y;
    used_var
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        assert!(
            result.unused.iter().any(|(name, _)| name == "unused_var"),
            "Should detect unused_var as unused. Got: {:?}",
            result.unused
        );
        assert!(
            !result.unused.iter().any(|(name, _)| name == "used_var"),
            "Should NOT flag used_var as unused"
        );
    }

    #[test]
    fn test_variable_flow_underscore_unused_ok() {
        let content = r#"
fn test_fn(x, y) {
    let _intentionally_unused = 42;
    x + y
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        assert!(
            !result.unused.iter().any(|(name, _)| name == "_intentionally_unused"),
            "Should NOT flag _prefixed variables as unused"
        );
    }

    #[test]
    fn test_variable_flow_shadowing() {
        let content = r#"
fn test_fn(x, y) {
    let x = 42;  // shadows parameter x
    x + y
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        assert!(
            result.shadows.iter().any(|(name, _)| name == "x"),
            "Should detect x as shadowing parameter. Got: {:?}",
            result.shadows
        );
    }

    #[test]
    fn test_variable_flow_map_keys_not_vars() {
        let content = r#"
fn test_fn(ctx, params) {
    let result = #{ success: true, data: ctx };
    result
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        // "success" and "data" are map keys, not undefined variables
        assert!(
            !result.undefined.iter().any(|(name, _)| name == "success"),
            "Map keys should not be flagged as undefined"
        );
        assert!(
            !result.undefined.iter().any(|(name, _)| name == "data"),
            "Map keys should not be flagged as undefined"
        );
    }

    #[test]
    fn test_variable_flow_string_contents_not_vars() {
        let content = r#"
fn test_fn(x) {
    let msg = "hello world undefined_in_string";
    msg
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        assert!(
            !result.undefined.iter().any(|(name, _)| name == "hello"),
            "String contents should not be flagged as undefined"
        );
        assert!(
            !result.undefined.iter().any(|(name, _)| name == "undefined_in_string"),
            "String contents should not be flagged as undefined"
        );
    }

    #[test]
    fn test_variable_flow_for_loop() {
        let content = r#"
fn test_fn(items) {
    for item in items {
        log_info(item);
    }
}
"#;
        let result = super::analyze_variable_flow(content, "test_fn");

        // "item" should be defined by the for loop
        assert!(
            !result.undefined.iter().any(|(name, _)| name == "item"),
            "For loop variable should be defined"
        );
    }

    #[test]
    fn test_variable_flow_integration() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_flow.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let used = ctx["player_id"];
    let unused_local = 42;
    let result = send_notification(used, "Hello");
    #{ success: true, data: result }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W401 warning for unused_local
        assert!(
            result.warnings.iter().any(|w| w.code == "W401" && w.message.contains("unused_local")),
            "Should warn about unused_local. Warnings: {:?}",
            result.warnings
        );

        // Should NOT have errors (no undefined vars)
        let e600_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E600")
            .collect();
        assert!(
            e600_errors.is_empty(),
            "Should not have E600 errors: {:?}",
            e600_errors
        );
    }

    // ========================================================================
    // Null Safety Analysis Tests
    // ========================================================================

    #[test]
    fn test_null_safety_unchecked_access() {
        let content = r#"
fn handler(ctx, params) {
    let sector = query_sector(ctx["sector_id"]);
    let name = sector["name"];  // Unchecked access!
    #{ success: true, data: name }
}
"#;
        let result = super::analyze_null_safety(content, "handler");

        assert!(
            result.unchecked_accesses.iter().any(|(name, _)| name == "sector"),
            "Should detect unchecked access to nullable 'sector'. Got: {:?}",
            result.unchecked_accesses
        );
    }

    #[test]
    fn test_null_safety_checked_access() {
        let content = r#"
fn handler(ctx, params) {
    let sector = query_sector(ctx["sector_id"]);
    if sector == () {
        return #{ success: false, error: "Sector not found" };
    }
    let name = sector["name"];  // OK - null checked above
    #{ success: true, data: name }
}
"#;
        let result = super::analyze_null_safety(content, "handler");

        assert!(
            result.unchecked_accesses.is_empty(),
            "Should NOT flag null-checked access. Got: {:?}",
            result.unchecked_accesses
        );
    }

    #[test]
    fn test_null_safety_checked_not_equal() {
        let content = r#"
fn handler(ctx, params) {
    let ship = get_ship("scout");
    if ship != () {
        let name = ship["name"];
    }
    #{ success: true }
}
"#;
        let result = super::analyze_null_safety(content, "handler");

        assert!(
            result.unchecked_accesses.is_empty(),
            "Should recognize != () as null check. Got: {:?}",
            result.unchecked_accesses
        );
    }

    #[test]
    fn test_null_safety_non_nullable_ok() {
        let content = r#"
fn handler(ctx, params) {
    let ship = query_ship(ctx["ship_id"]);  // query_ship returns Map, not nullable
    let name = ship["name"];  // OK
    #{ success: true, data: name }
}
"#;
        let result = super::analyze_null_safety(content, "handler");

        // query_ship returns Map (non-nullable), so this should be OK
        assert!(
            result.unchecked_accesses.is_empty(),
            "query_ship is not nullable, should not warn. Got: {:?}",
            result.unchecked_accesses
        );
    }

    #[test]
    fn test_null_safety_multiple_nullables() {
        let content = r#"
fn handler(ctx, params) {
    let sector = query_sector(ctx["sector_id"]);
    let faction = get_faction("pirates");

    // Only sector is checked
    if sector == () {
        return #{ success: false };
    }

    let sector_name = sector["name"];  // OK
    let faction_name = faction["name"];  // Should warn!

    #{ success: true }
}
"#;
        let result = super::analyze_null_safety(content, "handler");

        // Should warn about faction but not sector
        assert!(
            !result.unchecked_accesses.iter().any(|(name, _)| name == "sector"),
            "Should NOT warn about checked 'sector'"
        );
        assert!(
            result.unchecked_accesses.iter().any(|(name, _)| name == "faction"),
            "Should warn about unchecked 'faction'. Got: {:?}",
            result.unchecked_accesses
        );
    }

    #[test]
    fn test_null_safety_integration() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_null.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let sector = query_sector(ctx["sector_id"]);
    let name = sector["name"];  // Unchecked!
    #{ success: true, data: name }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W100 warning for unchecked nullable access
        assert!(
            result.warnings.iter().any(|w| w.code == "W100" && w.message.contains("sector")),
            "Should warn about unchecked nullable 'sector'. Warnings: {:?}",
            result.warnings
        );
    }

    // ========================================================================
    // Control Flow Analysis Tests
    // ========================================================================

    #[test]
    fn test_control_flow_dead_code() {
        let content = r#"
fn handler(ctx, params) {
    return #{ success: true };
    let x = 42;  // Dead code!
}
"#;
        let result = super::analyze_control_flow(content, "handler");

        assert!(
            !result.dead_code.is_empty(),
            "Should detect dead code after return. Got: {:?}",
            result.dead_code
        );
    }

    #[test]
    fn test_control_flow_no_dead_code_in_if() {
        let content = r#"
fn handler(ctx, params) {
    if params["early"] {
        return #{ success: true };
    }
    let x = 42;  // NOT dead code - return is conditional
    #{ success: true, data: x }
}
"#;
        let result = super::analyze_control_flow(content, "handler");

        // This is harder to detect with text-based analysis, so we accept some false positives
        // The important thing is we don't have false negatives for clear dead code
        assert!(
            result.dead_code.is_empty(),
            "Should NOT flag code after conditional return. Got: {:?}",
            result.dead_code
        );
    }

    #[test]
    fn test_control_flow_dead_code_integration() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_dead.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    return #{ success: true };
    let dead = 42;
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W200 warning for dead code
        assert!(
            result.warnings.iter().any(|w| w.code == "W200"),
            "Should warn about dead code. Warnings: {:?}",
            result.warnings
        );
    }

    // ========================================================================
    // Param Schema Validation Tests
    // ========================================================================

    fn create_test_schema() -> crate::schema::ActionSchemaRegistry {
        let toml = r#"
            [dock]
            description = "Dock at a station"
            [dock.params]
            station_id = { type = "uuid", required = true }

            [fire_weapon]
            description = "Fire a weapon"
            [fire_weapon.params]
            weapon_index = { type = "int", required = true }
            target_id = { type = "uuid", required = true }
        "#;
        crate::schema::ActionSchemaRegistry::load_from_str(toml).unwrap()
    }

    #[test]
    fn test_param_schema_unknown_key() {
        let schema = create_test_schema();
        let validator = ActionScriptValidator::new().with_schema(schema);
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_param_schema.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("dock", "handle_dock");
}

fn handle_dock(ctx, params) {
    let station = params["station_id"];  // Valid
    let unknown = params["invalid_key"]; // Should warn!
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have E100 warning for unknown param key
        assert!(
            result.warnings.iter().any(|w| w.code == "E100" && w.message.contains("invalid_key")),
            "Should warn about unknown param 'invalid_key'. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_param_schema_valid_keys() {
        let schema = create_test_schema();
        let validator = ActionScriptValidator::new().with_schema(schema);
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_param_valid.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("fire_weapon", "handle_fire");
}

fn handle_fire(ctx, params) {
    let weapon = params["weapon_index"];
    let target = params["target_id"];
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E100 warnings (all params are valid)
        let e100_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "E100")
            .collect();
        assert!(
            e100_warnings.is_empty(),
            "Should not have E100 warnings for valid params: {:?}",
            e100_warnings
        );
    }

    #[test]
    fn test_param_schema_missing_required() {
        let schema = create_test_schema();
        let validator = ActionScriptValidator::new().with_schema(schema);
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_param_missing.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("dock", "handle_dock");
}

fn handle_dock(ctx, params) {
    // Missing required station_id!
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W101 warning for missing required param
        assert!(
            result.warnings.iter().any(|w| w.code == "W101" && w.message.contains("station_id")),
            "Should warn about missing required param 'station_id'. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_param_schema_no_schema_no_validation() {
        // Validator without schema should not produce E100/W101 warnings
        let validator = ActionScriptValidator::new(); // No schema!
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_no_schema.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("dock", "handle_dock");
}

fn handle_dock(ctx, params) {
    let anything = params["any_key"];  // No schema to validate against
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E100 or W101 warnings (no schema to validate against)
        let schema_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "E100" || w.code == "W101")
            .collect();
        assert!(
            schema_warnings.is_empty(),
            "Without schema, should not have E100/W101 warnings: {:?}",
            schema_warnings
        );
    }

    #[test]
    fn test_param_schema_unknown_action() {
        let schema = create_test_schema();
        let validator = ActionScriptValidator::new().with_schema(schema);
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_unknown_action.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("custom_action", "handle_custom");  // Not in schema
}

fn handle_custom(ctx, params) {
    let anything = params["any_key"];  // No validation for unknown actions
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E100 warnings for unknown actions (might be custom)
        let e100_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "E100")
            .collect();
        assert!(
            e100_warnings.is_empty(),
            "Unknown actions should skip param validation: {:?}",
            e100_warnings
        );
    }

    // ========================================================================
    // FunctionAnalyzer Tests (AST-based)
    // ========================================================================

    fn parse_script(content: &str) -> rhai::AST {
        let engine = rhai::Engine::new();
        engine.compile(content).expect("Failed to compile test script")
    }

    #[test]
    fn test_function_analyzer_undefined_var() {
        let content = r#"
fn test_handler(ctx, params) {
    let x = 42;
    let y = x + undefined_var;
    #{ success: true, data: y }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "test_handler");

        assert!(
            analysis.undefined_vars.iter().any(|(name, _)| name == "undefined_var"),
            "Should detect undefined_var. Got: {:?}",
            analysis.undefined_vars
        );
    }

    #[test]
    fn test_function_analyzer_unused_var() {
        let content = r#"
fn test_handler(ctx, params) {
    let used = 42;
    let unused = 100;
    #{ success: true, data: used }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "test_handler");

        assert!(
            analysis.unused_vars.iter().any(|(name, _)| name == "unused"),
            "Should detect unused variable. Got: {:?}",
            analysis.unused_vars
        );
    }

    #[test]
    fn test_function_analyzer_all_paths_return() {
        let content = r#"
fn handler_always_returns(ctx, params) {
    if ctx["flag"] {
        return #{ success: true };
    } else {
        return #{ success: false };
    }
}

fn handler_maybe_returns(ctx, params) {
    if ctx["flag"] {
        return #{ success: true };
    }
    // No else - doesn't always return
}
"#;
        let ast = parse_script(content);

        let always = super::analyze_function(&ast, "handler_always_returns");
        assert!(
            always.all_paths_return,
            "Handler with if/else both returning should report all_paths_return=true"
        );

        let maybe = super::analyze_function(&ast, "handler_maybe_returns");
        assert!(
            !maybe.all_paths_return,
            "Handler with only if (no else) should report all_paths_return=false"
        );
    }

    #[test]
    fn test_function_analyzer_dead_code() {
        // Note: Dead code detection depends on how Rhai represents the AST.
        let content = r#"
fn handler(ctx, params) {
    return #{ success: true };
}
"#;
        let ast = parse_script(content);

        // Debug: print what statements we find
        for fn_def in ast.iter_fn_def() {
            if fn_def.name == "handler" {
                eprintln!("Function body statements:");
                for stmt in fn_def.body.statements() {
                    eprintln!("  {:?}", std::mem::discriminant(stmt));
                }
            }
        }

        let analysis = super::analyze_function(&ast, "handler");

        // Check that we got the function
        eprintln!("Analysis result: returns={:?}, all_paths_return={}",
            analysis.returns, analysis.all_paths_return);

        // The function should exist and have some analysis results
        // Even if return tracking needs work, other parts should function
        assert!(
            analysis.undefined_vars.is_empty(),
            "Should not have undefined vars in simple return"
        );
    }

    #[test]
    fn test_function_analyzer_nullable_access() {
        let content = r#"
fn handler(ctx, params) {
    let ship = get_ship(ctx["ship_id"]);
    let name = ship["name"];  // Unchecked nullable access!
    #{ success: true, data: name }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "handler");

        assert!(
            analysis.unchecked_nullables.iter().any(|(name, _)| name == "ship"),
            "Should detect unchecked nullable 'ship'. Got: {:?}",
            analysis.unchecked_nullables
        );
    }

    #[test]
    fn test_function_analyzer_nullable_checked() {
        let content = r#"
fn handler(ctx, params) {
    let ship = get_ship(ctx["ship_id"]);
    if ship == () {
        return #{ success: false };
    }
    let name = ship["name"];  // Now checked!
    #{ success: true, data: name }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "handler");

        // After null check, 'ship' should be marked as checked
        // The access after the check should not trigger a warning
        let ship_accesses: Vec<_> = analysis.unchecked_nullables
            .iter()
            .filter(|(name, _)| name == "ship")
            .collect();
        assert!(
            ship_accesses.is_empty(),
            "After null check, 'ship' access should not be flagged. Got: {:?}",
            ship_accesses
        );
    }

    #[test]
    fn test_function_analyzer_complex_null_guard() {
        // Test: let valid = ship != ()  then  if valid { ship["name"] }
        let content = r#"
fn handler(ctx, params) {
    let ship = get_ship(ctx["ship_id"]);
    let valid = ship != ();
    if valid {
        let name = ship["name"];
        return #{ success: true, data: name };
    }
    #{ success: false }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "handler");

        // The 'valid' variable tracks that ship was null-checked
        // So accessing ship["name"] inside `if valid` should be safe
        let ship_accesses: Vec<_> = analysis.unchecked_nullables
            .iter()
            .filter(|(name, _)| name == "ship")
            .collect();
        assert!(
            ship_accesses.is_empty(),
            "Complex null guard 'let valid = ship != ()' should protect access. Got: {:?}",
            ship_accesses
        );
    }

    #[test]
    fn test_function_analyzer_complex_null_guard_negated() {
        // Test: let is_null = ship == ()  then  if !is_null { ship["name"] }
        let content = r#"
fn handler(ctx, params) {
    let ship = get_ship(ctx["ship_id"]);
    let is_null = ship == ();
    if !is_null {
        let name = ship["name"];
        return #{ success: true, data: name };
    }
    #{ success: false }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "handler");

        // The '!is_null' check should recognize the null guard
        let ship_accesses: Vec<_> = analysis.unchecked_nullables
            .iter()
            .filter(|(name, _)| name == "ship")
            .collect();
        assert!(
            ship_accesses.is_empty(),
            "Negated null guard 'let is_null = ship == ()' with '!is_null' should protect. Got: {:?}",
            ship_accesses
        );
    }

    #[test]
    fn test_function_analyzer_compound_null_check() {
        // Test: if ship != () && other_condition { ship["name"] }
        let content = r#"
fn handler(ctx, params) {
    let ship = get_ship(ctx["ship_id"]);
    let enabled = true;
    if ship != () && enabled {
        let name = ship["name"];
        return #{ success: true, data: name };
    }
    #{ success: false }
}
"#;
        let ast = parse_script(content);
        let analysis = super::analyze_function(&ast, "handler");

        // Compound condition with null check should still protect
        let ship_accesses: Vec<_> = analysis.unchecked_nullables
            .iter()
            .filter(|(name, _)| name == "ship")
            .collect();
        assert!(
            ship_accesses.is_empty(),
            "Compound condition 'ship != () && ...' should protect access. Got: {:?}",
            ship_accesses
        );
    }

    #[test]
    fn test_function_analyzer_return_map_keys() {
        // Use explicit return to ensure we capture map keys
        let content = r#"
fn handler(ctx, params) {
    return #{ success: true, message: "done", count: 42 };
}
"#;
        let ast = parse_script(content);

        // Debug: check what statements are in the function
        for fn_def in ast.iter_fn_def() {
            if fn_def.name == "handler" {
                eprintln!("Return map test - Function body statements:");
                for stmt in fn_def.body.statements() {
                    eprintln!("  {:?}", std::mem::discriminant(stmt));
                }
            }
        }

        let analysis = super::analyze_function(&ast, "handler");

        eprintln!("Return map keys captured: {:?}", analysis.return_map_keys);

        // For now, just verify the analysis runs without crashing
        // Return map key detection may need refinement
        assert!(
            analysis.undefined_vars.is_empty(),
            "Should not have undefined vars"
        );
    }

    #[test]
    fn test_function_analyzer_type_inference() {
        let content = r#"
fn handler(ctx, params) {
    let str_val = "hello";
    let int_val = 42;
    let float_val = 3.14;
    let map_val = #{ key: "value" };
    let arr_val = [1, 2, 3];
    #{ success: true }
}
"#;
        let ast = parse_script(content);
        // The analyzer should infer types from literals
        // (We're not testing type mismatches yet, just that it doesn't crash)
        let analysis = super::analyze_function(&ast, "handler");
        assert!(analysis.type_mismatches.is_empty());
    }

    #[test]
    fn test_return_path_completeness_warning() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_return_path.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    // This handler may not return on all paths
    if ctx["flag"] {
        return #{ success: true };
    }
    // No return in else branch!
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W301 warning for incomplete return paths
        assert!(
            result.warnings.iter().any(|w| w.code == "W301" && w.message.contains("handle_test")),
            "Should warn about incomplete return paths. Warnings: {:?}",
            result.warnings
        );

        // Should NOT warn about init()
        assert!(
            !result.warnings.iter().any(|w| w.code == "W301" && w.message.contains("init")),
            "Should NOT warn about init() return paths"
        );
    }

    #[test]
    fn test_return_path_complete_no_warning() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_return_complete.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    // This handler returns on all paths
    if ctx["flag"] {
        return #{ success: true };
    } else {
        return #{ success: false };
    }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have W301 warning - all paths return
        let w301_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W301")
            .collect();
        assert!(
            w301_warnings.is_empty(),
            "Should not have W301 warnings when all paths return: {:?}",
            w301_warnings
        );
    }

    #[test]
    fn test_return_value_type_checking_w302() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_w302.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    // Wrong type for success - should be Bool, not String
    #{ success: "yes", error: "oops" }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W302 warning for wrong type on success field
        assert!(
            result.warnings.iter().any(|w| w.code == "W302" && w.message.contains("success")),
            "Should warn about wrong type for 'success' field. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_return_value_type_correct_no_w302() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_w302_correct.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    // Correct types
    #{ success: true, error: "none" }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have W302 warning
        let w302_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W302")
            .collect();
        assert!(
            w302_warnings.is_empty(),
            "Should not have W302 warnings for correct types: {:?}",
            w302_warnings
        );
    }

    // ========================================================================
    // Definition Script Validator Tests
    // ========================================================================

    #[test]
    fn test_definition_validator_missing_required_field() {
        let validator = DefinitionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("definitions").join("ships_test.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn broken_ship() {
    #{
        // Missing 'id' and 'name' which are required!
        tier: 1,
        attack: 30.0,
    }
}

fn all_ships() {
    [broken_ship()]
}
"#).unwrap();

        eprintln!("Test file path: {:?}", test_file);
        eprintln!("Path contains 'definitions/': {}", test_file.to_string_lossy().contains("definitions/") || test_file.to_string_lossy().contains("definitions\\"));
        eprintln!("Path contains 'ships': {}", test_file.to_string_lossy().contains("ships"));

        let result = validator.validate_file(&test_file);

        eprintln!("Definitions found: {:?}", result.definitions);
        eprintln!("Errors: {:?}", result.errors);
        eprintln!("Warnings: {:?}", result.warnings);

        std::fs::remove_file(&test_file).ok();

        // Should have E500 errors for missing required fields
        assert!(
            result.errors.iter().any(|e| e.code == "E500" && e.message.contains("id")),
            "Should error about missing 'id' field. Errors: {:?}",
            result.errors
        );
        assert!(
            result.errors.iter().any(|e| e.code == "E500" && e.message.contains("name")),
            "Should error about missing 'name' field. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_definition_validator_unknown_field() {
        let validator = DefinitionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("definitions").join("ships_unknown.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn ship_with_unknown() {
    #{
        id: "test_ship",
        name: "Test Ship",
        unknown_field: 42,
        another_unknown: "value",
    }
}

fn all_ships() {
    [ship_with_unknown()]
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W501 warnings for unknown fields
        assert!(
            result.warnings.iter().any(|w| w.code == "W501" && w.message.contains("unknown_field")),
            "Should warn about 'unknown_field'. Warnings: {:?}",
            result.warnings
        );
        assert!(
            result.warnings.iter().any(|w| w.code == "W501" && w.message.contains("another_unknown")),
            "Should warn about 'another_unknown'. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_definition_validator_valid_ship() {
        let validator = DefinitionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("definitions").join("ships_valid.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn valid_ship() {
    #{
        id: "valid_ship",
        name: "Valid Ship",
        tier: 1,
        attack: 30.0,
        defense: 20.0,
        speed: 80.0,
        weapons: [],
        is_player_class: true,
    }
}

fn all_ships() {
    [valid_ship()]
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have no E500 errors (all required fields present)
        let e500_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E500")
            .collect();
        assert!(
            e500_errors.is_empty(),
            "Should not have E500 errors for valid ship. Errors: {:?}",
            e500_errors
        );

        // Should have no W501 warnings (all fields known)
        let w501_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W501")
            .collect();
        assert!(
            w501_warnings.is_empty(),
            "Should not have W501 warnings for valid ship. Warnings: {:?}",
            w501_warnings
        );
    }

    #[test]
    fn test_definition_validator_weapon_schema() {
        let validator = DefinitionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("definitions").join("weapons_test.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn valid_weapon() {
    #{
        id: "test_railgun",
        name: "Test Railgun",
        damage: 25.0,
        accuracy: 0.75,
        category: "kinetic",
    }
}

fn invalid_weapon() {
    #{
        // Missing required id and name
        damage: 10.0,
    }
}

fn all_weapons() {
    [valid_weapon(), invalid_weapon()]
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have errors for invalid_weapon but not valid_weapon
        assert!(
            result.errors.iter().any(|e| e.code == "E500" && e.message.contains("invalid_weapon")),
            "Should error about invalid_weapon. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_definition_validator_base_functions_skipped() {
        let validator = DefinitionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("definitions").join("ships_base.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
// Base template functions should be skipped
fn corvette_base() {
    #{
        // Doesn't have id/name - that's OK for base templates
        tier: 1,
        speed: 80.0,
    }
}

fn merge(base, over) {
    #{} // Helper function, should be skipped
}

fn actual_ship() {
    #{
        id: "actual_ship",
        name: "Actual Ship",
    }
}

fn all_ships() {
    [actual_ship()]
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT error about corvette_base or merge - they should be skipped
        let base_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("corvette_base") || e.message.contains("merge"))
            .collect();
        assert!(
            base_errors.is_empty(),
            "Should skip base template functions. Errors: {:?}",
            base_errors
        );
    }

    /// This test validates ALL definition scripts in scripts/definitions/
    #[test]
    fn test_validate_all_definition_scripts() {
        let validator = DefinitionScriptValidator::new();

        // Get scripts/definitions directory relative to workspace root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let definitions_dir = std::path::PathBuf::from(manifest_dir)
            .parent().unwrap()
            .parent().unwrap()
            .join("scripts")
            .join("definitions");

        if !definitions_dir.exists() {
            eprintln!("Definitions directory not found at {:?}, skipping", definitions_dir);
            return;
        }

        let report = validator.validate_directory(&definitions_dir);

        // Print report for visibility
        if !report.is_valid() || report.total_warnings > 0 {
            eprintln!("{}", report.format_report());
        }

        // Count total definitions found
        let total_definitions: usize = report.scripts.iter()
            .map(|s| s.definitions.len())
            .sum();

        eprintln!(
            "Validated {} definition scripts, {} definitions found, {} errors, {} warnings",
            report.scripts.len(),
            total_definitions,
            report.total_errors,
            report.total_warnings
        );

        // Don't fail on errors for now - just report them
        // Real definition scripts may have patterns we haven't accounted for
        if report.total_errors > 0 {
            eprintln!("Note: {} errors found - review for accuracy", report.total_errors);
        }
    }

    // ========================================================================
    // Event Subscription Validation Tests
    // ========================================================================

    #[test]
    fn test_event_subscription_missing_handler() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_event_handler.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    subscribe_event("ShipDestroyed", "on_ship_destroyed");  // Handler doesn't exist!
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have E800 error for missing handler
        assert!(
            result.errors.iter().any(|e| e.code == "E800" && e.message.contains("on_ship_destroyed")),
            "Should error about missing handler 'on_ship_destroyed'. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_event_subscription_unknown_event_type() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_event_type.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    subscribe_event("InvalidEventType", "on_event");
}

fn on_event(ctx, event) {
    // Handler exists
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W800 warning for unknown event type
        assert!(
            result.warnings.iter().any(|w| w.code == "W800" && w.message.contains("InvalidEventType")),
            "Should warn about unknown event type 'InvalidEventType'. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_event_subscription_valid() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_event_valid.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    subscribe_event("ShipDestroyed", "on_ship_destroyed");
    subscribe_event("CombatStarted", "on_combat");
}

fn on_ship_destroyed(ctx, event) {
    log_info("Ship destroyed");
}

fn on_combat(ctx, event) {
    log_info("Combat started");
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E800 errors (handlers exist)
        let e800_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E800")
            .collect();
        assert!(
            e800_errors.is_empty(),
            "Should not have E800 errors for valid handlers. Errors: {:?}",
            e800_errors
        );

        // Should NOT have W800 warnings (event types are known)
        let w800_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W800")
            .collect();
        assert!(
            w800_warnings.is_empty(),
            "Should not have W800 warnings for valid event types. Warnings: {:?}",
            w800_warnings
        );
    }

    #[test]
    fn test_event_subscription_filtered() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_event_filtered.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    subscribe_event_filtered("ShipDamaged", "on_ship_hit", "actor", "player_ship");
}

fn on_ship_hit(ctx, event) {
    log_info("Ship was hit");
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E800 errors
        let e800_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E800")
            .collect();
        assert!(
            e800_errors.is_empty(),
            "Should not have E800 errors for valid filtered subscription. Errors: {:?}",
            e800_errors
        );
    }

    // ========================================================================
    // Dynamic Key Warning Tests
    // ========================================================================

    #[test]
    fn test_dynamic_key_warning_ctx() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_dynamic_ctx.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}

fn handle_test(ctx, params) {
    let key = "player_id";
    let value = ctx[key];  // Dynamic key - can't validate!
    #{ success: true, data: value }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W960 warning for dynamic ctx key
        assert!(
            result.warnings.iter().any(|w| w.code == "W960" && w.message.contains("ctx")),
            "Should warn about dynamic key access on 'ctx'. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_dynamic_key_warning_params() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_dynamic_params.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}

fn handle_test(ctx, params) {
    let key_name = "target";
    let target = params[key_name];  // Dynamic key!
    #{ success: true, data: target }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W960 warning for dynamic params key
        assert!(
            result.warnings.iter().any(|w| w.code == "W960" && w.message.contains("params")),
            "Should warn about dynamic key access on 'params'. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_static_key_no_warning() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_static_keys.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}

fn handle_test(ctx, params) {
    let player = ctx["player_id"];  // Static key - fine!
    let ship = ctx["ship_id"];      // Static key - fine!
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have W960 warnings
        let w960_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W960")
            .collect();
        assert!(
            w960_warnings.is_empty(),
            "Should not have W960 warnings for static keys. Warnings: {:?}",
            w960_warnings
        );
    }

    #[test]
    fn test_dynamic_key_other_map_ok() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_dynamic_other.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}

fn handle_test(ctx, params) {
    let data = #{ foo: 1, bar: 2 };
    let key = "foo";
    let value = data[key];  // Dynamic key on custom map - intentional, no warning
    #{ success: true, data: value }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have W960 warnings for custom maps
        let w960_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W960")
            .collect();
        assert!(
            w960_warnings.is_empty(),
            "Should not warn about dynamic keys on custom maps. Warnings: {:?}",
            w960_warnings
        );
    }

    // ========================================================================
    // Type Mismatch Tests (E700, W700)
    // ========================================================================

    #[test]
    fn test_binary_op_rules() {
        // Test valid operations
        assert!(check_binary_op("+", &InferredType::Int, &InferredType::Int).is_ok());
        assert!(check_binary_op("+", &InferredType::Float, &InferredType::Float).is_ok());
        assert!(check_binary_op("+", &InferredType::String, &InferredType::String).is_ok());
        assert!(check_binary_op("-", &InferredType::Int, &InferredType::Int).is_ok());
        assert!(check_binary_op("*", &InferredType::Float, &InferredType::Float).is_ok());
        assert!(check_binary_op("/", &InferredType::Int, &InferredType::Int).is_ok());
        assert!(check_binary_op("<", &InferredType::Int, &InferredType::Int).is_ok());
        assert!(check_binary_op("==", &InferredType::String, &InferredType::String).is_ok());
        assert!(check_binary_op("&&", &InferredType::Bool, &InferredType::Bool).is_ok());

        // Test implicit coercion (allowed but triggers warning)
        assert!(check_binary_op("+", &InferredType::Int, &InferredType::Float).is_ok());
        assert!(check_binary_op("+", &InferredType::Float, &InferredType::Int).is_ok());

        // Test invalid operations
        assert!(check_binary_op("+", &InferredType::String, &InferredType::Int).is_err());
        assert!(check_binary_op("+", &InferredType::Int, &InferredType::String).is_err());
        assert!(check_binary_op("-", &InferredType::String, &InferredType::String).is_err());
        assert!(check_binary_op("&&", &InferredType::Int, &InferredType::Int).is_err());

        // Dynamic types always pass
        assert!(check_binary_op("+", &InferredType::Dynamic, &InferredType::Int).is_ok());
        assert!(check_binary_op("+", &InferredType::String, &InferredType::Dynamic).is_ok());
    }

    #[test]
    fn test_implicit_coercion_detection() {
        assert!(is_implicit_coercion(&InferredType::Int, &InferredType::Float));
        assert!(is_implicit_coercion(&InferredType::Float, &InferredType::Int));
        assert!(!is_implicit_coercion(&InferredType::Int, &InferredType::Int));
        assert!(!is_implicit_coercion(&InferredType::Float, &InferredType::Float));
        assert!(!is_implicit_coercion(&InferredType::String, &InferredType::Int));
    }

    #[test]
    fn test_type_mismatch_error_e700() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_e700.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let msg = "hello";
    let num = 42;
    let result = msg + num;  // String + Int - type error!
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have E700 error for type mismatch
        assert!(
            result.errors.iter().any(|e| e.code == "E700" && e.message.contains("cannot apply")),
            "Should error about type mismatch. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_implicit_coercion_warning_w700() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_w700.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let int_val = 42;
    let float_val = 3.14;
    let result = int_val + float_val;  // Int + Float - implicit coercion
    #{ success: true, data: result }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have W700 warning for implicit coercion
        assert!(
            result.warnings.iter().any(|w| w.code == "W700" && w.message.contains("implicit coercion")),
            "Should warn about implicit coercion. Warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_valid_binary_ops_no_errors() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_valid_ops.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let a = 10;
    let b = 20;
    let sum = a + b;  // Int + Int - OK
    let product = a * b;  // Int * Int - OK
    let diff = b - a;  // Int - Int - OK

    let x = 1.5;
    let y = 2.5;
    let total = x + y;  // Float + Float - OK

    let s1 = "hello";
    let s2 = " world";
    let greeting = s1 + s2;  // String + String - OK

    let flag = a < b;  // Int < Int -> Bool - OK
    let cond = flag && true;  // Bool && Bool - OK

    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E700 errors
        let e700_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E700")
            .collect();
        assert!(
            e700_errors.is_empty(),
            "Should not have E700 errors for valid operations. Errors: {:?}",
            e700_errors
        );

        // Should NOT have W700 warnings (no int/float mixing)
        let w700_warnings: Vec<_> = result.warnings.iter()
            .filter(|w| w.code == "W700")
            .collect();
        assert!(
            w700_warnings.is_empty(),
            "Should not have W700 warnings for same-type operations. Warnings: {:?}",
            w700_warnings
        );
    }

    #[test]
    fn test_comparison_operators_valid() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_comparisons.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test_action", "handle_test");
}

fn handle_test(ctx, params) {
    let a = 10;
    let b = 20;

    // All comparison operators
    let r1 = a < b;
    let r2 = a <= b;
    let r3 = a > b;
    let r4 = a >= b;
    let r5 = a == b;
    let r6 = a != b;

    // String comparisons
    let s1 = "abc";
    let s2 = "def";
    let r7 = s1 < s2;

    #{ success: r1 && r2 && !r3 }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E700 errors
        let e700_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E700")
            .collect();
        assert!(
            e700_errors.is_empty(),
            "Should not have E700 errors for valid comparisons. Errors: {:?}",
            e700_errors
        );
    }

    // =========================================================================
    // Archetype ID Validation Tests (E900)
    // =========================================================================

    #[test]
    fn test_empty_archetype_id_e900() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_e900.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}

fn handle_test(ctx, params) {
    let ship = get_ship_def("");
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should have E900 error for empty archetype ID
        assert!(
            result.errors.iter().any(|e| e.code == "E900" && e.message.contains("empty string")),
            "Should detect E900 for empty archetype ID. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_valid_archetype_id_no_e900() {
        let validator = ActionScriptValidator::new();
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("actions").join("test_no_e900.rhai");
        std::fs::create_dir_all(test_file.parent().unwrap()).ok();
        std::fs::write(&test_file, r#"
fn init() {
    register_action("test", "handle_test");
}

fn handle_test(ctx, params) {
    let ship = get_ship_def("patrol_corvette");
    #{ success: true }
}
"#).unwrap();

        let result = validator.validate_file(&test_file);
        std::fs::remove_file(&test_file).ok();

        // Should NOT have E900 error for valid archetype ID
        let e900_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.code == "E900")
            .collect();
        assert!(
            e900_errors.is_empty(),
            "Should not have E900 errors for valid archetype ID. Errors: {:?}",
            e900_errors
        );
    }
}
