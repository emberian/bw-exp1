//! Rhai Debugger Integration
//!
//! Integrates with Rhai's built-in debugger API to provide:
//! - Breakpoint handling
//! - Step-through execution
//! - Variable inspection
//! - Call stack collection

use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Engine, Dynamic, Map, Scope, ImmutableString};
use rhai::debugger::{BreakPoint, Debugger, DebuggerCommand, DebuggerEvent};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use super::types::*;

/// Type of breakpoint for index mapping.
#[derive(Debug, Clone, Copy)]
pub enum BreakpointType {
    /// Line breakpoint at a position
    Line,
    /// Function entry breakpoint
    FunctionEntry,
}

/// State maintained externally for the debugger callback.
/// This is stored in an Arc and captured by closures since Rhai's debugger
/// state can only hold a Dynamic value.
pub struct DebuggerState {
    /// Session ID
    pub session_id: Uuid,
    /// Shared breakpoints (can be updated externally)
    pub breakpoints: SharedBreakpoints,
    /// Shared function breakpoints
    pub function_breakpoints: SharedFunctionBreakpoints,
    /// Channel to send pause notifications
    pub pause_tx: broadcast::Sender<PausedState>,
    /// Channel to receive commands (wrapped in sync Mutex for callback use)
    pub command_rx: Arc<std::sync::Mutex<mpsc::Receiver<DebugCommand>>>,
    /// Current step mode
    pub step_mode: RwLock<Option<StepMode>>,
    /// Call depth when step was initiated (for step-over/out)
    pub step_depth: RwLock<usize>,
    /// Entity context (set before script execution)
    pub entity_context: Option<EntityContext>,
    /// Script path being executed
    pub script_path: String,
    /// Mapping from Rhai breakpoint index to our breakpoint ID and type
    /// This is set during init and used when breakpoints fire
    pub breakpoint_index_to_id: RwLock<Vec<(Uuid, BreakpointType)>>,
}

/// Register the debugger with a Rhai engine.
///
/// This configures the engine to support debugging with the given session's
/// breakpoints and channels. The complex state is stored in an Arc and
/// captured by the callbacks, while Rhai's internal debugger state stores
/// just a session ID reference.
pub fn register_debugger(
    engine: &mut Engine,
    session_id: Uuid,
    breakpoints: SharedBreakpoints,
    function_breakpoints: SharedFunctionBreakpoints,
    pause_tx: broadcast::Sender<PausedState>,
    command_rx: Arc<std::sync::Mutex<mpsc::Receiver<DebugCommand>>>,
    script_path: String,
    entity_context: Option<EntityContext>,
) {
    // Create the shared state that will be captured by closures
    let state = Arc::new(DebuggerState {
        session_id,
        breakpoints: breakpoints.clone(),
        function_breakpoints: function_breakpoints.clone(),
        pause_tx,
        command_rx,
        step_mode: RwLock::new(None),
        step_depth: RwLock::new(0),
        entity_context,
        script_path: script_path.clone(),
        breakpoint_index_to_id: RwLock::new(Vec::new()),
    });

    let init_state = Arc::clone(&state);
    let callback_state = Arc::clone(&state);

    #[allow(deprecated)] // register_debugger is marked volatile but not deprecated
    engine.register_debugger(
        // Init callback - called once when debugging starts
        move |_engine, mut debugger| {
            // Store session ID as the debugger's internal state
            debugger.set_state(init_state.session_id.to_string());

            // Register breakpoints with Rhai and store the index mapping
            let index_to_id = sync_breakpoints_to_rhai(
                &mut debugger,
                &init_state.breakpoints.read(),
                &init_state.function_breakpoints.read(),
                &init_state.script_path,
            );
            *init_state.breakpoint_index_to_id.write() = index_to_id;

            debugger
        },
        // Step callback - called at each debugger event
        move |context, event, _node, source, pos| {
            handle_debug_event(&callback_state, context.call_level(), &context, event, source, pos)
        },
    );
}

/// Sync breakpoints from our format to Rhai's format.
/// Returns a mapping from Rhai breakpoint index to our breakpoint ID and type.
fn sync_breakpoints_to_rhai(
    debugger: &mut Debugger,
    breakpoints: &[Breakpoint],
    function_breakpoints: &[FunctionBreakpoint],
    current_script: &str,
) -> Vec<(Uuid, BreakpointType)> {
    // Clear existing breakpoints
    debugger.break_points_mut().clear();

    let mut index_to_id = Vec::new();

    // Add enabled line breakpoints for the current script
    for bp in breakpoints {
        if bp.enabled && bp.source == current_script {
            let source: Option<ImmutableString> = Some(bp.source.clone().into());
            debugger.break_points_mut().push(BreakPoint::AtPosition {
                source,
                pos: rhai::Position::new(bp.line as u16, bp.column.unwrap_or(1) as u16),
                enabled: true,
            });
            index_to_id.push((bp.id, BreakpointType::Line));
        }
    }

    // Add enabled function breakpoints (these are global, not script-specific)
    for fbp in function_breakpoints {
        if fbp.enabled && fbp.break_on_entry {
            debugger.break_points_mut().push(BreakPoint::AtFunctionName {
                name: fbp.function_name.clone().into(),
                enabled: true,
            });
            index_to_id.push((fbp.id, BreakpointType::FunctionEntry));
        }
    }

    index_to_id
}

/// Handle a debug event from Rhai.
fn handle_debug_event(
    state: &Arc<DebuggerState>,
    call_level: usize,
    context: &rhai::EvalContext,
    event: DebuggerEvent,
    source: Option<&str>,
    pos: rhai::Position,
) -> Result<DebuggerCommand, Box<rhai::EvalAltResult>> {
    match event {
        DebuggerEvent::Start => {
            // Script execution starting
            tracing::trace!(session_id = %state.session_id, "Debug: script start");
            Ok(DebuggerCommand::Continue)
        }

        DebuggerEvent::Step => {
            // Check if we should pause based on step mode
            let step_mode = *state.step_mode.read();
            if let Some(mode) = step_mode {
                let step_depth = *state.step_depth.read();
                let should_pause = match mode {
                    StepMode::StepInto => true,
                    StepMode::StepOver => call_level <= step_depth,
                    StepMode::StepOut => call_level < step_depth,
                };

                if should_pause {
                    *state.step_mode.write() = None;
                    return pause_and_wait(
                        state,
                        context,
                        source,
                        pos,
                        PauseReason::Step,
                    );
                }
            }
            Ok(DebuggerCommand::Continue)
        }

        DebuggerEvent::BreakPoint(bp_index) => {
            // A breakpoint was hit - look up our ID and type from the index mapping
            let (bp_id, bp_type) = state.breakpoint_index_to_id.read()
                .get(bp_index)
                .copied()
                .unwrap_or((Uuid::nil(), BreakpointType::Line));

            tracing::debug!(
                session_id = %state.session_id,
                breakpoint_id = %bp_id,
                breakpoint_type = ?bp_type,
                rhai_index = bp_index,
                line = ?pos.line(),
                "Breakpoint hit"
            );

            // Handle based on breakpoint type
            match bp_type {
                BreakpointType::Line => {
                    // Check for conditional breakpoint (only for line breakpoints)
                    let condition = state.breakpoints.read()
                        .iter()
                        .find(|b| b.id == bp_id)
                        .and_then(|bp| bp.condition.clone());

                    // Evaluate condition if present
                    let should_pause = if let Some(ref cond_expr) = condition {
                        // Create a scope clone for evaluation
                        let mut eval_scope = context.scope().clone();
                        match context.engine().eval_expression_with_scope::<bool>(&mut eval_scope, cond_expr) {
                            Ok(result) => result,
                            Err(e) => {
                                // On evaluation error, treat as true (pause) but log warning
                                tracing::warn!(
                                    session_id = %state.session_id,
                                    breakpoint_id = %bp_id,
                                    condition = %cond_expr,
                                    error = %e,
                                    "Conditional breakpoint evaluation failed, pausing anyway"
                                );
                                true
                            }
                        }
                    } else {
                        true // No condition = always pause
                    };

                    if !should_pause {
                        tracing::trace!(
                            session_id = %state.session_id,
                            breakpoint_id = %bp_id,
                            "Conditional breakpoint skipped (condition false)"
                        );
                        return Ok(DebuggerCommand::Continue);
                    }

                    // Increment hit count (only when actually pausing)
                    if bp_id != Uuid::nil()
                        && let Some(bp) = state.breakpoints.write().iter_mut().find(|b| b.id == bp_id) {
                            bp.hit_count += 1;
                        }

                    pause_and_wait(
                        state,
                        context,
                        source,
                        pos,
                        PauseReason::Breakpoint { breakpoint_id: bp_id },
                    )
                }

                BreakpointType::FunctionEntry => {
                    // Get function name for the pause reason
                    let function_name = state.function_breakpoints.read()
                        .iter()
                        .find(|fbp| fbp.id == bp_id)
                        .map(|fbp| fbp.function_name.clone())
                        .unwrap_or_else(|| "<unknown>".to_string());

                    pause_and_wait(
                        state,
                        context,
                        source,
                        pos,
                        PauseReason::FunctionEntry { function_name },
                    )
                }
            }
        }

        DebuggerEvent::FunctionExitWithValue(value) => {
            // Function exit with return value
            let step_mode = *state.step_mode.read();
            let step_depth = *state.step_depth.read();
            if matches!(step_mode, Some(StepMode::StepOut)) && call_level < step_depth {
                *state.step_mode.write() = None;
                return pause_and_wait(
                    state,
                    context,
                    source,
                    pos,
                    PauseReason::Step,
                );
            }

            let _ = value; // Silence unused warning
            Ok(DebuggerCommand::Continue)
        }

        DebuggerEvent::FunctionExitWithError(err) => {
            // Function exit with error - pause on exceptions
            tracing::debug!(
                session_id = %state.session_id,
                error = %err,
                "Function exited with error"
            );

            pause_and_wait(
                state,
                context,
                source,
                pos,
                PauseReason::Exception { message: err.to_string() },
            )
        }

        DebuggerEvent::End => {
            // Script execution ended
            tracing::trace!(session_id = %state.session_id, "Debug: script end");
            Ok(DebuggerCommand::Continue)
        }

        // DebuggerEvent is non-exhaustive, handle future variants
        _ => Ok(DebuggerCommand::Continue),
    }
}

/// Pause execution and wait for a command from the GM client.
fn pause_and_wait(
    state: &Arc<DebuggerState>,
    context: &rhai::EvalContext,
    source: Option<&str>,
    pos: rhai::Position,
    reason: PauseReason,
) -> Result<DebuggerCommand, Box<rhai::EvalAltResult>> {
    // Collect call stack from debugger
    let debugger = context.global_runtime_state().debugger();
    let call_stack = collect_call_stack(debugger);

    // Collect local variables from scope
    let local_variables = collect_variables_from_scope(context.scope());

    // Build paused state
    let paused_state = PausedState {
        script_path: source.unwrap_or(&state.script_path).to_string(),
        line: pos.line().unwrap_or(0),
        column: pos.position().unwrap_or(0),
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
                *state.step_mode.write() = Some(StepMode::StepInto);
                tracing::trace!(session_id = %state.session_id, "Debug: step into");
                return Ok(DebuggerCommand::StepInto);
            }
            Some(DebugCommand::StepOver) => {
                *state.step_mode.write() = Some(StepMode::StepOver);
                let depth = context.global_runtime_state().debugger().call_stack().len();
                *state.step_depth.write() = depth;
                tracing::trace!(session_id = %state.session_id, depth = depth, "Debug: step over");
                return Ok(DebuggerCommand::StepOver);
            }
            Some(DebugCommand::StepOut) => {
                *state.step_mode.write() = Some(StepMode::StepOut);
                let depth = context.global_runtime_state().debugger().call_stack().len();
                *state.step_depth.write() = depth;
                tracing::trace!(session_id = %state.session_id, depth = depth, "Debug: step out");
                // Rhai doesn't have a direct StepOut command, use Continue with our tracking
                return Ok(DebuggerCommand::Continue);
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
                function_name: frame.fn_name.to_string(),
                source: frame.source.as_ref().map(|s| s.to_string()),
                line: frame.pos.line(),
                column: frame.pos.position(),
            }
        })
        .collect()
}

/// Collect variables from a Rhai scope.
pub fn collect_variables_from_scope(scope: &Scope) -> Vec<Variable> {
    scope
        .iter()
        .map(|(name, _const, value)| {
            dynamic_to_variable(name, &value, name)
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
#[allow(dead_code)] // Public API for debugger clients - implementation pending in server
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
                dynamic_to_variable(k.as_ref(), v, &child_path)
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
