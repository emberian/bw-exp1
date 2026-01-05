//! Core types for script validation.
//!
//! Defines types for type inference, variable tracking, scopes, and analysis results.

use std::collections::HashMap;
use rhai::Position;

// ============================================================================
// Type Inference
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
    pub const fn new(op: &'static str, left: InferredType, right: InferredType, result: InferredType) -> Self {
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
    // String concatenation - Rhai allows String + anything with auto-conversion
    BinaryOpRule::new("+", InferredType::String, InferredType::String, InferredType::String),
    BinaryOpRule::new("+", InferredType::String, InferredType::Int, InferredType::String),
    BinaryOpRule::new("+", InferredType::String, InferredType::Float, InferredType::String),
    BinaryOpRule::new("+", InferredType::String, InferredType::Bool, InferredType::String),
    BinaryOpRule::new("+", InferredType::Int, InferredType::String, InferredType::String),
    BinaryOpRule::new("+", InferredType::Float, InferredType::String, InferredType::String),
    BinaryOpRule::new("+", InferredType::Bool, InferredType::String, InferredType::String),

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

// ============================================================================
// Null Guard Tracking
// ============================================================================

/// Tracks null guard information for complex conditionals.
#[derive(Debug, Clone, PartialEq)]
pub enum NullGuardInfo {
    /// This boolean variable represents `target_var != ()` (is_not_null check)
    IsNotNull(String),
    /// This boolean variable represents `target_var == ()` (is_null check)
    IsNull(String),
}

// ============================================================================
// Variable State and Scopes
// ============================================================================

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
    /// Schema name for nested access validation (e.g., "Ship", "Player")
    pub schema_name: Option<String>,
}

/// A scope containing variables.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Variables in this scope
    pub vars: HashMap<String, VarState>,
    /// Parent scope index (None for root)
    pub parent: Option<usize>,
}

// ============================================================================
// Analysis Results
// ============================================================================

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
    /// Unknown archetype IDs (not in registry) - W900 (populated by validator, not analyzer)
    pub unknown_archetype_ids: Vec<(String, String, Option<Position>)>, // (fn_name, id, position)
    /// All archetype lookups found: (fn_name, id, position) - for W900 checking against registry
    pub archetype_lookups: Vec<(String, String, Option<Position>)>,
    /// Nested access paths for E501/W502 validation
    pub nested_accesses: Vec<super::schema::AccessPath>,
    /// Invalid nested field accesses - E501 (field doesn't exist in schema)
    pub invalid_nested_accesses: Vec<(String, Option<Position>)>, // (description, position)
    /// Dynamic nested accesses that can't be validated - W502
    pub dynamic_nested_accesses: Vec<(String, Option<Position>)>, // (description, position)
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
