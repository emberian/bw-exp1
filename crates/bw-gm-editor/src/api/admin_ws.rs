//! Admin WebSocket client for GM Editor
//!
//! Uses thread_local storage since WASM is single-threaded and Leptos
//! context requires Send+Sync which Rc<RefCell<WebSocket>> doesn't provide.

use std::cell::RefCell;

use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use bw_shared::{
    AdminClientMessage, AdminServerMessage, ClientMessage, EntityFilter, EntityType,
    ServerMessage, StagedChange,
    dto::DebugTargetDto,
};

use crate::state::{
    GMEditorState, StagedChangesState, DebugPanelState,
    SchemaState, ValidationState, InspectorState, ExportState,
};

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
    /// Store closures to prevent memory leaks (dropped when client is dropped)
    #[allow(dead_code)]
    closures: RefCell<Vec<Closure<dyn FnMut(web_sys::Event)>>>,
    #[allow(dead_code)]
    message_closure: RefCell<Option<Closure<dyn FnMut(MessageEvent)>>>,
    #[allow(dead_code)]
    error_closure: RefCell<Option<Closure<dyn FnMut(web_sys::ErrorEvent)>>>,
    #[allow(dead_code)]
    close_closure: RefCell<Option<Closure<dyn FnMut(web_sys::CloseEvent)>>>,
}

impl AdminWsClient {
    fn new(ws_url: &str, auth_token: &str) -> Self {
        let mut client = Self {
            ws: RefCell::new(None),
            closures: RefCell::new(Vec::new()),
            message_closure: RefCell::new(None),
            error_closure: RefCell::new(None),
            close_closure: RefCell::new(None),
        };

        client.connect(ws_url, auth_token);
        client
    }

    /// Connect to the WebSocket server
    fn connect(&mut self, ws_url: &str, auth_token: &str) {
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
        // Store closure to prevent memory leak
        self.closures.borrow_mut().push(onopen);

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
        // Store closure to prevent memory leak
        *self.message_closure.borrow_mut() = Some(onmessage);

        // Error handler
        let onerror = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
            tracing::error!("[gm-ws] WebSocket error: {:?}", e.message());
        }) as Box<dyn FnMut(_)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        // Store closure to prevent memory leak
        *self.error_closure.borrow_mut() = Some(onerror);

        // Close handler
        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            tracing::info!("[gm-ws] Connection closed");
        }) as Box<dyn FnMut(_)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        // Store closure to prevent memory leak
        *self.close_closure.borrow_mut() = Some(onclose);
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

    // === Schema Introspection ===

    /// Get all archetype schemas
    pub fn get_archetype_schemas(&self) {
        self.send(AdminClientMessage::GetArchetypeSchemas);
    }

    /// Get detailed schema for a specific archetype type
    pub fn get_archetype_schema(&self, archetype_type: &str) {
        self.send(AdminClientMessage::GetArchetypeSchema {
            archetype_type: archetype_type.to_string(),
        });
    }

    /// Get action parameter schemas
    pub fn get_action_schemas(&self) {
        self.send(AdminClientMessage::GetActionSchemas);
    }

    // === Script Validation ===

    /// Validate a script without saving
    pub fn validate_script(&self, path: &str, content: &str) {
        self.send(AdminClientMessage::ValidateScript {
            path: path.to_string(),
            content: content.to_string(),
        });
    }

    /// Validate an archetype definition
    pub fn validate_definition(&self, definition_type: &str, content: &str) {
        self.send(AdminClientMessage::ValidateDefinition {
            definition_type: definition_type.to_string(),
            content: content.to_string(),
        });
    }

    // === State Introspection ===

    /// Get a snapshot of state for entity type
    pub fn get_state_snapshot(&self, entity_type: EntityType, limit: usize, offset: usize) {
        use bw_shared::dto::DebugTargetDto;
        self.send(AdminClientMessage::GetStateSnapshot {
            target: DebugTargetDto::Live,
            entity_type,
            limit,
            offset,
        });
    }

    /// Create a watch expression
    pub fn create_watch(&self, expression: &str, name: Option<String>) {
        self.send(AdminClientMessage::CreateWatch {
            expression: expression.to_string(),
            name,
        });
    }

    /// Remove a watch expression
    pub fn remove_watch(&self, watch_id: uuid::Uuid) {
        self.send(AdminClientMessage::RemoveWatch { watch_id });
    }

    /// List all watches
    pub fn list_watches(&self) {
        self.send(AdminClientMessage::ListWatches);
    }

    // === Export ===

    /// Create a new export
    pub fn create_export(&self, name: &str, config: bw_shared::dto::ExportConfigDto) {
        self.send(AdminClientMessage::CreateExport {
            name: name.to_string(),
            config,
        });
    }

    /// Get export status
    pub fn get_export_status(&self, export_id: uuid::Uuid) {
        self.send(AdminClientMessage::GetExportStatus { export_id });
    }

    /// List all exports
    pub fn list_exports(&self) {
        self.send(AdminClientMessage::ListExports);
    }

    /// Delete an export
    pub fn delete_export(&self, export_id: uuid::Uuid) {
        self.send(AdminClientMessage::DeleteExport { export_id });
    }

    /// Request export download
    pub fn download_export(&self, export_id: uuid::Uuid) {
        self.send(AdminClientMessage::DownloadExport { export_id });
    }
}

// =============================================================================
// Convenience Functions (for simpler API in components)
// =============================================================================

/// Send a request to get archetype schemas
pub fn send_get_archetype_schemas() {
    with_admin_ws(|ws| ws.get_archetype_schemas());
}

/// Send a request to get a specific archetype schema
pub fn send_get_archetype_schema(archetype_type: &str) {
    let t = archetype_type.to_string();
    with_admin_ws(move |ws| ws.get_archetype_schema(&t));
}

/// Send a request to get state snapshot
pub fn send_get_state_snapshot(entity_type: EntityType, limit: usize, offset: usize) {
    with_admin_ws(move |ws| ws.get_state_snapshot(entity_type, limit, offset));
}

/// Send a request to list exports
pub fn send_list_exports() {
    with_admin_ws(|ws| ws.list_exports());
}

/// Send a request to create an export
pub fn send_create_export(name: &str, config: bw_shared::dto::ExportConfigDto) {
    let n = name.to_string();
    with_admin_ws(move |ws| ws.create_export(&n, config));
}

/// Send a request to delete an export
pub fn send_delete_export(export_id: uuid::Uuid) {
    with_admin_ws(move |ws| ws.delete_export(export_id));
}

/// Send a request to download an export
pub fn send_download_export(export_id: uuid::Uuid) {
    with_admin_ws(move |ws| ws.download_export(export_id));
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

        // === Schema Introspection ===

        AdminServerMessage::ArchetypeSchemas { schemas } => {
            tracing::info!("[gm-ws] Received {} archetype schemas", schemas.len());
            if let Some(schema_state) = use_context::<SchemaState>() {
                schema_state.on_schemas(schemas);
            }
        }

        AdminServerMessage::ArchetypeSchemaDetail { archetype_type, name, fields } => {
            tracing::info!("[gm-ws] Schema detail for {}: {} ({} fields)",
                archetype_type, name, fields.len());
            if let Some(schema_state) = use_context::<SchemaState>() {
                schema_state.on_schema_detail(archetype_type, name, fields);
            }
        }

        AdminServerMessage::ActionSchemas { actions } => {
            tracing::info!("[gm-ws] Received {} action schemas", actions.len());
            if let Some(schema_state) = use_context::<SchemaState>() {
                schema_state.on_action_schemas(actions);
            }
        }

        // === Validation ===

        AdminServerMessage::ValidationResult { path, errors, warnings, is_valid } => {
            if is_valid {
                tracing::info!("[gm-ws] Validation passed for {}", path);
            } else {
                tracing::warn!("[gm-ws] Validation failed for {}: {} errors, {} warnings",
                    path, errors.len(), warnings.len());
            }
            if let Some(validation_state) = use_context::<ValidationState>() {
                validation_state.on_validation_result(path, errors, warnings, is_valid);
            }
        }

        AdminServerMessage::DefinitionValidationResult { definition_type, errors, warnings, is_valid } => {
            if is_valid {
                tracing::info!("[gm-ws] Definition validation passed for {}", definition_type);
            } else {
                tracing::warn!("[gm-ws] Definition validation failed for {}: {} errors, {} warnings",
                    definition_type, errors.len(), warnings.len());
            }
            if let Some(validation_state) = use_context::<ValidationState>() {
                validation_state.on_validation_result(definition_type, errors, warnings, is_valid);
            }
        }

        // === State Introspection ===

        AdminServerMessage::StateSubscribed { entity_types } => {
            tracing::info!("[gm-ws] Subscribed to state updates for {:?}", entity_types);
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_subscribed(entity_types);
            }
        }

        AdminServerMessage::StateUnsubscribed => {
            tracing::info!("[gm-ws] Unsubscribed from state updates");
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_unsubscribed();
            }
        }

        AdminServerMessage::StateUpdate { tick, entity_type, changes } => {
            tracing::debug!("[gm-ws] State update (tick {}): {:?} - {} changes",
                tick, entity_type, changes.len());
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_state_update(tick, entity_type, changes);
            }
        }

        AdminServerMessage::StateSnapshot { tick, entity_type, entities, total_count } => {
            tracing::info!("[gm-ws] State snapshot (tick {}): {:?} - {} of {} entities",
                tick, entity_type, entities.len(), total_count);
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_snapshot(tick, entity_type, entities, total_count);
            }
        }

        AdminServerMessage::WatchCreated { watch } => {
            tracing::info!("[gm-ws] Watch created: {} ({})",
                watch.name.as_deref().unwrap_or("unnamed"), watch.id);
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_watch_created(watch);
            }
        }

        AdminServerMessage::WatchRemoved { watch_id } => {
            tracing::info!("[gm-ws] Watch removed: {}", watch_id);
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_watch_removed(watch_id);
            }
        }

        AdminServerMessage::WatchList { watches } => {
            tracing::info!("[gm-ws] Received {} watches", watches.len());
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_watch_list(watches);
            }
        }

        AdminServerMessage::WatchValue { watch_id, tick, value, error } => {
            if let Some(ref err) = error {
                tracing::warn!("[gm-ws] Watch {} error at tick {}: {}", watch_id, tick, err);
            } else {
                tracing::debug!("[gm-ws] Watch {} value at tick {}: {:?}", watch_id, tick, value);
            }
            if let Some(inspector_state) = use_context::<InspectorState>() {
                inspector_state.on_watch_value(watch_id, tick, value, error);
            }
        }

        // === Export ===

        AdminServerMessage::ExportCreated { export_id, name } => {
            tracing::info!("[gm-ws] Export created: {} ({})", name, export_id);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_export_created(export_id, name.clone());
            }
            if let Some(state) = gm_state {
                state.add_notification(format!("Export '{}' started", name), "info".to_string());
            }
        }

        AdminServerMessage::ExportProgress { export_id, phase, percent } => {
            tracing::info!("[gm-ws] Export {} progress: {} ({}%)", export_id, phase, percent);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_progress(export_id, phase, percent);
            }
        }

        AdminServerMessage::ExportCompleted { export_id, size_bytes, download_url } => {
            let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
            tracing::info!("[gm-ws] Export {} completed: {:.2}MB, url: {}",
                export_id, size_mb, download_url);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_completed(export_id, size_bytes, download_url);
            }
            if let Some(state) = gm_state {
                state.add_notification(
                    format!("Export completed ({:.2}MB)", size_mb),
                    "success".to_string(),
                );
            }
        }

        AdminServerMessage::ExportFailed { export_id, error } => {
            tracing::error!("[gm-ws] Export {} failed: {}", export_id, error);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_failed(export_id, error.clone());
            }
            if let Some(state) = gm_state {
                state.add_notification(format!("Export failed: {}", error), "error".to_string());
            }
        }

        AdminServerMessage::ExportStatus { export } => {
            tracing::info!("[gm-ws] Export status: {} - {:?}", export.name, export.status);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_status(export);
            }
        }

        AdminServerMessage::ExportList { exports } => {
            tracing::info!("[gm-ws] Received {} exports", exports.len());
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_export_list(exports);
            }
        }

        AdminServerMessage::ExportDeleted { export_id } => {
            tracing::info!("[gm-ws] Export deleted: {}", export_id);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_deleted(export_id);
            }
        }

        AdminServerMessage::ExportDownloadUrl { export_id, url, expires_at } => {
            tracing::info!("[gm-ws] Export {} download URL: {} (expires {})",
                export_id, url, expires_at);
            if let Some(export_state) = use_context::<ExportState>() {
                export_state.on_download_url(export_id, url, expires_at);
            }
        }
    }
}
