//! Registry of Rhai built-in function signatures.
//!
//! Provides type information for all Rhai standard library functions
//! to enable comprehensive type inference during script validation.

use super::types::InferredType;

// ============================================================================
// Built-in Function Signatures
// ============================================================================

/// Specification of a built-in Rhai function.
#[derive(Debug, Clone)]
pub struct BuiltinFn {
    /// Function/method name
    pub name: &'static str,
    /// Receiver type for methods (None for standalone functions)
    pub receiver: Option<InferredType>,
    /// Parameter types
    pub params: &'static [InferredType],
    /// Return type
    pub returns: InferredType,
}

impl BuiltinFn {
    /// Create a new standalone function signature.
    pub const fn func(
        name: &'static str,
        params: &'static [InferredType],
        returns: InferredType,
    ) -> Self {
        Self {
            name,
            receiver: None,
            params,
            returns,
        }
    }

    /// Create a new method signature (called on a receiver).
    pub const fn method(
        name: &'static str,
        receiver: InferredType,
        params: &'static [InferredType],
        returns: InferredType,
    ) -> Self {
        Self {
            name,
            receiver: Some(receiver),
            params,
            returns,
        }
    }

    /// Create a method with no parameters.
    pub const fn method0(name: &'static str, receiver: InferredType, returns: InferredType) -> Self {
        Self::method(name, receiver, &[], returns)
    }

    /// Create a method with one parameter.
    pub const fn method1(
        name: &'static str,
        receiver: InferredType,
        _param: InferredType,
        returns: InferredType,
    ) -> Self {
        // We can't create static arrays in const fn, so we use a workaround
        // The _param is noted for documentation but params are checked dynamically
        Self {
            name,
            receiver: Some(receiver),
            params: &[],  // Will be handled specially in lookup
            returns,
        }
    }
}

// ============================================================================
// Built-in Function Registry
// ============================================================================

/// All Rhai built-in functions with type signatures.
///
/// This list covers the standard Rhai library functions documented in
/// the Rhai Book. Functions are grouped by category.
pub static BUILTIN_FUNCTIONS: &[BuiltinFn] = &[
    // ========================================================================
    // Numeric Functions - Float Methods
    // ========================================================================

    // Rounding (these return Float, not Int!)
    BuiltinFn::method0("floor", InferredType::Float, InferredType::Float),
    BuiltinFn::method0("ceiling", InferredType::Float, InferredType::Float),
    BuiltinFn::method0("round", InferredType::Float, InferredType::Float),
    BuiltinFn::method0("int", InferredType::Float, InferredType::Float),  // truncate to int part as float
    BuiltinFn::method0("fraction", InferredType::Float, InferredType::Float),

    // Conversion (these actually return Int!)
    BuiltinFn::method0("to_int", InferredType::Float, InferredType::Int),
    BuiltinFn::method0("to_int", InferredType::String, InferredType::Int),

    // Testing
    BuiltinFn::method0("is_nan", InferredType::Float, InferredType::Bool),
    BuiltinFn::method0("is_finite", InferredType::Float, InferredType::Bool),
    BuiltinFn::method0("is_infinite", InferredType::Float, InferredType::Bool),

    // ========================================================================
    // Numeric Functions - Int Methods
    // ========================================================================

    BuiltinFn::method0("is_odd", InferredType::Int, InferredType::Bool),
    BuiltinFn::method0("is_even", InferredType::Int, InferredType::Bool),
    BuiltinFn::method0("to_float", InferredType::Int, InferredType::Float),

    // ========================================================================
    // Math Functions - Standalone
    // ========================================================================

    // Trigonometry
    BuiltinFn::func("sin", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("cos", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("tan", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("sinh", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("cosh", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("tanh", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("asin", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("acos", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("atan", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("asinh", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("acosh", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("atanh", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("hypot", &[InferredType::Float, InferredType::Float], InferredType::Float),

    // Exponential/Logarithmic
    BuiltinFn::func("sqrt", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("exp", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("ln", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("log", &[InferredType::Float], InferredType::Float),  // log10

    // Other math
    BuiltinFn::func("abs", &[InferredType::Float], InferredType::Float),
    BuiltinFn::func("abs", &[InferredType::Int], InferredType::Int),
    BuiltinFn::func("sign", &[InferredType::Float], InferredType::Int),
    BuiltinFn::func("sign", &[InferredType::Int], InferredType::Int),
    BuiltinFn::func("min", &[InferredType::Int, InferredType::Int], InferredType::Int),
    BuiltinFn::func("min", &[InferredType::Float, InferredType::Float], InferredType::Float),
    BuiltinFn::func("max", &[InferredType::Int, InferredType::Int], InferredType::Int),
    BuiltinFn::func("max", &[InferredType::Float, InferredType::Float], InferredType::Float),

    // Parsing
    BuiltinFn::func("parse_int", &[InferredType::String], InferredType::Int),
    BuiltinFn::func("parse_float", &[InferredType::String], InferredType::Float),

    // Formatting
    BuiltinFn::method0("to_binary", InferredType::Int, InferredType::String),
    BuiltinFn::method0("to_octal", InferredType::Int, InferredType::String),
    BuiltinFn::method0("to_hex", InferredType::Int, InferredType::String),

    // Constants (actually 0-arg functions)
    BuiltinFn::func("PI", &[], InferredType::Float),
    BuiltinFn::func("E", &[], InferredType::Float),

    // ========================================================================
    // String Functions
    // ========================================================================

    // Properties
    BuiltinFn::method0("len", InferredType::String, InferredType::Int),
    BuiltinFn::method0("is_empty", InferredType::String, InferredType::Bool),

    // Case conversion
    BuiltinFn::method0("to_upper", InferredType::String, InferredType::String),
    BuiltinFn::method0("to_lower", InferredType::String, InferredType::String),

    // Trimming
    BuiltinFn::method0("trim", InferredType::String, InferredType::String),
    BuiltinFn::method0("trim_start", InferredType::String, InferredType::String),
    BuiltinFn::method0("trim_end", InferredType::String, InferredType::String),

    // Searching (these take String params but we simplify)
    BuiltinFn::method0("contains", InferredType::String, InferredType::Bool),
    BuiltinFn::method0("starts_with", InferredType::String, InferredType::Bool),
    BuiltinFn::method0("ends_with", InferredType::String, InferredType::Bool),
    BuiltinFn::method0("index_of", InferredType::String, InferredType::Int),

    // Splitting/joining
    BuiltinFn::method0("split", InferredType::String, InferredType::Array),
    BuiltinFn::method0("chars", InferredType::String, InferredType::Array),
    BuiltinFn::method0("bytes", InferredType::String, InferredType::Array),

    // Manipulation
    BuiltinFn::method0("replace", InferredType::String, InferredType::String),
    BuiltinFn::method0("sub_string", InferredType::String, InferredType::String),
    BuiltinFn::method0("crop", InferredType::String, InferredType::Unit),
    BuiltinFn::method0("reverse", InferredType::String, InferredType::String),

    // ========================================================================
    // Array Functions
    // ========================================================================

    // Properties
    BuiltinFn::method0("len", InferredType::Array, InferredType::Int),
    BuiltinFn::method0("is_empty", InferredType::Array, InferredType::Bool),

    // Stack operations
    BuiltinFn::method0("push", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("pop", InferredType::Array, InferredType::Dynamic),
    BuiltinFn::method0("shift", InferredType::Array, InferredType::Dynamic),
    BuiltinFn::method0("insert", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("append", InferredType::Array, InferredType::Unit),

    // Modification
    BuiltinFn::method0("clear", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("truncate", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("chop", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("reverse", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("sort", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("dedup", InferredType::Array, InferredType::Unit),

    // Searching
    BuiltinFn::method0("contains", InferredType::Array, InferredType::Bool),
    BuiltinFn::method0("index_of", InferredType::Array, InferredType::Int),
    BuiltinFn::method0("find", InferredType::Array, InferredType::Dynamic),

    // Iteration/transformation
    BuiltinFn::method0("filter", InferredType::Array, InferredType::Array),
    BuiltinFn::method0("map", InferredType::Array, InferredType::Array),
    BuiltinFn::method0("reduce", InferredType::Array, InferredType::Dynamic),
    BuiltinFn::method0("reduce_rev", InferredType::Array, InferredType::Dynamic),
    BuiltinFn::method0("for_each", InferredType::Array, InferredType::Unit),
    BuiltinFn::method0("all", InferredType::Array, InferredType::Bool),
    BuiltinFn::method0("some", InferredType::Array, InferredType::Bool),

    // Slicing
    BuiltinFn::method0("extract", InferredType::Array, InferredType::Array),
    BuiltinFn::method0("split", InferredType::Array, InferredType::Array),
    BuiltinFn::method0("drain", InferredType::Array, InferredType::Array),
    BuiltinFn::method0("retain", InferredType::Array, InferredType::Unit),

    // ========================================================================
    // Map/Object Functions
    // ========================================================================

    // Properties
    BuiltinFn::method0("len", InferredType::Map, InferredType::Int),
    BuiltinFn::method0("is_empty", InferredType::Map, InferredType::Bool),

    // Access
    BuiltinFn::method0("contains", InferredType::Map, InferredType::Bool),
    BuiltinFn::method0("get", InferredType::Map, InferredType::Dynamic),
    BuiltinFn::method0("keys", InferredType::Map, InferredType::Array),
    BuiltinFn::method0("values", InferredType::Map, InferredType::Array),

    // Modification
    BuiltinFn::method0("set", InferredType::Map, InferredType::Unit),
    BuiltinFn::method0("remove", InferredType::Map, InferredType::Dynamic),
    BuiltinFn::method0("clear", InferredType::Map, InferredType::Unit),
    BuiltinFn::method0("mixin", InferredType::Map, InferredType::Unit),
    BuiltinFn::method0("fill_with", InferredType::Map, InferredType::Unit),

    // ========================================================================
    // Utility Functions
    // ========================================================================

    // Type checking
    BuiltinFn::func("type_of", &[InferredType::Dynamic], InferredType::String),

    // Output
    BuiltinFn::func("print", &[InferredType::Dynamic], InferredType::Unit),
    BuiltinFn::func("debug", &[InferredType::Dynamic], InferredType::Unit),

    // Conversion
    BuiltinFn::method0("to_string", InferredType::Dynamic, InferredType::String),
    BuiltinFn::method0("to_debug", InferredType::Dynamic, InferredType::String),

    // ========================================================================
    // Random Functions (from rhai-rand or custom)
    // ========================================================================

    BuiltinFn::func("rand", &[], InferredType::Float),
    BuiltinFn::func("rand_int", &[InferredType::Int, InferredType::Int], InferredType::Int),
    BuiltinFn::func("rand_float", &[InferredType::Float, InferredType::Float], InferredType::Float),
    BuiltinFn::func("random_float", &[], InferredType::Float),
    BuiltinFn::func("random_int", &[InferredType::Int, InferredType::Int], InferredType::Int),
];

// ============================================================================
// Lookup Functions
// ============================================================================

/// Look up a built-in standalone function by name.
pub fn get_builtin_function(name: &str) -> Option<&'static BuiltinFn> {
    BUILTIN_FUNCTIONS
        .iter()
        .find(|f| f.name == name && f.receiver.is_none())
}

/// Look up a built-in method by name and receiver type.
pub fn get_builtin_method(name: &str, receiver: &InferredType) -> Option<&'static BuiltinFn> {
    BUILTIN_FUNCTIONS.iter().find(|f| {
        f.name == name
            && f.receiver
                .as_ref()
                .map_or(false, |r| is_type_compatible(r, receiver))
    })
}

/// Look up any built-in (function or method) by name.
pub fn get_builtin(name: &str) -> Option<&'static BuiltinFn> {
    BUILTIN_FUNCTIONS.iter().find(|f| f.name == name)
}

/// Check if two types are compatible for method lookup.
fn is_type_compatible(expected: &InferredType, actual: &InferredType) -> bool {
    // Exact match
    if expected == actual {
        return true;
    }

    // Dynamic matches anything
    if *actual == InferredType::Dynamic {
        return true;
    }

    // Int can match Float for some methods
    if *expected == InferredType::Float && *actual == InferredType::Int {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_function() {
        let rand = get_builtin_function("rand");
        assert!(rand.is_some());
        assert_eq!(rand.unwrap().returns, InferredType::Float);

        let sqrt = get_builtin_function("sqrt");
        assert!(sqrt.is_some());
        assert_eq!(sqrt.unwrap().returns, InferredType::Float);
    }

    #[test]
    fn test_get_builtin_method() {
        // Float method
        let floor = get_builtin_method("floor", &InferredType::Float);
        assert!(floor.is_some());
        assert_eq!(floor.unwrap().returns, InferredType::Float);

        // to_int on Float returns Int
        let to_int = get_builtin_method("to_int", &InferredType::Float);
        assert!(to_int.is_some());
        assert_eq!(to_int.unwrap().returns, InferredType::Int);

        // Array method
        let len = get_builtin_method("len", &InferredType::Array);
        assert!(len.is_some());
        assert_eq!(len.unwrap().returns, InferredType::Int);
    }

    #[test]
    fn test_floor_returns_float_not_int() {
        // This is the bug that prompted this whole system!
        let floor = get_builtin_method("floor", &InferredType::Float);
        assert!(floor.is_some());
        // floor() returns Float, NOT Int
        assert_eq!(floor.unwrap().returns, InferredType::Float);
        assert_ne!(floor.unwrap().returns, InferredType::Int);
    }
}
