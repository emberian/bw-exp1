//! Rhai Debugger Integration
//!
//! Integrates with Rhai's built-in debugger API to provide:
//! - Breakpoint handling
//! - Step-through execution
//! - Variable inspection
//! - Call stack collection

use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Engine, Dynamic, Map, Scope};
use rhai::debugger::{BreakPoint, Debugger, DebuggerCommand, DebuggerEvent};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use super::types::*;

/// State maintained by the debugger callback.
/// This is stored in Rhai's debugger state storage.
pub struct DebuggerState {
    /// Session ID
    pub session_id: Uuid,
    /// Shared breakpoints (can be updated externally)
    pub breakpoints: SharedBreakpoints,
    /// Channel to send pause notifications
    pub pause_tx: broadcast::Sender<PausedState>,
    /// Channel to receive commands (wrapped in sync Mutex for callback use)
    pub command_rx: Arc<std::sync::Mutex<mpsc::Receiver<DebugCommand>>>,
    /// Current step mode
    pub step_mode: Option<StepMode>,
    /// Call depth when step was initiated (for step-over/out)
    pub step_depth: usize,
    /// Entity context (set before script execution)
    pub entity_context: Option<EntityContext>,
    /// Script path being executed
    pub script_path: String,
}

/// Register the debugger with a Rhai engine.
///
/// This configures the engine to support debugging with the given session's
/// breakpoints and channels.
pub fn register_debugger(
    engine: &mut Engine,
    session_id: Uuid,
    breakpoints: SharedBreakpoints,
    pause_tx: broadcast::Sender<PausedState>,
    command_rx: Arc<std::sync::Mutex<mpsc::Receiver<DebugCommand>>>,
    script_path: String,
    entity_context: Option<EntityContext>,
) {
    let init_breakpoints = breakpoints.clone();
    let init_session_id = session_id;
    let init_pause_tx = pause_tx.clone();
    let init_command_rx = command_rx.clone();
    let init_script_path = script_path.clone();
    let init_entity_context = entity_context.clone();

    engine.register_debugger(
        // Init callback - called once when debugging starts
        move |_engine, mut debugger| {
            // Set up debugger state
            let state = DebuggerState {
                session_id: init_session_id,
                breakpoints: init_breakpoints.clone(),
                pause_tx: init_pause_tx.clone(),
                command_rx: init_command_rx.clone(),
                step_mode: None,
                step_depth: 0,
                entity_context: init_entity_context.clone(),
                script_path: init_script_path.clone(),
            };
            debugger.set_state(state);

            // Register breakpoints with Rhai
            sync_breakpoints_to_rhai(&mut debugger, &init_breakpoints.read(), &init_script_path);

            debugger
        },
        // Step callback - called at each debugger event
        move |context, event, _node, source, pos| {
            // Get our state from the debugger
            let debugger = context.global_runtime_state_mut().debugger_mut();

            // Handle the event
            handle_debug_event(debugger, context.call_level(), event, source, pos)
        },
    );
}

/// Sync breakpoints from our format to Rhai's format.
fn sync_breakpoints_to_rhai(debugger: &mut Debugger, breakpoints: &[Breakpoint], current_script: &str) {
    // Clear existing breakpoints
    debugger.break_points_mut().clear();

    // Add enabled breakpoints for the current script
    for bp in breakpoints {
        if bp.enabled && bp.source == current_script {
            debugger.break_points_mut().push(BreakPoint::AtPosition {
                source: bp.source.clone().into(),
                pos: rhai::Position::new(bp.line as u16, bp.column.unwrap_or(1) as u16),
                enabled: true,
            });
        }
    }
}

/// Handle a debug event from Rhai.
fn handle_debug_event(
    debugger: &mut Debugger,
    call_level: usize,
    event: DebuggerEvent,
    source: Option<&str>,
    pos: rhai::Position,
) -> Result<DebuggerCommand, Box<rhai::EvalAltResult>> {
    let state = debugger.state_mut::<DebuggerState>()
        .expect("DebuggerState not set");

    match event {
        DebuggerEvent::Start => {
            // Script execution starting
            tracing::trace!(session_id = %state.session_id, "Debug: script start");
            Ok(DebuggerCommand::Continue)
        }

        DebuggerEvent::Step => {
            // Check if we should pause based on step mode
            if let Some(mode) = state.step_mode {
                let should_pause = match mode {
                    StepMode::StepInto => true,
                    StepMode::StepOver => call_level <= state.step_depth,
                    StepMode::StepOut => call_level < state.step_depth,
                };

                if should_pause {
                    state.step_mode = None;
                    return pause_and_wait(
                        state,
                        debugger,
                        source,
                        pos,
                        PauseReason::Step,
                    );
                }
            }
            Ok(DebuggerCommand::Continue)
        }

        DebuggerEvent::BreakPoint(bp_index) => {
            // A breakpoint was hit
            let bp_id = state.breakpoints.read()
                .get(bp_index)
                .map(|bp| bp.id)
                .unwrap_or(Uuid::nil());

            tracing::debug!(
                session_id = %state.session_id,
                breakpoint_id = %bp_id,
                line = ?pos.line(),
                "Breakpoint hit"
            );

            pause_and_wait(
                state,
                debugger,
                source,
                pos,
                PauseReason::Breakpoint { breakpoint_id: bp_id },
            )
        }

        DebuggerEvent::FunctionExitWithValue(value, fn_pos) => {
            // Function exit with return value
            if matches!(state.step_mode, Some(StepMode::StepOut)) && call_level < state.step_depth {
                state.step_mode = None;
                return pause_and_wait(
                    state,
                    debugger,
                    source,
                    fn_pos,
                    PauseReason::Step,
                );
            }

            // Check for function breakpoints
            // (Would need function name from call stack here)

            let _ = value; // Silence unused warning
            Ok(DebuggerCommand::Continue)
        }

        DebuggerEvent::FunctionExitWithError(err, fn_pos) => {
            // Function exit with error - pause on exceptions
            tracing::debug!(
                session_id = %state.session_id,
                error = %err,
                "Function exited with error"
            );

            pause_and_wait(
                state,
                debugger,
                source,
                fn_pos,
                PauseReason::Exception { message: err.to_string() },
            )
        }

        DebuggerEvent::End => {
            // Script execution ended
            tracing::trace!(session_id = %state.session_id, "Debug: script end");
            Ok(DebuggerCommand::Continue)
        }
    }
}

/// Pause execution and wait for a command from the GM client.
fn pause_and_wait(
    state: &mut DebuggerState,
    debugger: &Debugger,
    source: Option<&str>,
    pos: rhai::Position,
    reason: PauseReason,
) -> Result<DebuggerCommand, Box<rhai::EvalAltResult>> {
    // Collect call stack
    let call_stack = collect_call_stack(debugger);

    // Collect local variables from scope
    // Note: In the debugger callback, we don't have direct scope access
    // Variables would need to be collected differently - for now, return empty
    let local_variables = Vec::new();

    // Build paused state
    let paused_state = PausedState {
        script_path: source.unwrap_or(&state.script_path).to_string(),
        line: pos.line().unwrap_or(0) as usize,
        column: pos.position().unwrap_or(0) as usize,
        call_stack,
        local_variables,
        entity_context: state.entity_context.clone(),
        reason,
        paused_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };

    // Send pause notification
    if state.pause_tx.send(paused_state).is_err() {
        tracing::warn!(session_id = %state.session_id, "No listeners for pause event");
    }

    // Block waiting for command
    loop {
        // Try to receive with a timeout to allow checking for session termination
        let result = {
            let mut rx = state.command_rx.lock().unwrap();
            rx.blocking_recv()
        };

        match result {
            Some(DebugCommand::Continue) => {
                tracing::trace!(session_id = %state.session_id, "Debug: continue");
                return Ok(DebuggerCommand::Continue);
            }
            Some(DebugCommand::StepInto) => {
                state.step_mode = Some(StepMode::StepInto);
                tracing::trace!(session_id = %state.session_id, "Debug: step into");
                return Ok(DebuggerCommand::StepInto);
            }
            Some(DebugCommand::StepOver) => {
                state.step_mode = Some(StepMode::StepOver);
                state.step_depth = debugger.call_stack().len();
                tracing::trace!(session_id = %state.session_id, depth = state.step_depth, "Debug: step over");
                return Ok(DebuggerCommand::StepOver);
            }
            Some(DebugCommand::StepOut) => {
                state.step_mode = Some(StepMode::StepOut);
                state.step_depth = debugger.call_stack().len();
                tracing::trace!(session_id = %state.session_id, depth = state.step_depth, "Debug: step out");
                return Ok(DebuggerCommand::Continue); // Rhai doesn't have FunctionExit, use Continue
            }
            Some(DebugCommand::Pause) => {
                // Already paused, ignore
                continue;
            }
            Some(DebugCommand::Evaluate { expression, frame_index }) => {
                // TODO: Evaluate expression in context
                // For now, just continue waiting
                let _ = (expression, frame_index);
                continue;
            }
            None => {
                // Channel closed - debug session terminated
                tracing::info!(session_id = %state.session_id, "Debug session terminated while paused");
                return Err("Debug session terminated".into());
            }
        }
    }
}

/// Collect the call stack from the debugger.
fn collect_call_stack(debugger: &Debugger) -> Vec<StackFrame> {
    debugger
        .call_stack()
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            StackFrame {
                index,
                function_name: frame.to_string(),
                source: None, // Call stack frames don't have source info directly
                line: None,
                column: None,
            }
        })
        .collect()
}

/// Collect variables from a Rhai scope.
pub fn collect_variables_from_scope(scope: &Scope) -> Vec<Variable> {
    scope
        .iter()
        .map(|(name, _const, value)| {
            dynamic_to_variable(name, value, name)
        })
        .collect()
}

/// Convert a Rhai Dynamic value to a Variable for display.
pub fn dynamic_to_variable(name: &str, value: &Dynamic, path: &str) -> Variable {
    let type_name = value.type_name().to_string();
    let expandable = value.is::<Map>() || value.is::<rhai::Array>();

    let formatted_value = if value.is_unit() {
        "()".to_string()
    } else if let Some(b) = value.clone().try_cast::<bool>() {
        b.to_string()
    } else if let Some(i) = value.clone().try_cast::<i64>() {
        i.to_string()
    } else if let Some(f) = value.clone().try_cast::<f64>() {
        format!("{:.6}", f)
    } else if let Some(s) = value.clone().try_cast::<String>() {
        format!("\"{}\"", s)
    } else if let Some(arr) = value.clone().try_cast::<rhai::Array>() {
        format!("Array[{}]", arr.len())
    } else if let Some(map) = value.clone().try_cast::<Map>() {
        format!("Map{{{} entries}}", map.len())
    } else {
        value.to_string()
    };

    Variable {
        name: name.to_string(),
        value: formatted_value,
        type_name,
        expandable,
        path: path.to_string(),
        children: None,
    }
}

/// Expand a Map or Array variable into its children.
pub fn expand_variable(value: &Dynamic, parent_path: &str) -> Vec<Variable> {
    if let Some(arr) = value.clone().try_cast::<rhai::Array>() {
        arr.iter()
            .enumerate()
            .map(|(i, v)| {
                let child_path = format!("{}[{}]", parent_path, i);
                dynamic_to_variable(&format!("[{}]", i), v, &child_path)
            })
            .collect()
    } else if let Some(map) = value.clone().try_cast::<Map>() {
        map.iter()
            .map(|(k, v)| {
                let child_path = format!("{}.{}", parent_path, k);
                dynamic_to_variable(&k.to_string(), v, &child_path)
            })
            .collect()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_to_variable() {
        // Integer
        let var = dynamic_to_variable("x", &Dynamic::from(42_i64), "x");
        assert_eq!(var.name, "x");
        assert_eq!(var.value, "42");
        assert!(!var.expandable);

        // String
        let var = dynamic_to_variable("s", &Dynamic::from("hello".to_string()), "s");
        assert_eq!(var.value, "\"hello\"");

        // Array
        let arr: rhai::Array = vec![Dynamic::from(1_i64), Dynamic::from(2_i64)];
        let var = dynamic_to_variable("arr", &Dynamic::from(arr), "arr");
        assert_eq!(var.value, "Array[2]");
        assert!(var.expandable);

        // Map
        let mut map = Map::new();
        map.insert("key".into(), Dynamic::from("value"));
        let var = dynamic_to_variable("map", &Dynamic::from(map), "map");
        assert_eq!(var.value, "Map{1 entries}");
        assert!(var.expandable);
    }

    #[test]
    fn test_expand_array() {
        let arr: rhai::Array = vec![Dynamic::from(1_i64), Dynamic::from(2_i64)];
        let children = expand_variable(&Dynamic::from(arr), "arr");

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "[0]");
        assert_eq!(children[0].path, "arr[0]");
        assert_eq!(children[1].name, "[1]");
        assert_eq!(children[1].path, "arr[1]");
    }

    #[test]
    fn test_expand_map() {
        let mut map = Map::new();
        map.insert("foo".into(), Dynamic::from(42_i64));
        map.insert("bar".into(), Dynamic::from("hello"));

        let children = expand_variable(&Dynamic::from(map), "data");
        assert_eq!(children.len(), 2);
        // Map iteration order may vary, so just check paths exist
        assert!(children.iter().any(|c| c.path == "data.foo"));
        assert!(children.iter().any(|c| c.path == "data.bar"));
    }
}
