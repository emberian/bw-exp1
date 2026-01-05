//! Admin WebSocket client for GM Editor
//!
//! Uses thread_local storage since WASM is single-threaded and Leptos
//! context requires Send+Sync which Rc<RefCell<WebSocket>> doesn't provide.

use std::cell::RefCell;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use bw_shared::{
    AdminClientMessage, AdminServerMessage, ClientMessage, EntityFilter, EntityType,
    ServerMessage, StagedChange,
    dto::DebugTargetDto,
};

use crate::state::{GMEditorState, StagedChangesState, DebugPanelState};

thread_local! {
    static ADMIN_WS: RefCell<Option<AdminWsClient>> = const { RefCell::new(None) };
}

/// Initialize the admin WebSocket client (call once from app.rs)
pub fn init_admin_ws(ws_url: &str, auth_token: &str) {
    ADMIN_WS.with(|ws| {
        *ws.borrow_mut() = Some(AdminWsClient::new(ws_url, auth_token));
    });
}

/// Execute a function with access to the admin WebSocket client
pub fn with_admin_ws<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AdminWsClient) -> R,
{
    ADMIN_WS.with(|ws| ws.borrow().as_ref().map(f))
}

/// Shutdown the admin WebSocket client
pub fn shutdown_admin_ws() {
    ADMIN_WS.with(|ws| {
        if let Some(client) = ws.borrow_mut().take() {
            client.close();
        }
    });
}

/// WebSocket client for admin operations
pub struct AdminWsClient {
    ws: RefCell<Option<WebSocket>>,
}

impl AdminWsClient {
    fn new(ws_url: &str, auth_token: &str) -> Self {
        let client = Self {
            ws: RefCell::new(None),
        };

        client.connect(ws_url, auth_token);
        client
    }

    /// Connect to the WebSocket server
    fn connect(&self, ws_url: &str, auth_token: &str) {
        let ws = match WebSocket::new(ws_url) {
            Ok(ws) => ws,
            Err(e) => {
                tracing::error!("[gm-ws] Failed to create WebSocket: {:?}", e);
                return;
            }
        };

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Store reference for sending
        *self.ws.borrow_mut() = Some(ws.clone());

        // Set up open handler - authenticate on connect
        let auth_token = auth_token.to_string();
        let ws_clone = ws.clone();
        let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
            tracing::info!("[gm-ws] Connected, authenticating...");

            let auth_msg = ClientMessage::Authenticate {
                token: auth_token.clone(),
            };

            if let Ok(bytes) = rmp_serde::to_vec(&auth_msg) {
                let array = js_sys::Uint8Array::from(&bytes[..]);
                if let Err(e) = ws_clone.send_with_array_buffer(&array.buffer()) {
                    tracing::error!("[gm-ws] Failed to send auth: {:?}", e);
                }
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // Message handler
        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&buffer);
                let bytes = array.to_vec();

                if let Ok(msg) = rmp_serde::from_slice::<ServerMessage>(&bytes) {
                    handle_server_message(msg);
                }
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // Error handler
        let onerror = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
            tracing::error!("[gm-ws] WebSocket error: {:?}", e.message());
        }) as Box<dyn FnMut(_)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // Close handler
        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            tracing::info!("[gm-ws] Connection closed");
        }) as Box<dyn FnMut(_)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();
    }

    /// Close the WebSocket connection
    fn close(&self) {
        if let Some(ws) = self.ws.borrow().as_ref() {
            let _ = ws.close();
        }
    }

    /// Send an admin message
    fn send(&self, msg: AdminClientMessage) {
        let client_msg = ClientMessage::Admin(msg);

        if let Ok(bytes) = rmp_serde::to_vec(&client_msg) {
            if let Some(ws) = self.ws.borrow().as_ref() {
                let array = js_sys::Uint8Array::from(&bytes[..]);
                if let Err(e) = ws.send_with_array_buffer(&array.buffer()) {
                    tracing::error!("[gm-ws] Failed to send: {:?}", e);
                }
            }
        }
    }

    /// Request list of scripts
    pub fn list_scripts(&self) {
        self.send(AdminClientMessage::ListScripts);
    }

    /// Read a script file
    pub fn read_script(&self, path: &str) {
        self.send(AdminClientMessage::ReadScript {
            path: path.to_string(),
        });
    }

    /// Get simulation config
    pub fn get_sim_config(&self) {
        self.send(AdminClientMessage::GetSimConfig);
    }

    /// Query entities
    pub fn query_entities(
        &self,
        entity_type: EntityType,
        filters: Vec<EntityFilter>,
        limit: usize,
        offset: usize,
    ) {
        self.send(AdminClientMessage::QueryEntities {
            entity_type,
            filters,
            limit,
            offset,
        });
    }

    /// Get entity details
    pub fn get_entity(&self, entity_type: EntityType, id: uuid::Uuid) {
        self.send(AdminClientMessage::GetEntity { entity_type, id });
    }

    /// Query sectors
    pub fn query_sectors(&self) {
        self.send(AdminClientMessage::QuerySectors);
    }

    /// Preview staged changes
    pub fn preview_staged(&self, changes: Vec<StagedChange>) {
        self.send(AdminClientMessage::PreviewStaged { changes });
    }

    /// Commit staged changes
    pub fn commit_staged(&self, changes: Vec<StagedChange>) {
        self.send(AdminClientMessage::CommitStaged { changes });
    }

    /// Subscribe to script errors
    pub fn subscribe_script_errors(&self) {
        self.send(AdminClientMessage::SubscribeScriptErrors);
    }

    /// Unsubscribe from script errors
    pub fn unsubscribe_script_errors(&self) {
        self.send(AdminClientMessage::UnsubscribeScriptErrors);
    }

    /// Request reload of a specific script
    pub fn reload_script(&self, path: &str) {
        self.send(AdminClientMessage::ReloadScript {
            path: path.to_string(),
        });
    }

    /// Request reload of archetype definitions
    pub fn reload_definitions(&self) {
        self.send(AdminClientMessage::ReloadDefinitions);
    }

    /// Get recent script errors
    pub fn get_recent_errors(&self, limit: usize) {
        self.send(AdminClientMessage::GetRecentScriptErrors { limit });
    }

    // === Debug Operations ===

    /// Start a debug session
    pub fn start_debug_session(&self, target: DebugTargetDto) {
        self.send(AdminClientMessage::StartDebugSession { target });
    }

    /// End the current debug session
    pub fn end_debug_session(&self) {
        self.send(AdminClientMessage::EndDebugSession);
    }

    /// Set a line breakpoint
    pub fn set_breakpoint(&self, script: &str, line: usize, condition: Option<String>) {
        self.send(AdminClientMessage::SetBreakpoint {
            script: script.to_string(),
            line,
            condition,
        });
    }

    /// Set a function breakpoint
    pub fn set_function_breakpoint(&self, function_name: &str, break_on_entry: bool, break_on_exit: bool) {
        self.send(AdminClientMessage::SetFunctionBreakpoint {
            function_name: function_name.to_string(),
            break_on_entry,
            break_on_exit,
        });
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&self, breakpoint_id: uuid::Uuid) {
        self.send(AdminClientMessage::RemoveBreakpoint { breakpoint_id });
    }

    /// Toggle a breakpoint
    pub fn toggle_breakpoint(&self, breakpoint_id: uuid::Uuid, enabled: bool) {
        self.send(AdminClientMessage::ToggleBreakpoint { breakpoint_id, enabled });
    }

    /// List all breakpoints
    pub fn list_breakpoints(&self) {
        self.send(AdminClientMessage::ListBreakpoints);
    }

    /// Continue execution
    pub fn debug_continue(&self) {
        self.send(AdminClientMessage::DebugContinue);
    }

    /// Pause execution
    pub fn debug_pause(&self) {
        self.send(AdminClientMessage::DebugPause);
    }

    /// Step into
    pub fn debug_step_into(&self) {
        self.send(AdminClientMessage::DebugStepInto);
    }

    /// Step over
    pub fn debug_step_over(&self) {
        self.send(AdminClientMessage::DebugStepOver);
    }

    /// Step out
    pub fn debug_step_out(&self) {
        self.send(AdminClientMessage::DebugStepOut);
    }

    /// Get variables for a stack frame
    pub fn get_variables(&self, frame_index: usize) {
        self.send(AdminClientMessage::GetVariables { frame_index });
    }

    /// Expand a variable
    pub fn expand_variable(&self, variable_path: &str) {
        self.send(AdminClientMessage::ExpandVariable {
            variable_path: variable_path.to_string(),
        });
    }

    /// Evaluate an expression
    pub fn evaluate_expression(&self, expression: &str, frame_index: Option<usize>) {
        self.send(AdminClientMessage::EvaluateExpression {
            expression: expression.to_string(),
            frame_index,
        });
    }

    /// Get the call stack
    pub fn get_call_stack(&self) {
        self.send(AdminClientMessage::GetCallStack);
    }
}

/// Handle incoming server messages
fn handle_server_message(msg: ServerMessage) {
    match msg {
        ServerMessage::Admin(admin_msg) => handle_admin_message(admin_msg),
        ServerMessage::AuthResult { success, error, .. } => {
            if success {
                tracing::info!("[gm-ws] Authenticated successfully");
            } else {
                tracing::error!("[gm-ws] Auth failed: {:?}", error);
            }
        }
        _ => {
            // Ignore non-admin messages
        }
    }
}

/// Handle admin-specific messages
fn handle_admin_message(msg: AdminServerMessage) {
    // Try to get contexts - they may not be available during initial setup
    let gm_state = use_context::<GMEditorState>();
    let staged = use_context::<StagedChangesState>();

    match msg {
        AdminServerMessage::ScriptList { files } => {
            if let Some(state) = gm_state {
                state.scripts.set(files);
                state.loading_scripts.set(false);
            }
        }

        AdminServerMessage::ScriptContent {
            path,
            content,
            last_modified: _,
        } => {
            if let Some(state) = gm_state {
                state.selected_script.set(Some(path));
                state.script_content.set(content.clone());
                state.script_original.set(content);
                state.loading_scripts.set(false);
            }
        }

        AdminServerMessage::SimConfigData { config } => {
            if let Some(state) = gm_state {
                state.sim_config.set(Some(config));
                state.loading_config.set(false);
            }
        }

        AdminServerMessage::EntityList {
            entities,
            total_count: _,
            entity_type: _,
        } => {
            if let Some(state) = gm_state {
                state.entities.set(entities);
                state.loading_entities.set(false);
            }
        }

        AdminServerMessage::EntityDetails { data, .. } => {
            if let Some(state) = gm_state {
                state.entity_details.set(Some(data));
            }
        }

        AdminServerMessage::SectorList { sectors } => {
            if let Some(state) = gm_state {
                state.sectors.set(sectors);
            }
        }

        AdminServerMessage::StagedPreview { changes, errors } => {
            if let Some(staged) = staged {
                staged.update_previews(changes, errors);
                staged.preview_loading.set(false);
            }
        }

        AdminServerMessage::CommitResult {
            success,
            applied_count,
            errors,
        } => {
            if let Some(staged) = staged {
                staged.commit_loading.set(false);
                if success {
                    staged.clear();
                    staged
                        .last_result
                        .set(Some(format!("Successfully applied {} changes", applied_count)));
                } else {
                    staged
                        .last_result
                        .set(Some(format!("Commit failed: {}", errors.join(", "))));
                }
            }
        }

        AdminServerMessage::AdminError { code, message } => {
            tracing::error!("[gm-ws] Admin error {}: {}", code, message);
            if let Some(state) = gm_state {
                state.error.set(Some(format!("{}: {}", code, message)));
                state.clear_error_delayed();
            }
        }

        // Playtest messages - not yet handled in GM editor UI
        AdminServerMessage::PlaytestCreated { playtest_id, name } => {
            tracing::info!("[gm-ws] Playtest created: {} ({})", name, playtest_id);
        }
        AdminServerMessage::PlaytestList { playtests } => {
            tracing::info!("[gm-ws] Received {} playtests", playtests.len());
        }
        AdminServerMessage::PlaytestDetails { playtest } => {
            tracing::info!("[gm-ws] Playtest details: {}", playtest.name);
        }
        AdminServerMessage::PlaytestJoined { playtest_id, .. } => {
            tracing::info!("[gm-ws] Joined playtest: {}", playtest_id);
        }
        AdminServerMessage::PlaytestLeft => {
            tracing::info!("[gm-ws] Left playtest");
        }
        AdminServerMessage::PlaytestInviteSent { playtest_id, player_id } => {
            tracing::info!("[gm-ws] Invite sent to {} for playtest {}", player_id, playtest_id);
        }
        AdminServerMessage::PlaytestPlayerKicked { playtest_id, player_id } => {
            tracing::info!("[gm-ws] Player {} kicked from playtest {}", player_id, playtest_id);
        }
        AdminServerMessage::PlaytestStateChanged { playtest_id, paused, time_scale, tick } => {
            tracing::info!("[gm-ws] Playtest {} state: paused={}, scale={}, tick={}", playtest_id, paused, time_scale, tick);
        }
        AdminServerMessage::PlaytestDestroyed { playtest_id } => {
            tracing::info!("[gm-ws] Playtest destroyed: {}", playtest_id);
        }
        AdminServerMessage::PromotePreview { ships_to_update, players_to_update, new_entities, deletions } => {
            tracing::info!("[gm-ws] Promote preview: {} ships, {} players, {} new, {} deleted",
                ships_to_update, players_to_update, new_entities, deletions);
        }
        AdminServerMessage::PromoteResult { success, applied_count, errors } => {
            if success {
                tracing::info!("[gm-ws] Promote succeeded: {} changes applied", applied_count);
            } else {
                tracing::warn!("[gm-ws] Promote failed: {:?}", errors);
            }
        }

        // === Script Debugging ===

        AdminServerMessage::ScriptError { script, function, message, line, column, tick } => {
            tracing::warn!("[gm-ws] Script error in {}::{}:{}: {} (tick {})",
                script, function, line, message, tick);
            if let Some(state) = gm_state {
                state.add_script_error(script, function, message, line, column, tick);
            }
        }

        AdminServerMessage::ScriptErrors { errors } => {
            tracing::info!("[gm-ws] Received {} script errors", errors.len());
            if let Some(state) = gm_state {
                for error in errors {
                    state.add_script_error(
                        error.script,
                        error.function,
                        error.message,
                        error.line,
                        error.column,
                        error.tick,
                    );
                }
            }
        }

        AdminServerMessage::ScriptReloaded { path, success, error, warnings } => {
            if success {
                tracing::info!("[gm-ws] Script reloaded: {} ({} warnings)", path, warnings.len());
                if let Some(state) = gm_state {
                    state.add_notification(format!("Reloaded: {}", path), "success".to_string());
                    for warning in warnings {
                        state.add_notification(format!("Warning: {}", warning), "warning".to_string());
                    }
                }
            } else {
                tracing::warn!("[gm-ws] Script reload failed: {} - {:?}", path, error);
                if let Some(state) = gm_state {
                    state.add_notification(
                        format!("Reload failed: {} - {}", path, error.unwrap_or_default()),
                        "error".to_string(),
                    );
                }
            }
        }

        AdminServerMessage::DefinitionsReloaded { file, ships_loaded, weapons_loaded, errors } => {
            tracing::info!("[gm-ws] Definitions reloaded: {} ships, {} weapons from {}",
                ships_loaded, weapons_loaded, file);
            if let Some(state) = gm_state {
                if errors.is_empty() {
                    state.add_notification(
                        format!("Reloaded: {} ships, {} weapons", ships_loaded, weapons_loaded),
                        "success".to_string(),
                    );
                } else {
                    state.add_notification(
                        format!("Partial reload: {} errors", errors.len()),
                        "warning".to_string(),
                    );
                }
            }
        }

        AdminServerMessage::SubscribedToScriptErrors => {
            tracing::info!("[gm-ws] Subscribed to script errors");
        }

        AdminServerMessage::UnsubscribedFromScriptErrors => {
            tracing::info!("[gm-ws] Unsubscribed from script errors");
        }

        // === Interactive Debugger Messages ===

        AdminServerMessage::DebugSessionStarted { session_id } => {
            tracing::info!("[gm-ws] Debug session started: {}", session_id);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_session_started(session_id);
            }
        }

        AdminServerMessage::DebugSessionEnded => {
            tracing::info!("[gm-ws] Debug session ended");
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_session_ended();
            }
        }

        AdminServerMessage::BreakpointSet { breakpoint } => {
            tracing::debug!("[gm-ws] Breakpoint set: {}:{}",
                breakpoint.script, breakpoint.line);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_breakpoint_set(breakpoint);
            }
        }

        AdminServerMessage::FunctionBreakpointSet { breakpoint } => {
            tracing::debug!("[gm-ws] Function breakpoint set: {}",
                breakpoint.function_name);
            // Function breakpoints stored separately in debug state
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.function_breakpoints.update(|bps| bps.push(breakpoint));
            }
        }

        AdminServerMessage::BreakpointRemoved { breakpoint_id } => {
            tracing::debug!("[gm-ws] Breakpoint removed: {}", breakpoint_id);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_breakpoint_removed(breakpoint_id);
            }
        }

        AdminServerMessage::BreakpointToggled { breakpoint_id, enabled } => {
            tracing::debug!("[gm-ws] Breakpoint toggled: {} = {}",
                breakpoint_id, enabled);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_breakpoint_toggled(breakpoint_id, enabled);
            }
        }

        AdminServerMessage::BreakpointList { breakpoints, function_breakpoints } => {
            tracing::debug!("[gm-ws] Received {} breakpoints, {} function breakpoints",
                breakpoints.len(), function_breakpoints.len());
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_breakpoint_list(breakpoints, function_breakpoints);
            }
        }

        AdminServerMessage::DebugPaused { script, line, column, reason, call_stack, entity_context } => {
            tracing::info!("[gm-ws] Debug paused at {}:{}", script, line);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_paused(script, line, column, reason, call_stack, entity_context);
                // Automatically request variables for frame 0
                with_admin_ws(|ws| ws.get_variables(0));
            }
        }

        AdminServerMessage::DebugResumed => {
            tracing::debug!("[gm-ws] Debug resumed");
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_resumed();
            }
        }

        AdminServerMessage::DebugVariables { frame_index, variables } => {
            tracing::debug!("[gm-ws] Received {} variables for frame {}",
                variables.len(), frame_index);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_variables(frame_index, variables);
            }
        }

        AdminServerMessage::EvaluationResult { expression, result, type_name, success, error } => {
            if success {
                tracing::debug!("[gm-ws] Eval '{}' = {} ({})",
                    expression, result, type_name);
            } else {
                tracing::warn!("[gm-ws] Eval '{}' failed: {:?}",
                    expression, error);
            }
            // Could store in state for display if needed
        }

        AdminServerMessage::CallStack { frames } => {
            tracing::debug!("[gm-ws] Received call stack with {} frames",
                frames.len());
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_call_stack(frames);
            }
        }

        AdminServerMessage::DebugError { message } => {
            tracing::error!("[gm-ws] Debug error: {}", message);
            if let Some(debug_state) = use_context::<DebugPanelState>() {
                debug_state.on_error(message);
            }
        }

        AdminServerMessage::VariableExpanded { variable_path, children } => {
            tracing::debug!("[gm-ws] Variable expanded '{}': {} children",
                variable_path, children.len());
            // Could update variables in state if we implement expansion UI
            let _ = (variable_path, children);
        }
    }
}
