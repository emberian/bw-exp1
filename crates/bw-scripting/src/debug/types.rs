//! Core types for the script debugging system.

use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

/// A breakpoint in a script.
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique identifier for this breakpoint
    pub id: Uuid,
    /// Script path (e.g., "ai/pirate.rhai")
    pub source: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed, optional)
    pub column: Option<usize>,
    /// Condition expression (Rhai code that must evaluate to true)
    pub condition: Option<String>,
    /// Number of times this breakpoint has been hit
    pub hit_count: u64,
    /// Whether the breakpoint is enabled
    pub enabled: bool,
}

impl Breakpoint {
    /// Create a new breakpoint at a line.
    pub fn at_line(source: impl Into<String>, line: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            source: source.into(),
            line,
            column: None,
            condition: None,
            hit_count: 0,
            enabled: true,
        }
    }

    /// Create a new breakpoint with a condition.
    pub fn at_line_with_condition(
        source: impl Into<String>,
        line: usize,
        condition: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source: source.into(),
            line,
            column: None,
            condition: Some(condition.into()),
            hit_count: 0,
            enabled: true,
        }
    }

    /// Set the condition for this breakpoint.
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// Set the enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A function breakpoint.
#[derive(Debug, Clone)]
pub struct FunctionBreakpoint {
    /// Unique identifier
    pub id: Uuid,
    /// Function name to break on
    pub function_name: String,
    /// Break when entering the function
    pub break_on_entry: bool,
    /// Break when exiting the function
    pub break_on_exit: bool,
    /// Whether enabled
    pub enabled: bool,
}

impl FunctionBreakpoint {
    /// Create a new function breakpoint.
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            function_name: function_name.into(),
            break_on_entry: true,
            break_on_exit: false,
            enabled: true,
        }
    }

    /// Also break on function exit.
    pub fn with_exit(mut self) -> Self {
        self.break_on_exit = true;
        self
    }
}

/// Target for a debug session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugTarget {
    /// Debug within a specific playtest instance
    Playtest(Uuid),
    /// Debug on the live server (dangerous but powerful)
    Live,
}

/// A debug session tracks breakpoints and state for a debugging client.
pub struct DebugSession {
    /// Unique session ID
    pub id: Uuid,
    /// What we're debugging
    pub target: DebugTarget,
    /// GM player ID who owns this session
    pub owner_id: Uuid,
    /// Line breakpoints (Arc-wrapped for sharing with debugger)
    pub breakpoints: SharedBreakpoints,
    /// Function breakpoints (Arc-wrapped for sharing)
    pub function_breakpoints: SharedFunctionBreakpoints,
    /// Current paused state (if paused)
    pub paused_at: RwLock<Option<PausedState>>,
    /// Whether this session is active
    pub active: bool,
    /// Auto-timeout for paused scripts (in seconds, 0 = no timeout)
    pub pause_timeout_secs: u32,
}

impl std::fmt::Debug for DebugSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugSession")
            .field("id", &self.id)
            .field("target", &self.target)
            .field("owner_id", &self.owner_id)
            .field("breakpoints", &self.breakpoints.read().len())
            .field("active", &self.active)
            .finish()
    }
}

impl DebugSession {
    /// Create a new debug session.
    pub fn new(owner_id: Uuid, target: DebugTarget) -> Self {
        Self {
            id: Uuid::new_v4(),
            target,
            owner_id,
            breakpoints: Arc::new(RwLock::new(Vec::new())),
            function_breakpoints: Arc::new(RwLock::new(Vec::new())),
            paused_at: RwLock::new(None),
            active: true,
            pause_timeout_secs: 30, // Default 30s timeout for safety
        }
    }

    /// Get the shared breakpoints Arc for use with debugger registration.
    pub fn shared_breakpoints(&self) -> SharedBreakpoints {
        Arc::clone(&self.breakpoints)
    }

    /// Get the shared function breakpoints Arc for use with debugger registration.
    pub fn shared_function_breakpoints(&self) -> SharedFunctionBreakpoints {
        Arc::clone(&self.function_breakpoints)
    }

    /// Add a breakpoint.
    pub fn add_breakpoint(&self, bp: Breakpoint) {
        self.breakpoints.write().push(bp);
    }

    /// Remove a breakpoint by ID.
    pub fn remove_breakpoint(&self, id: Uuid) -> bool {
        let mut bps = self.breakpoints.write();
        if let Some(idx) = bps.iter().position(|bp| bp.id == id) {
            bps.remove(idx);
            true
        } else {
            false
        }
    }

    /// Get a breakpoint by ID.
    pub fn get_breakpoint(&self, id: Uuid) -> Option<Breakpoint> {
        self.breakpoints.read().iter().find(|bp| bp.id == id).cloned()
    }

    /// Set breakpoint enabled state.
    pub fn set_breakpoint_enabled(&self, id: Uuid, enabled: bool) -> bool {
        let mut bps = self.breakpoints.write();
        if let Some(bp) = bps.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get all breakpoints for a specific script.
    pub fn breakpoints_for_script(&self, script: &str) -> Vec<Breakpoint> {
        self.breakpoints
            .read()
            .iter()
            .filter(|bp| bp.source == script && bp.enabled)
            .cloned()
            .collect()
    }

    /// Check if paused.
    pub fn is_paused(&self) -> bool {
        self.paused_at.read().is_some()
    }
}

/// State when execution is paused.
#[derive(Debug, Clone)]
pub struct PausedState {
    /// Script path where we paused
    pub script_path: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Call stack at pause point
    pub call_stack: Vec<StackFrame>,
    /// Local variables in current scope
    pub local_variables: Vec<Variable>,
    /// Entity context (if debugging a behavior script)
    pub entity_context: Option<EntityContext>,
    /// Reason for pausing
    pub reason: PauseReason,
    /// Timestamp when paused (for timeout tracking)
    pub paused_at_ms: u64,
}

/// A frame in the call stack.
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Frame index (0 = top of stack)
    pub index: usize,
    /// Function name (or "<script>" for top-level)
    pub function_name: String,
    /// Source file (if known)
    pub source: Option<String>,
    /// Line number (if known)
    pub line: Option<usize>,
    /// Column number (if known)
    pub column: Option<usize>,
}

/// A variable for inspection.
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Formatted value representation
    pub value: String,
    /// Type name (e.g., "i64", "Map", "Array")
    pub type_name: String,
    /// Whether this can be expanded (maps, arrays)
    pub expandable: bool,
    /// Path for expansion requests (e.g., "local_data.enemies[0]")
    pub path: String,
    /// Child variables (for expanded items)
    pub children: Option<Vec<Variable>>,
}

impl Variable {
    /// Create a simple variable.
    pub fn simple(name: impl Into<String>, value: impl Into<String>, type_name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            path: name.clone(),
            name,
            value: value.into(),
            type_name: type_name.into(),
            expandable: false,
            children: None,
        }
    }

    /// Create an expandable variable.
    pub fn expandable(
        name: impl Into<String>,
        value: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        let name = name.into();
        Self {
            path: name.clone(),
            name,
            value: value.into(),
            type_name: type_name.into(),
            expandable: true,
            children: None,
        }
    }

    /// Set the path for this variable.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }
}

/// Context about the entity being debugged.
#[derive(Debug, Clone)]
pub struct EntityContext {
    /// Entity type (e.g., "ship", "player")
    pub entity_type: String,
    /// Entity UUID
    pub entity_id: Uuid,
    /// Display name
    pub entity_name: String,
    /// Sector ID (if applicable)
    pub sector_id: Option<Uuid>,
}

/// Reason execution was paused.
#[derive(Debug, Clone)]
pub enum PauseReason {
    /// Hit a line breakpoint
    Breakpoint { breakpoint_id: Uuid },
    /// Hit a function breakpoint (entry)
    FunctionEntry { function_name: String },
    /// Hit a function breakpoint (exit)
    FunctionExit { function_name: String },
    /// Step completed
    Step,
    /// Manual pause requested
    Pause,
    /// Exception was thrown
    Exception { message: String },
}

/// Commands that can be sent to a paused debugger.
#[derive(Debug, Clone)]
pub enum DebugCommand {
    /// Continue normal execution
    Continue,
    /// Step into function calls
    StepInto,
    /// Step over function calls
    StepOver,
    /// Step out of current function
    StepOut,
    /// Pause execution (from Continue state)
    Pause,
    /// Evaluate an expression in current context
    Evaluate {
        expression: String,
        frame_index: usize,
    },
}

/// Step mode for the debugger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Step into all calls
    StepInto,
    /// Step over calls (stay at same depth)
    StepOver,
    /// Step out of current function
    StepOut,
}

/// Result of expression evaluation.
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// The expression that was evaluated
    pub expression: String,
    /// Result value (formatted)
    pub result: String,
    /// Type of the result
    pub type_name: String,
    /// Whether evaluation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Shared breakpoint state that can be accessed from the debugger callback.
pub type SharedBreakpoints = Arc<RwLock<Vec<Breakpoint>>>;

/// Shared function breakpoint state.
pub type SharedFunctionBreakpoints = Arc<RwLock<Vec<FunctionBreakpoint>>>;
