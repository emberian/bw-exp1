//! Script Error Accumulator
//!
//! Provides script-visible error tracking for debugging and GM tools.
//!
//! # Usage from Rhai scripts
//!
//! ```rhai,ignore
//! let ship = query_ship(maybe_invalid_id);
//! if has_error() {
//!     log_error(`Failed to query ship: ${last_error()}`);
//! }
//! ```
//!
//! # Server-side usage
//!
//! ```ignore
//! // Collect errors after script execution
//! let errors = bw_scripting::errors::take_errors();
//! for error in errors {
//!     // Send to GM Editor via admin WebSocket
//!     // send_admin_message(AdminServerMessage::ScriptError { ... });
//! }
//! ```

use std::cell::RefCell;
use std::collections::VecDeque;

/// Maximum number of errors to retain in the buffer.
const MAX_ERRORS: usize = 100;

/// A script error with context for debugging.
#[derive(Debug, Clone)]
pub struct ScriptError {
    /// The script file where the error occurred.
    pub script: String,
    /// The function that generated the error.
    pub function: String,
    /// Human-readable error message.
    pub message: String,
    /// Line number in the script (0 if unknown).
    pub line: usize,
    /// Column number in the script (0 if unknown).
    pub column: usize,
    /// Server tick when the error occurred.
    pub tick: u64,
}

impl Default for ScriptError {
    fn default() -> Self {
        Self {
            script: String::new(),
            function: String::new(),
            message: String::new(),
            line: 0,
            column: 0,
            tick: 0,
        }
    }
}

impl ScriptError {
    /// Create a new script error with basic info.
    pub fn new(function: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            function: function.into(),
            message: message.into(),
            ..Default::default()
        }
    }

    /// Set the script path.
    pub fn with_script(mut self, script: impl Into<String>) -> Self {
        self.script = script.into();
        self
    }

    /// Set the line and column.
    pub fn with_position(mut self, line: usize, column: usize) -> Self {
        self.line = line;
        self.column = column;
        self
    }

    /// Set the tick.
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }
}

thread_local! {
    /// Thread-local buffer for script errors.
    /// Uses a ring buffer to cap memory usage.
    static ERROR_BUFFER: RefCell<VecDeque<ScriptError>> = RefCell::new(VecDeque::with_capacity(MAX_ERRORS));

    /// The currently executing script path (set by script runner).
    static CURRENT_SCRIPT: RefCell<String> = const { RefCell::new(String::new()) };

    /// Current server tick (set by simulation loop).
    static CURRENT_TICK: RefCell<u64> = const { RefCell::new(0) };
}

/// Push an error to the thread-local buffer.
///
/// Automatically fills in script path and tick if not provided.
pub fn push_error(mut error: ScriptError) {
    // Fill in context if missing
    if error.script.is_empty() {
        CURRENT_SCRIPT.with(|s| {
            error.script = s.borrow().clone();
        });
    }
    if error.tick == 0 {
        CURRENT_TICK.with(|t| {
            error.tick = *t.borrow();
        });
    }

    ERROR_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        if buf.len() >= MAX_ERRORS {
            buf.pop_front();
        }
        buf.push_back(error);
    });
}

/// Take all errors from the buffer, clearing it.
///
/// Call this after script execution to retrieve errors for broadcasting.
pub fn take_errors() -> Vec<ScriptError> {
    ERROR_BUFFER.with(|buf| buf.borrow_mut().drain(..).collect())
}

/// Peek at the most recent error without removing it.
pub fn last_error() -> Option<ScriptError> {
    ERROR_BUFFER.with(|buf| buf.borrow().back().cloned())
}

/// Get the most recent error message, or empty string if none.
pub fn last_error_message() -> String {
    last_error().map(|e| e.message).unwrap_or_default()
}

/// Check if there are any errors in the buffer.
pub fn has_errors() -> bool {
    ERROR_BUFFER.with(|buf| !buf.borrow().is_empty())
}

/// Clear all errors from the buffer.
pub fn clear_errors() {
    ERROR_BUFFER.with(|buf| buf.borrow_mut().clear());
}

/// Get the current error count.
pub fn error_count() -> usize {
    ERROR_BUFFER.with(|buf| buf.borrow().len())
}

/// Set the current script path for error context.
///
/// Called by the script runner before executing a script.
pub fn set_current_script(script: impl Into<String>) {
    CURRENT_SCRIPT.with(|s| {
        *s.borrow_mut() = script.into();
    });
}

/// Get the current script path.
pub fn get_current_script() -> String {
    CURRENT_SCRIPT.with(|s| s.borrow().clone())
}

/// Clear the current script path.
pub fn clear_current_script() {
    CURRENT_SCRIPT.with(|s| {
        s.borrow_mut().clear();
    });
}

/// Set the current tick for error timestamps.
///
/// Called by the simulation loop at the start of each tick.
pub fn set_current_tick(tick: u64) {
    CURRENT_TICK.with(|t| {
        *t.borrow_mut() = tick;
    });
}

/// Get the current tick.
pub fn get_current_tick() -> u64 {
    CURRENT_TICK.with(|t| *t.borrow())
}

/// Guard that sets current script on creation and clears on drop.
pub struct ScriptErrorContext {
    previous: String,
}

impl ScriptErrorContext {
    /// Create a new error context for a script.
    pub fn new(script: impl Into<String>) -> Self {
        let previous = get_current_script();
        set_current_script(script);
        Self { previous }
    }
}

impl Drop for ScriptErrorContext {
    fn drop(&mut self) {
        set_current_script(std::mem::take(&mut self.previous));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_take_errors() {
        clear_errors();

        push_error(ScriptError::new("test_fn", "Something went wrong"));
        push_error(ScriptError::new("another_fn", "Another error"));

        assert_eq!(error_count(), 2);

        let errors = take_errors();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].function, "test_fn");
        assert_eq!(errors[1].function, "another_fn");

        assert_eq!(error_count(), 0);
    }

    #[test]
    fn test_last_error() {
        clear_errors();

        assert!(last_error().is_none());

        push_error(ScriptError::new("fn1", "error1"));
        push_error(ScriptError::new("fn2", "error2"));

        let last = last_error().unwrap();
        assert_eq!(last.function, "fn2");
        assert_eq!(last.message, "error2");

        // Didn't consume the error
        assert_eq!(error_count(), 2);
    }

    #[test]
    fn test_error_context() {
        clear_errors();
        set_current_script("outer.rhai");

        {
            let _ctx = ScriptErrorContext::new("inner.rhai");
            push_error(ScriptError::new("inner_fn", "inner error"));
        }

        // Script context restored
        push_error(ScriptError::new("outer_fn", "outer error"));

        let errors = take_errors();
        assert_eq!(errors[0].script, "inner.rhai");
        assert_eq!(errors[1].script, "outer.rhai");
    }

    #[test]
    fn test_max_errors() {
        clear_errors();

        for i in 0..150 {
            push_error(ScriptError::new("fn", format!("error {}", i)));
        }

        assert_eq!(error_count(), MAX_ERRORS);

        let errors = take_errors();
        // Should have the last MAX_ERRORS errors
        assert_eq!(errors[0].message, "error 50");
        assert_eq!(errors[99].message, "error 149");
    }
}
