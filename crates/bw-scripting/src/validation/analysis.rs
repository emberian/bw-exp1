//! AST analysis for script validation.
//!
//! Provides AST walking infrastructure and function analysis with
//! scope tracking, type inference, and null safety checking.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rhai::{AST, ASTNode, Dynamic, Engine, Expr, FnCallExpr, Position, Stmt};

use super::api_functions::{get_api_function, EventSubscriptionInfo, ReturnType};
use super::builtin_functions::{get_builtin_function, get_builtin_method};
use super::schema::{get_object_schema, api_return_schema, AccessPath, AccessSegment};
use super::types::{
    check_binary_op, check_boolean_operand, check_index_op, is_binary_operator, is_implicit_coercion,
    FunctionAnalysis, InferredType, NullGuardInfo, ReturnMapField, Scope, VarState,
};

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
            if call_expr.args.len() >= 2
                && let (Some(event_type), Some(handler_fn)) = (
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
        "subscribe_events" => {
            // subscribe_events(event_types_array, handler_fn)
            // We can't easily extract array literals here, but we can get the handler
            if call_expr.args.len() >= 2
                && let Some(handler_fn) = extract_string_literal(&call_expr.args[1]) {
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
        _ => {}
    }
}

/// Extract a string literal from an expression.
pub fn extract_string_literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::StringConstant(s, _) => Some(s.to_string()),
        Expr::DynamicConstant(boxed, _) => {
            boxed.clone().try_cast::<rhai::ImmutableString>().map(|s| s.to_string())
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
        // Query functions that may return () if not found
        "query_sector" | "query_location" | "query_combat_engagement" |
        "query_squadron" | "query_mission" |
        // Context that may not exist
        "get_context_sector" |
        // Data lookups that may return () if not found
        "get_data" | "get_ship_def" | "get_upgrade_def" | "get_cargo_def" |
        "get_stance_def" | "get_ship" | "get_weapon" | "get_effect" |
        "get_ability" | "get_faction" |
        // Squadron/alliance lookups
        "get_squadron_invite" | "get_alliance_proposal"
    )
}

/// Check if a variable name is a builtin or commonly injected.
pub fn is_builtin_var(name: &str) -> bool {
    // Common builtins, context variables, and special patterns
    matches!(name, "ctx" | "params" | "this" | "true" | "false" | "_")
}

/// Infer type from a Rhai Dynamic value (for optimized constants).
pub fn infer_dynamic_type(value: &Dynamic) -> InferredType {
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

// ============================================================================
// Archetype Registry
// ============================================================================

/// Functions that look up archetypes by ID string.
pub const ARCHETYPE_LOOKUP_FNS: &[&str] = &[
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
pub fn is_archetype_lookup_fn(name: &str) -> bool {
    ARCHETYPE_LOOKUP_FNS.contains(&name)
}

/// Maps archetype lookup functions to their registry category.
pub fn archetype_category(fn_name: &str) -> Option<&'static str> {
    match fn_name {
        "get_ship_def" | "get_ship" => Some("ships"),
        "get_weapon_def" | "get_weapon" => Some("weapons"),
        "get_cargo_def" => Some("cargo"),
        "get_effect" => Some("effects"),
        "get_ability" => Some("abilities"),
        "get_faction" => Some("factions"),
        "get_upgrade_def" => Some("upgrades"),
        "get_stance_def" => Some("stances"),
        _ => None,
    }
}

/// Registry of known archetype IDs collected from definition scripts.
#[derive(Debug, Clone, Default)]
pub struct ArchetypeRegistry {
    pub ships: HashSet<String>,
    pub weapons: HashSet<String>,
    pub cargo: HashSet<String>,
    pub effects: HashSet<String>,
    pub abilities: HashSet<String>,
    pub factions: HashSet<String>,
    pub upgrades: HashSet<String>,
    pub stances: HashSet<String>,
}

impl ArchetypeRegistry {
    /// Create empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an ID exists in the specified category.
    pub fn contains(&self, category: &str, id: &str) -> bool {
        match category {
            "ships" => self.ships.contains(id),
            "weapons" => self.weapons.contains(id),
            "cargo" => self.cargo.contains(id),
            "effects" => self.effects.contains(id),
            "abilities" => self.abilities.contains(id),
            "factions" => self.factions.contains(id),
            "upgrades" => self.upgrades.contains(id),
            "stances" => self.stances.contains(id),
            _ => false,
        }
    }

    /// Add an ID to the specified category.
    pub fn insert(&mut self, category: &str, id: String) {
        match category {
            "ships" => { self.ships.insert(id); }
            "weapons" => { self.weapons.insert(id); }
            "cargo" => { self.cargo.insert(id); }
            "effects" => { self.effects.insert(id); }
            "abilities" => { self.abilities.insert(id); }
            "factions" => { self.factions.insert(id); }
            "upgrades" => { self.upgrades.insert(id); }
            "stances" => { self.stances.insert(id); }
            _ => {}
        }
    }

    /// Build registry by executing definition scripts and calling all_*() functions.
    pub fn from_definitions_dir(dir: &Path) -> Self {
        let mut registry = Self::new();
        let mut engine = Engine::new();
        engine.set_max_expr_depths(128, 128);

        // Map of filename patterns to (category, function_name)
        let script_configs: &[(&str, &str, &str)] = &[
            ("ships", "ships", "all_ships"),
            ("weapons", "weapons", "all_weapons"),
            ("cargo", "cargo", "all_cargo"),
            ("effects", "effects", "all_effects"),
            ("abilities", "abilities", "all_abilities"),
            ("factions", "factions", "all_factions"),
        ];

        for (filename, category, fn_name) in script_configs {
            let script_path = dir.join(format!("{}.rhai", filename));
            if !script_path.exists() {
                continue;
            }

            let content = match std::fs::read_to_string(&script_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let ast = match engine.compile(&content) {
                Ok(a) => a,
                Err(_) => continue,
            };

            // Call the all_*() function and extract IDs
            let result: Result<rhai::Array, _> = engine.call_fn(&mut rhai::Scope::new(), &ast, fn_name, ());
            if let Ok(array) = result {
                for item in array {
                    if let Some(map) = item.try_cast::<rhai::Map>()
                        && let Some(id_val) = map.get("id")
                            && let Ok(id) = id_val.clone().into_string() {
                                registry.insert(category, id);
                            }
                }
            }
        }

        registry
    }

    /// Build registry from definition script validation results (static analysis fallback).
    pub fn from_definition_results(results: &[super::validators::DefinitionScriptValidation]) -> Self {
        let mut registry = Self::new();

        for result in results {
            // Determine category from script path
            let category = if result.path.contains("ships") {
                "ships"
            } else if result.path.contains("weapons") {
                "weapons"
            } else if result.path.contains("cargo") {
                "cargo"
            } else if result.path.contains("effects") {
                "effects"
            } else if result.path.contains("abilities") {
                "abilities"
            } else if result.path.contains("factions") {
                "factions"
            } else {
                continue;
            };

            for def in &result.definitions {
                registry.insert(category, def.id.clone());
            }
        }

        registry
    }

    /// Total number of registered IDs across all categories.
    pub fn total_count(&self) -> usize {
        self.ships.len() + self.weapons.len() + self.cargo.len() +
        self.effects.len() + self.abilities.len() + self.factions.len() +
        self.upgrades.len() + self.stances.len()
    }
}

// ============================================================================
// Function Analyzer - Proper AST-based analysis with scope/type tracking
// ============================================================================

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
            vars: HashMap::new(),
            parent: Some(self.current_scope),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    /// Pop the current scope.
    /// Note: Unused variable checking is done in analyze() at the end,
    /// which iterates all scopes comprehensively. Don't duplicate here.
    fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
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
            schema_name: None,
        };
        self.scopes[self.current_scope].vars.insert(name.to_string(), state);
    }

    /// Define a variable with a known schema type.
    fn define_var_with_schema(
        &mut self,
        name: &str,
        inferred_type: InferredType,
        pos: Option<Position>,
        schema_name: &str,
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
            null_guard_of: None,
            schema_name: Some(schema_name.to_string()),
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
            schema_name: None,
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
        if let Some(state) = self.lookup_var(name)
            && state.inferred_type == InferredType::Nullable && !state.null_checked {
                self.result.unchecked_nullables.push((name.to_string(), pos));
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
            if is_last && !has_returned
                && let Stmt::Expr(expr) = stmt {
                    // Record as implicit return and capture map keys/fields if present
                    self.result.returns.push(Some(stmt.position()));
                    if let Some(keys) = self.extract_map_keys(expr.as_ref()) {
                        self.result.return_map_keys.push(keys);
                    }
                    if let Some(fields) = self.extract_map_fields(expr.as_ref()) {
                        self.result.return_map_fields.push(fields);
                    }
                    has_returned = true;
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
                }
                // Check if the init expression is a function call that returns a known schema type
                else if let Some(schema_name) = self.extract_schema_from_expr(init_expr) {
                    self.define_var_with_schema(&var_name, inferred_type, Some(*pos), &schema_name);
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

                // Detect IsNull guard BEFORE extract_null_checks - we need it for early-return pattern
                let is_null_guard_var = self.detect_null_guard(&flow.expr)
                    .and_then(|guard| {
                        if let NullGuardInfo::IsNull(var_name) = guard {
                            Some(var_name)
                        } else {
                            None
                        }
                    });

                self.extract_null_checks(&flow.expr);

                // Analyze if branch (body)
                self.push_scope();
                let if_returns = self.analyze_statements(flow.body.statements().iter());
                self.pop_scope();

                // Early-return pattern: `if x == () { return ... }`
                // If the if-body always returns AND the condition was `IsNull(var)`,
                // then code AFTER this if-block can safely assume var is non-null
                // because the only way to reach it is if the condition was false (var != ())
                if if_returns {
                    if let Some(var_name) = is_null_guard_var {
                        self.mark_null_checked(&var_name);
                    }
                }

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
                let (scrutinee, cases) = boxed.as_ref();
                self.analyze_expr(scrutinee);

                // Analyze all case expressions (both conditions and body expressions)
                for case_expr in &cases.expressions {
                    self.analyze_expr(&case_expr.lhs);  // condition
                    self.analyze_expr(&case_expr.rhs);  // body/result expression
                }

                // Switch statements with a default case that returns could be considered
                // as returning, but for simplicity we don't assume they return
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
                // Note: Map key/field extraction for implicit returns is handled in
                // analyze_statements() which checks if this is actually the last statement.
                // Don't extract here to avoid duplicates.
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
                // x["key"] or nested x["a"]["b"] - check for nullable access and validate path

                // E501: Check for primitive literal being indexed (e.g., 123["field"], "str"["key"])
                let primitive_type = match &boxed.lhs {
                    Expr::IntegerConstant(_, _) => Some("integer"),
                    Expr::FloatConstant(_, _) => Some("float"),
                    Expr::StringConstant(_, _) => Some("string"),
                    Expr::CharConstant(_, _) => Some("char"),
                    Expr::BoolConstant(_, _) => Some("boolean"),
                    Expr::Unit(_) => Some("unit"),
                    _ => None,
                };
                if let Some(type_name) = primitive_type {
                    self.result.invalid_nested_accesses.push((
                        format!("Cannot index {} literal - only maps and arrays support indexing", type_name),
                        Some(*pos),
                    ));
                }

                // E701: Check index type matches container requirements
                // This catches bugs like: arr[floor(rand() * len)] where floor() returns Float
                let container_type = self.infer_expr_type(&boxed.lhs);
                let index_type = self.infer_expr_type(&boxed.rhs);
                if let Err(msg) = check_index_op(&container_type, &index_type) {
                    self.result.index_type_errors.push((msg, Some(*pos)));
                }

                if let Expr::Variable(var_box, _, _) = &boxed.lhs {
                    let var_name = var_box.1.to_string();
                    self.check_nullable_access(&var_name, Some(*pos));
                    self.use_var(&var_name, Some(*pos));
                }

                // Try to extract and validate the full access path (E501/W502)
                if let Some((root_var, segments, path_pos)) = self.extract_access_path(expr) {
                    // Record the access path
                    self.result.nested_accesses.push(AccessPath {
                        root: root_var.clone(),
                        segments: segments.clone(),
                        position: path_pos,
                    });

                    // Check for dynamic segments (W502)
                    let has_dynamic = segments.iter().any(|s| matches!(s, AccessSegment::Dynamic));
                    if has_dynamic {
                        let path_desc = self.format_access_path(&root_var, &segments);
                        self.result.dynamic_nested_accesses.push((
                            format!("Dynamic key in access path '{}' cannot be validated", path_desc),
                            path_pos,
                        ));
                    } else {
                        // Validate static path against schema (E501)
                        if let Some(error_msg) = self.validate_access_path(&root_var, &segments) {
                            self.result.invalid_nested_accesses.push((error_msg, path_pos));
                        }
                    }
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

            Expr::And(boxed, pos) | Expr::Or(boxed, pos) => {
                // And/Or contain a vector of expressions
                // E702: Check that all operands are Bool
                let op_name = if matches!(expr, Expr::And(_, _)) { "&&" } else { "||" };
                for sub_expr in boxed.iter() {
                    self.analyze_expr(sub_expr);
                    let operand_type = self.infer_expr_type(sub_expr);
                    if let Err(msg) = check_boolean_operand(op_name, &operand_type) {
                        self.result.boolean_op_errors.push((msg, Some(*pos)));
                    }
                }
            }

            // Statement block as expression (e.g., `let x = switch { ... }`)
            Expr::Stmt(stmt_block) => {
                // Analyze all statements in the block
                self.analyze_statements(stmt_block.statements().iter());
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

        // Check logical NOT operator (E702)
        if fn_name == "!" && call_expr.args.len() == 1 {
            let operand_type = self.infer_expr_type(&call_expr.args[0]);
            if let Err(msg) = check_boolean_operand("!", &operand_type) {
                self.result.boolean_op_errors.push((msg, None));
            }
        }

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
        if is_archetype_lookup_fn(fn_name) && !call_expr.args.is_empty()
            && let Some(id) = extract_string_literal(&call_expr.args[0]) {
                if id.is_empty() {
                    // E900: Empty string is definitely invalid
                    self.result.invalid_archetype_ids.push((
                        format!("{}() called with empty string", fn_name),
                        None,
                    ));
                } else {
                    // Record lookup for W900 validation against registry
                    self.result.archetype_lookups.push((
                        fn_name.to_string(),
                        id,
                        None,
                    ));
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
                self.infer_fn_call_type(call_expr)
            }

            // Method call: x.method() - infer receiver and look up method
            Expr::Dot(boxed, _, _) => {
                if let Expr::FnCall(call_expr, _) = &boxed.rhs {
                    let fn_name = call_expr.name.as_str();
                    let receiver_type = self.infer_expr_type(&boxed.lhs);

                    // Look up as built-in method
                    if let Some(builtin) = get_builtin_method(fn_name, &receiver_type) {
                        return builtin.returns.clone();
                    }
                }
                // If not a method call or unknown method, check if it's property access
                InferredType::Dynamic
            }

            // Index operation: x[key] - result depends on container type
            Expr::Index(boxed, _, _) => {
                let container_type = self.infer_expr_type(&boxed.lhs);
                let index_type = self.infer_expr_type(&boxed.rhs);

                // Use index rules to determine result type
                check_index_op(&container_type, &index_type)
                    .unwrap_or(InferredType::Dynamic)
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

    /// Infer the return type of a function call.
    fn infer_fn_call_type(&self, call_expr: &FnCallExpr) -> InferredType {
        let fn_name = call_expr.name.as_str();

        // Check if it's a binary operator first
        if is_binary_operator(fn_name) && call_expr.args.len() == 2 {
            let left_type = self.infer_expr_type(&call_expr.args[0]);
            let right_type = self.infer_expr_type(&call_expr.args[1]);
            // Try to find result type, default to Dynamic on error
            return check_binary_op(fn_name, &left_type, &right_type)
                .unwrap_or(InferredType::Dynamic);
        }

        // Check if it's a method call (has receiver as first arg)
        // In Rhai, method calls like x.floor() become FnCall("floor", [x])
        if !call_expr.args.is_empty() {
            let receiver_type = self.infer_expr_type(&call_expr.args[0]);
            if let Some(builtin) = get_builtin_method(fn_name, &receiver_type) {
                return builtin.returns.clone();
            }
        }

        // Check built-in standalone functions
        if let Some(builtin) = get_builtin_function(fn_name) {
            return builtin.returns.clone();
        }

        // Check API function return types
        if let Some(api_fn) = get_api_function(fn_name) {
            return match api_fn.returns {
                ReturnType::Unit => InferredType::Unit,
                ReturnType::Bool => InferredType::Bool,
                ReturnType::Map => InferredType::Map,
                ReturnType::Array => InferredType::Array,
                ReturnType::Number => InferredType::Float, // Could be Int too
                ReturnType::String => InferredType::String,
                ReturnType::Nullable => InferredType::Nullable,
                ReturnType::Dynamic => InferredType::Dynamic,
            };
        }

        if is_nullable_fn_name(fn_name) {
            InferredType::Nullable
        } else {
            InferredType::Dynamic
        }
    }

    /// Extract schema name from an expression if it's a function call that returns a known type.
    fn extract_schema_from_expr(&self, expr: &Expr) -> Option<String> {
        if let Expr::FnCall(call_expr, _) = expr {
            let fn_name = call_expr.name.as_str();
            return api_return_schema(fn_name).map(|s| s.to_string());
        }
        None
    }

    /// Extract a nested access path from an expression.
    /// Returns (root_var_name, segments, position) if it's an access chain starting from a variable.
    fn extract_access_path(&self, expr: &Expr) -> Option<(String, Vec<AccessSegment>, Option<Position>)> {
        let mut segments = Vec::new();
        let mut current = expr;
        let mut pos = None;

        // Walk up the nested Index expressions to find the root
        loop {
            match current {
                Expr::Index(boxed, _flags, p) => {
                    if pos.is_none() {
                        pos = Some(*p);
                    }
                    // Extract the index key
                    let segment = match &boxed.rhs {
                        Expr::StringConstant(s, _) => AccessSegment::Field(s.to_string()),
                        Expr::IntegerConstant(i, _) => AccessSegment::Index(*i),
                        Expr::DynamicConstant(dyn_val, _) => {
                            if let Some(s) = dyn_val.clone().try_cast::<rhai::ImmutableString>() {
                                AccessSegment::Field(s.to_string())
                            } else if let Some(i) = dyn_val.clone().try_cast::<i64>() {
                                AccessSegment::Index(i)
                            } else {
                                AccessSegment::Dynamic
                            }
                        }
                        _ => AccessSegment::Dynamic,
                    };
                    segments.push(segment);
                    current = &boxed.lhs;
                }
                Expr::Variable(var_box, _, _) => {
                    let var_name = var_box.1.to_string();
                    segments.reverse(); // Reverse since we built from inside-out
                    return Some((var_name, segments, pos));
                }
                _ => {
                    // Not a simple variable[...] chain
                    return None;
                }
            }
        }
    }

    /// Validate an access path against schema.
    /// Returns None if valid, Some(error_msg) if invalid.
    fn validate_access_path(&self, root_var: &str, segments: &[AccessSegment]) -> Option<String> {
        // Get the root variable's schema
        let var_state = self.lookup_var(root_var)?;
        let schema_name = var_state.schema_name.as_ref()?;
        let mut current_schema = get_object_schema(schema_name)?;

        let mut path_str = root_var.to_string();

        for segment in segments {
            match segment {
                AccessSegment::Field(field_name) => {
                    path_str.push_str(&format!("[\"{}\"]", field_name));

                    if let Some(field) = current_schema.get_field(field_name) {
                        // Valid field - update current schema if it's a nested object
                        match &field.field_type {
                            super::schema::NestedFieldType::Object(obj_name) => {
                                if let Some(next_schema) = get_object_schema(obj_name) {
                                    current_schema = next_schema;
                                } else {
                                    return None; // Unknown nested type, can't validate further
                                }
                            }
                            super::schema::NestedFieldType::Array(elem_name) => {
                                // After accessing an array field, we need an index next
                                // For now, continue validation if next segment is an index
                                if let Some(next_schema) = get_object_schema(elem_name) {
                                    current_schema = next_schema;
                                }
                                // If elem_name is not a known schema, we can't validate further
                            }
                            _ => {
                                // Primitive type - further access would be invalid
                                // but we'll let it pass for now
                            }
                        }
                    } else {
                        // Field doesn't exist in schema
                        return Some(format!(
                            "Unknown field '{}' in {}: {} has no field '{}'",
                            field_name, path_str, current_schema.name, field_name
                        ));
                    }
                }
                AccessSegment::Index(_) => {
                    path_str.push_str("[n]");
                    // Index into array - schema already updated above
                }
                AccessSegment::Dynamic => {
                    // Can't validate dynamic access
                    return None;
                }
            }
        }

        None // Valid
    }

    /// Format an access path as a human-readable string (e.g., `ship["cargo"][0]["type"]`).
    fn format_access_path(&self, root: &str, segments: &[AccessSegment]) -> String {
        let mut result = root.to_string();
        for segment in segments {
            match segment {
                AccessSegment::Field(name) => result.push_str(&format!("[\"{}\"]", name)),
                AccessSegment::Index(i) => result.push_str(&format!("[{}]", i)),
                AccessSegment::Dynamic => result.push_str("[?]"),
            }
        }
        result
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
    /// This is called BEFORE entering the if-body scope, so we only mark variables
    /// as checked when the condition GUARANTEES the variable is non-null in the if-body.
    ///
    /// - `x != ()` (IsNotNull) -> x is safe in if-body, mark as checked
    /// - `x == ()` (IsNull) -> x is NULL in if-body, DO NOT mark (would be safe in else)
    fn extract_null_checks(&mut self, condition: &Expr) {
        // Direct null check pattern: var == () or var != ()
        if let Some(guard_info) = self.detect_null_guard(condition) {
            // Only mark as checked for IsNotNull - the if-body is the safe branch
            // For IsNull, the if-body is the NULL case, so don't mark as safe
            if let NullGuardInfo::IsNotNull(name) = guard_info {
                self.mark_null_checked(&name);
            }
            // IsNull: the variable is null in the if-body, so accessing it would be unsafe
            // We could track this for the else-body, but we don't have that infrastructure
            return;
        }

        // Check if condition is a variable that tracks a null guard
        if let Expr::Variable(var_box, _, _) = condition {
            let var_name = var_box.1.to_string();
            // Clone the guarded var name before mutating self
            let guarded_var_to_mark = self.lookup_var(&var_name)
                .and_then(|state| state.null_guard_of.as_ref())
                .and_then(|guard_info| {
                    // Only mark if the guard indicates non-null
                    // e.g., `let valid = x != (); if valid { ... }` -> x is safe
                    // but `let is_null = x == (); if is_null { ... }` -> x is NULL, unsafe
                    if let NullGuardInfo::IsNotNull(guarded_var) = guard_info {
                        Some(guarded_var.clone())
                    } else {
                        None
                    }
                });
            if let Some(name) = guarded_var_to_mark {
                self.mark_null_checked(&name);
                return;
            }
        }

        // Handle Expr::And (&&) - recurse into both sides
        if let Expr::And(boxed_exprs, _) = condition {
            for expr in boxed_exprs.iter() {
                self.extract_null_checks(expr);
            }
            return;
        }

        // Handle Expr::Or (||) - DO NOT extract null checks
        // For `a != () || b != ()`, only ONE side needs to be true,
        // so we can't guarantee either variable is non-null in the body.
        if let Expr::Or(_, _) = condition {
            return;
        }

        // Check for negation: !valid where valid is a null guard
        if let Expr::FnCall(call_expr, _) = condition {
            let fn_name = call_expr.name.as_str();
            if fn_name == "!" && call_expr.args.len() == 1
                && let Expr::Variable(var_box, _, _) = &call_expr.args[0] {
                    let var_name = var_box.1.to_string();
                    // Clone the guarded var name before mutating self
                    // Negation flips the meaning:
                    // - `!valid` where `valid = x != ()` -> x IS null, unsafe
                    // - `!is_null` where `is_null = x == ()` -> x is NOT null, safe
                    let guarded_var_to_mark = self.lookup_var(&var_name)
                        .and_then(|state| state.null_guard_of.as_ref())
                        .and_then(|guard_info| {
                            match guard_info {
                                NullGuardInfo::IsNull(guarded_var) => {
                                    // !is_null means variable is NOT null - safe to access
                                    Some(guarded_var.clone())
                                }
                                NullGuardInfo::IsNotNull(_) => {
                                    // !valid means variable IS null - NOT safe to access
                                    None
                                }
                            }
                        });
                    if let Some(name) = guarded_var_to_mark {
                        self.mark_null_checked(&name);
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
