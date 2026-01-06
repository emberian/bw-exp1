//! Error Code Taxonomy for Script Validation
//!
//! This module documents all error and warning codes used by the script validators.
//! Codes follow the format `[E|W]XXX` where E = Error, W = Warning, and XXX is a 3-digit number.
//!
//! # Code Categories
//!
//! - **E0XX / W0XX**: File and Structure Errors
//! - **E1XX / W1XX**: Parameter Access Validation
//! - **E2XX / W2XX**: API Function Validation
//! - **E3XX / W3XX**: Return Path Analysis
//! - **E4XX / W4XX**: Context and Variable Access
//! - **E5XX / W5XX**: Schema and Field Validation
//! - **E6XX / W6XX**: Variable Flow Analysis
//! - **E7XX / W7XX**: Type System Errors
//! - **E8XX / W8XX**: Event System Validation
//! - **E9XX / W9XX**: Archetype and Cross-Script Validation
//!
//! # Error Codes (Script Failures)
//!
//! ## E0XX - File and Structure Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E001 | Failed to read file |
//! | E002 | Syntax error in script |
//! | E003 | Action script missing required `init()` function |
//! | E004 | Registered action references non-existent handler function |
//! | E005 | Handler function has wrong parameter count (expected 2: ctx, params) |
//!
//! ## E1XX - Parameter Access Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E100 | Unknown param key accessed for action (schema validation) |
//!
//! ## E2XX - API Function Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E200 | API function called with wrong number of arguments |
//!
//! ## E5XX - Schema and Field Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E500 | Definition script missing required field |
//! | E501 | Invalid nested field access (field doesn't exist in schema) |
//! | E502 | Invalid field access for @schema annotation |
//! | E503 | @returns_schema mismatch - function returns wrong schema type |
//!
//! ## E6XX - Variable Flow and Contract Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E600 | Use of undefined variable |
//! | E601 | @requires precondition violated at call site |
//! | E602 | Impure operation in @pure function |
//! | E603 | @modifies declares variable not in scope |
//!
//! ## E7XX - Type System Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E700 | Type mismatch in binary operation |
//! | E701 | Index type mismatch (array/string requires Int, map requires String) |
//! | E702 | Boolean operator requires Bool operand |
//! | E703 | Return type doesn't match @returns annotation |
//! | E704 | Argument type doesn't match @param annotation |
//! | E705 | @as type assertion failed - expression is incompatible type |
//!
//! ## E8XX - Event System Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E800 | Event subscription references non-existent handler function |
//! | E801 | Invalid ctx key for @event type (key not in event schema) |
//! | E802 | Unknown event type in @event annotation |
//!
//! ## E9XX - Archetype Errors
//!
//! | Code | Description |
//! |------|-------------|
//! | E900 | Invalid archetype ID (empty string) |
//! | E901 | Unknown archetype in @archetype annotation |
//! | E902 | Wrong archetype category (e.g., weapon ID used with get_ship_def) |
//!
//! # Warning Codes (Potential Issues)
//!
//! ## W0XX - Structure Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W001 | `init()` exists but no actions are registered |
//! | W002 | Handler may not return proper result (missing `success:` field) |
//! | W003 | Handler has no error paths (never returns `success: false`) |
//!
//! ## W1XX - Parameter Access Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W100 | Accessing potentially null value without check |
//! | W101 | Required param for action is not accessed |
//!
//! ## W2XX - Dead Code Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W200 | Unreachable code detected (after return statement) |
//!
//! ## W3XX - Return Path Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W301 | Handler may exit without returning a value on all code paths |
//! | W302 | Return field has unexpected type (e.g., `success` should be Bool) |
//!
//! ## W4XX - Variable Access Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W400 | Unknown context key accessed (e.g., `ctx["unknown"]`) |
//! | W401 | Variable declared but never used |
//! | W402 | Variable shadows another variable in outer scope |
//!
//! ## W5XX - Schema Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W500 | Definition script missing export function |
//! | W501 | Unknown field in definition |
//! | W502 | Dynamic nested access cannot be validated at compile time |
//! | W503 | @schema annotation could not be verified (dynamic expression) |
//!
//! ## W6XX - Contract Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W600 | @requires precondition could not be verified (dynamic values) |
//! | W601 | @pure function calls function with unknown purity |
//!
//! ## W7XX - Type System Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W700 | Implicit type coercion (mixing Int and Float) |
//! | W701 | Type annotation could not be verified (dynamic expression) |
//! | W702 | Function return type inferred as Dynamic |
//! | W703 | @nonnull on variable that wasn't null-checked |
//! | W704 | @nullable param accessed without null check |
//!
//! ## W8XX - Event System Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W800 | Unknown event type in subscription |
//! | W801 | @event ctx key access could not be verified (dynamic key) |
//!
//! ## W9XX - Archetype and Cross-Script Warnings
//!
//! | Code | Description |
//! |------|-------------|
//! | W900 | Archetype ID not found in definitions |
//! | W950 | Action is registered in multiple scripts (duplicate registration) |
//! | W960 | Dynamic key access on critical maps (ctx, params) cannot be validated |
//!
//! # Usage Examples
//!
//! ## Reading Validation Output
//!
//! ```text
//! === scripts/actions/combat.rhai ===
//!   ERROR [E004]: Action 'attack' references handler 'handle_attack()' which doesn't exist
//!   WARN  [W003] (line 15): Handler 'handle_defend()' has no error paths
//!
//! Validation: 1 errors, 1 warnings across 12 scripts
//! ```
//!
//! ## Error Categories Quick Reference
//!
//! - **E0XX**: Can't parse or run the script
//! - **E1XX-E2XX**: API misuse
//! - **E5XX-E7XX**: Type/schema/contract violations
//! - **E8XX-E9XX**: System integration issues
//!
//! - **W0XX-W3XX**: Code quality concerns
//! - **W4XX-W5XX**: Potential bugs (undefined access)
//! - **W6XX**: Contract verification limitations
//! - **W7XX-W9XX**: Best practice violations

/// Error code category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// File and structure errors (E0XX)
    FileStructure,
    /// Parameter access (E1XX, W1XX)
    ParameterAccess,
    /// API function validation (E2XX)
    ApiFunction,
    /// Return path analysis (W3XX)
    ReturnPath,
    /// Context and variable access (W4XX)
    ContextAccess,
    /// Schema and field validation (E5XX, W5XX)
    Schema,
    /// Variable flow and contracts (E6XX, W6XX)
    VariableFlowAndContracts,
    /// Type system (E7XX, W7XX)
    TypeSystem,
    /// Event system (E8XX, W8XX)
    EventSystem,
    /// Archetype and cross-script (E9XX, W9XX)
    Archetype,
}

impl ErrorCategory {
    /// Get category from error/warning code.
    pub fn from_code(code: &str) -> Option<Self> {
        let num: u16 = code.get(1..)?.parse().ok()?;
        let category = num / 100;

        Some(match category {
            0 => ErrorCategory::FileStructure,
            1 => ErrorCategory::ParameterAccess,
            2 => ErrorCategory::ApiFunction,
            3 => ErrorCategory::ReturnPath,
            4 => ErrorCategory::ContextAccess,
            5 => ErrorCategory::Schema,
            6 => ErrorCategory::VariableFlowAndContracts,
            7 => ErrorCategory::TypeSystem,
            8 => ErrorCategory::EventSystem,
            9 => ErrorCategory::Archetype,
            _ => return None,
        })
    }

    /// Get human-readable name for this category.
    pub fn name(&self) -> &'static str {
        match self {
            ErrorCategory::FileStructure => "File & Structure",
            ErrorCategory::ParameterAccess => "Parameter Access",
            ErrorCategory::ApiFunction => "API Function",
            ErrorCategory::ReturnPath => "Return Path",
            ErrorCategory::ContextAccess => "Context & Variable Access",
            ErrorCategory::Schema => "Schema & Field",
            ErrorCategory::VariableFlowAndContracts => "Variable Flow & Contracts",
            ErrorCategory::TypeSystem => "Type System",
            ErrorCategory::EventSystem => "Event System",
            ErrorCategory::Archetype => "Archetype & Cross-Script",
        }
    }
}

/// Check if a code is an error (E) or warning (W).
pub fn is_error(code: &str) -> bool {
    code.starts_with('E')
}

/// Check if a code is a warning.
pub fn is_warning(code: &str) -> bool {
    code.starts_with('W')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_from_code() {
        assert_eq!(
            ErrorCategory::from_code("E001"),
            Some(ErrorCategory::FileStructure)
        );
        assert_eq!(
            ErrorCategory::from_code("W100"),
            Some(ErrorCategory::ParameterAccess)
        );
        assert_eq!(
            ErrorCategory::from_code("E700"),
            Some(ErrorCategory::TypeSystem)
        );
        assert_eq!(
            ErrorCategory::from_code("W950"),
            Some(ErrorCategory::Archetype)
        );
    }

    #[test]
    fn test_is_error_warning() {
        assert!(is_error("E001"));
        assert!(is_error("E900"));
        assert!(!is_error("W001"));

        assert!(is_warning("W001"));
        assert!(is_warning("W950"));
        assert!(!is_warning("E001"));
    }
}
