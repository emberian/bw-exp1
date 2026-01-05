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
};

use crate::state::{GMEditorState, StagedChangesState};

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
    }
}
