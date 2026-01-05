//! Debug panel state for the GM Editor
//!
//! Manages debug session state, breakpoints, and execution control.

use leptos::prelude::*;
use uuid::Uuid;

use bw_shared::dto::{
    BreakpointDto, FunctionBreakpointDto, StackFrameDto, VariableDto,
    EntityContextDto, PauseReasonDto, DebugTargetDto,
};

/// Debug panel state
#[derive(Clone, Copy)]
pub struct DebugPanelState {
    /// Current debug session ID (None if not debugging)
    pub session_id: RwSignal<Option<Uuid>>,
    /// Debug target (playtest or live)
    pub target: RwSignal<Option<DebugTargetDto>>,
    /// Whether we're starting a session
    pub connecting: RwSignal<bool>,

    /// Line breakpoints
    pub breakpoints: RwSignal<Vec<BreakpointDto>>,
    /// Function breakpoints
    pub function_breakpoints: RwSignal<Vec<FunctionBreakpointDto>>,

    /// Whether script is currently paused
    pub is_paused: RwSignal<bool>,
    /// Current paused script path
    pub paused_script: RwSignal<Option<String>>,
    /// Current paused line
    pub paused_line: RwSignal<Option<usize>>,
    /// Current paused column
    pub paused_column: RwSignal<Option<usize>>,
    /// Pause reason
    pub pause_reason: RwSignal<Option<PauseReasonDto>>,

    /// Call stack frames
    pub call_stack: RwSignal<Vec<StackFrameDto>>,
    /// Selected stack frame index
    pub selected_frame: RwSignal<usize>,

    /// Variables at current scope
    pub variables: RwSignal<Vec<VariableDto>>,

    /// Entity context (what entity the script is running on)
    pub entity_context: RwSignal<Option<EntityContextDto>>,

    /// Last debug error message
    pub error: RwSignal<Option<String>>,
}

impl DebugPanelState {
    pub fn new() -> Self {
        Self {
            session_id: RwSignal::new(None),
            target: RwSignal::new(None),
            connecting: RwSignal::new(false),

            breakpoints: RwSignal::new(vec![]),
            function_breakpoints: RwSignal::new(vec![]),

            is_paused: RwSignal::new(false),
            paused_script: RwSignal::new(None),
            paused_line: RwSignal::new(None),
            paused_column: RwSignal::new(None),
            pause_reason: RwSignal::new(None),

            call_stack: RwSignal::new(vec![]),
            selected_frame: RwSignal::new(0),

            variables: RwSignal::new(vec![]),

            entity_context: RwSignal::new(None),

            error: RwSignal::new(None),
        }
    }

    /// Check if a debug session is active
    pub fn is_active(&self) -> bool {
        self.session_id.get().is_some()
    }

    /// Handle session started event
    pub fn on_session_started(&self, session_id: Uuid) {
        self.session_id.set(Some(session_id));
        self.connecting.set(false);
        self.error.set(None);
    }

    /// Handle session ended event
    pub fn on_session_ended(&self) {
        self.session_id.set(None);
        self.target.set(None);
        self.is_paused.set(false);
        self.paused_script.set(None);
        self.paused_line.set(None);
        self.paused_column.set(None);
        self.pause_reason.set(None);
        self.call_stack.set(vec![]);
        self.variables.set(vec![]);
        self.entity_context.set(None);
    }

    /// Handle breakpoint set event
    pub fn on_breakpoint_set(&self, bp: BreakpointDto) {
        self.breakpoints.update(|bps| {
            // Replace if exists, otherwise add
            if let Some(pos) = bps.iter().position(|b| b.id == bp.id) {
                bps[pos] = bp;
            } else {
                bps.push(bp);
            }
        });
    }

    /// Handle breakpoint removed event
    pub fn on_breakpoint_removed(&self, breakpoint_id: Uuid) {
        self.breakpoints.update(|bps| {
            bps.retain(|bp| bp.id != breakpoint_id);
        });
    }

    /// Handle breakpoint toggled event
    pub fn on_breakpoint_toggled(&self, breakpoint_id: Uuid, enabled: bool) {
        self.breakpoints.update(|bps| {
            if let Some(bp) = bps.iter_mut().find(|b| b.id == breakpoint_id) {
                bp.enabled = enabled;
            }
        });
    }

    /// Handle breakpoint list event
    pub fn on_breakpoint_list(&self, breakpoints: Vec<BreakpointDto>, function_breakpoints: Vec<FunctionBreakpointDto>) {
        self.breakpoints.set(breakpoints);
        self.function_breakpoints.set(function_breakpoints);
    }

    /// Handle paused event
    pub fn on_paused(
        &self,
        script: String,
        line: usize,
        column: usize,
        reason: PauseReasonDto,
        call_stack: Vec<StackFrameDto>,
        entity_context: Option<EntityContextDto>,
    ) {
        self.is_paused.set(true);
        self.paused_script.set(Some(script));
        self.paused_line.set(Some(line));
        self.paused_column.set(Some(column));
        self.pause_reason.set(Some(reason));
        self.call_stack.set(call_stack);
        self.selected_frame.set(0);
        self.entity_context.set(entity_context);
        // Variables will be requested separately
    }

    /// Handle resumed event
    pub fn on_resumed(&self) {
        self.is_paused.set(false);
        self.paused_script.set(None);
        self.paused_line.set(None);
        self.paused_column.set(None);
        self.pause_reason.set(None);
        // Keep call stack for reference
    }

    /// Handle variables received event
    pub fn on_variables(&self, _frame_index: usize, variables: Vec<VariableDto>) {
        self.variables.set(variables);
    }

    /// Handle call stack received event
    pub fn on_call_stack(&self, frames: Vec<StackFrameDto>) {
        self.call_stack.set(frames);
    }

    /// Handle debug error
    pub fn on_error(&self, message: String) {
        self.error.set(Some(message));
        self.connecting.set(false);

        // Auto-clear error after delay
        let error = self.error;
        gloo_timers::callback::Timeout::new(5000, move || {
            error.set(None);
        })
        .forget();
    }

    /// Get breakpoints for a specific script
    pub fn breakpoints_for_script(&self, script: &str) -> Vec<BreakpointDto> {
        self.breakpoints.get()
            .into_iter()
            .filter(|bp| bp.script == script)
            .collect()
    }

    /// Check if there's a breakpoint at a specific line
    pub fn has_breakpoint(&self, script: &str, line: usize) -> Option<BreakpointDto> {
        self.breakpoints.get()
            .into_iter()
            .find(|bp| bp.script == script && bp.line == line)
    }

    /// Select a stack frame
    pub fn select_frame(&self, index: usize) {
        self.selected_frame.set(index);
        // Trigger variable request for this frame
    }
}

impl Default for DebugPanelState {
    fn default() -> Self {
        Self::new()
    }
}
