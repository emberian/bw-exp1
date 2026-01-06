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

    /// Check if this type is numeric (Int or Float).
    pub fn is_numeric(&self) -> bool {
        matches!(self, InferredType::Int | InferredType::Float)
    }

    /// Check if this type is compatible with another type.
    ///
    /// This is used for checking if a value can be assigned or passed to a parameter.
    /// Dynamic is compatible with anything. Int and Float have implicit coercion.
    pub fn is_compatible_with(&self, other: &InferredType) -> bool {
        // Same type
        if self == other {
            return true;
        }

        // Dynamic is compatible with anything
        if *self == InferredType::Dynamic || *other == InferredType::Dynamic {
            return true;
        }

        // Nullable is compatible with Map and Unit
        if *self == InferredType::Nullable {
            return matches!(other, InferredType::Map | InferredType::Unit | InferredType::Nullable);
        }
        if *other == InferredType::Nullable {
            return matches!(self, InferredType::Map | InferredType::Unit | InferredType::Nullable);
        }

        // Int and Float have implicit coercion
        if self.is_numeric() && other.is_numeric() {
            return true;
        }

        false
    }

    /// Check if this type can be assigned to a target type.
    ///
    /// More restrictive than is_compatible_with - used for type annotation checking.
    pub fn is_assignable_to(&self, target: &InferredType) -> bool {
        // Same type
        if self == target {
            return true;
        }

        // Dynamic source can be assigned to anything (we can't verify)
        if *self == InferredType::Dynamic {
            return true;
        }

        // Anything can be assigned to Dynamic target
        if *target == InferredType::Dynamic {
            return true;
        }

        // Nullable target accepts Map or Unit
        if *target == InferredType::Nullable {
            return matches!(self, InferredType::Map | InferredType::Unit | InferredType::Nullable);
        }

        // Int can be assigned to Float (widening)
        if *self == InferredType::Int && *target == InferredType::Float {
            return true;
        }

        false
    }

    /// Get a human-readable type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            InferredType::Unit => "()",
            InferredType::Bool => "Bool",
            InferredType::Int => "Int",
            InferredType::Float => "Float",
            InferredType::String => "String",
            InferredType::Array => "Array",
            InferredType::Map => "Map",
            InferredType::Nullable => "Nullable",
            InferredType::Dynamic => "Dynamic",
        }
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
// Index Operation Type Rules
// ============================================================================

/// A rule for indexing operations: container[index] -> result
#[derive(Debug, Clone)]
pub struct IndexRule {
    /// Container type (Array, String, Map)
    pub container: InferredType,
    /// Required index type
    pub index: InferredType,
    /// Result type
    pub result: InferredType,
}

impl IndexRule {
    pub const fn new(container: InferredType, index: InferredType, result: InferredType) -> Self {
        Self { container, index, result }
    }
}

/// Type rules for indexing operations.
///
/// These catch bugs like using Float as an array index (floor() returns Float, not Int).
pub static INDEX_RULES: &[IndexRule] = &[
    // Array[Int] -> Dynamic (element type unknown)
    IndexRule::new(InferredType::Array, InferredType::Int, InferredType::Dynamic),
    // String[Int] -> String (single character as string)
    IndexRule::new(InferredType::String, InferredType::Int, InferredType::String),
    // Map[String] -> Dynamic (value type unknown)
    IndexRule::new(InferredType::Map, InferredType::String, InferredType::Dynamic),
];

/// Check an index operation for type correctness.
///
/// Returns Ok(result_type) if valid, Err(error_message) if type mismatch.
///
/// # Arguments
/// * `container` - The type being indexed
/// * `index` - The type of the index expression
///
/// # Examples
/// ```ignore
/// // This is OK: array[42] where 42 is Int
/// check_index_op(&InferredType::Array, &InferredType::Int) // Ok(Dynamic)
///
/// // This is an ERROR: array[3.14] where 3.14 is Float
/// check_index_op(&InferredType::Array, &InferredType::Float) // Err(...)
///
/// // This catches the floor() bug:
/// // let idx = floor(rand() * arr.len());  // idx is Float!
/// // arr[idx]  // ERROR: Array index must be Int, got Float
/// ```
pub fn check_index_op(container: &InferredType, index: &InferredType) -> Result<InferredType, String> {
    // Dynamic containers can be indexed with anything (we can't verify)
    if *container == InferredType::Dynamic {
        return Ok(InferredType::Dynamic);
    }

    // Nullable containers (e.g., from query_ship) - treat as potentially valid Map
    // Null safety is handled separately by W100 unchecked nullable warnings
    if *container == InferredType::Nullable {
        return Ok(InferredType::Dynamic);
    }

    // Dynamic index - can't verify, allow with warning
    if *index == InferredType::Dynamic {
        return Ok(InferredType::Dynamic);
    }

    // Find matching rule
    for rule in INDEX_RULES {
        if rule.container == *container {
            // Check if index type matches
            if rule.index == *index {
                return Ok(rule.result.clone());
            }

            // Special case: Int is compatible where Float is expected (but not vice versa)
            // Actually, this is backwards - we want to be strict about Float where Int is required

            // If the required index is Int and we got Float, that's the floor() bug
            if rule.index == InferredType::Int && *index == InferredType::Float {
                return Err(format!(
                    "{} index must be {}, got {} (hint: use .to_int() to convert)",
                    container.type_name(),
                    rule.index.type_name(),
                    index.type_name()
                ));
            }

            // Other type mismatch
            return Err(format!(
                "{} index must be {}, got {}",
                container.type_name(),
                rule.index.type_name(),
                index.type_name()
            ));
        }
    }

    // Container type can't be indexed
    Err(format!("type {} cannot be indexed", container.type_name()))
}

/// Check if a type is indexable.
pub fn is_indexable(ty: &InferredType) -> bool {
    matches!(ty, InferredType::Array | InferredType::String | InferredType::Map | InferredType::Dynamic)
}

// ============================================================================
// Boolean Operation Rules
// ============================================================================

/// Check that a boolean operator receives boolean operands.
///
/// Returns Err if the operand is not Bool (and not Dynamic).
pub fn check_boolean_operand(op: &str, operand: &InferredType) -> Result<(), String> {
    if *operand == InferredType::Bool || *operand == InferredType::Dynamic {
        return Ok(());
    }

    Err(format!(
        "operator '{}' requires Bool operand, got {}",
        op,
        operand.type_name()
    ))
}

/// Check logical NOT operator.
pub fn check_logical_not(operand: &InferredType) -> Result<InferredType, String> {
    check_boolean_operand("!", operand)?;
    Ok(InferredType::Bool)
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
    /// Index type mismatches (description, position) - E701
    pub index_type_errors: Vec<(String, Option<Position>)>,
    /// Boolean operator requires Bool (description, position) - E702
    pub boolean_op_errors: Vec<(String, Option<Position>)>,
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
