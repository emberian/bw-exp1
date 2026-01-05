//! Script hooks for simulation systems.
//!
//! Provides a fallback-safe way to call Rhai scripts from simulation code.
//! Scripts are optional enhancements - if they don't exist or fail, the
//! system falls back to hardcoded Rust logic.

use std::sync::Arc;

use bw_scripting::ScriptEngine;
use rhai::Dynamic;

/// Helper for calling simulation scripts with fallback behavior.
pub struct ScriptHooks {
    engine: Arc<ScriptEngine>,
}

impl ScriptHooks {
    /// Create a new ScriptHooks instance.
    pub fn new(engine: Arc<ScriptEngine>) -> Self {
        Self { engine }
    }

    /// Try to call a script function, returning None if script doesn't exist
    /// or function isn't found.
    ///
    /// Errors during execution are logged and return None.
    pub fn try_call(&self, script: &str, function: &str, ctx: Dynamic) -> Option<Dynamic> {
        // Check if script is loaded
        if !self.engine.has_script(script) {
            tracing::debug!(script = script, "Script not loaded, using fallback");
            return None;
        }

        // Try to call the function
        match self.engine.call_function::<Dynamic>(script, function, (ctx,)) {
            Ok(result) => Some(result),
            Err(e) => {
                // Check if it's just a missing function (expected for optional hooks)
                let err_str = e.to_string();
                if err_str.contains("Function not found") || err_str.contains("not found") {
                    tracing::debug!(
                        script = script,
                        function = function,
                        "Function not found, using fallback"
                    );
                } else {
                    tracing::warn!(
                        script = script,
                        function = function,
                        error = %e,
                        "Script hook failed, using fallback"
                    );
                }
                None
            }
        }
    }

    /// Call script function, falling back to provided default on any failure.
    pub fn call_or<F, T>(&self, script: &str, function: &str, ctx: Dynamic, fallback: F) -> T
    where
        F: FnOnce() -> T,
        T: TryFrom<Dynamic>,
    {
        match self.try_call(script, function, ctx) {
            Some(result) => match T::try_from(result) {
                Ok(value) => value,
                Err(_) => {
                    tracing::warn!(
                        script = script,
                        function = function,
                        "Script returned wrong type, using fallback"
                    );
                    fallback()
                }
            },
            None => fallback(),
        }
    }

    /// Check if a specific script is loaded.
    pub fn has_script(&self, script: &str) -> bool {
        self.engine.has_script(script)
    }
}
