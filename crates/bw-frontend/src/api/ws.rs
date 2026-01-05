//! WebSocket client for game server communication
//!
//! Handles connection, message serialization, and dispatch to GameState.
//! Uses signals for thread-safe communication in Leptos context.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, WebSocket};

use bw_shared::messages::*;
use bw_shared::{ChatChannel, deserialize_message, serialize_message};

use crate::state::{GameState, SquadronInfo, SquadronInviteInfo, AllianceProposalInfo};

/// Container for WebSocket closures to prevent memory leaks.
/// When this is dropped, the closures are properly freed.
struct WsClosures {
    _onopen: Closure<dyn FnMut(web_sys::Event)>,
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
    _onclose: Closure<dyn FnMut(web_sys::CloseEvent)>,
    _onerror: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

/// Shared state for WebSocket connection, properly managing closure lifetimes.
struct WsConnection {
    ws: WebSocket,
    _closures: WsClosures,
}

/// WebSocket connection state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Maximum reconnect attempts before giving up.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
/// Base delay for reconnection (in milliseconds).
const BASE_RECONNECT_DELAY_MS: u32 = 1000;
/// Maximum delay between reconnect attempts.
const MAX_RECONNECT_DELAY_MS: u32 = 30000;

/// Shared WebSocket connection state (stored in Rc<RefCell> for closure access).
type SharedConnection = Rc<RefCell<Option<WsConnection>>>;

/// WebSocket service for game communication.
/// Uses signals for thread-safe state management.
#[derive(Clone)]
pub struct WsService {
    pub state: RwSignal<ConnectionState>,
    /// Queue of outgoing messages (supports multiple rapid sends)
    outgoing_queue: RwSignal<Vec<ClientMessage>>,
    /// Stored auth token for reconnection
    auth_token: RwSignal<Option<String>>,
    /// Reconnect attempt counter
    reconnect_attempts: RwSignal<u32>,
    /// Whether auto-reconnect is enabled
    auto_reconnect: RwSignal<bool>,
    /// Whether effects have been initialized (to avoid duplicates on reconnect)
    effects_initialized: RwSignal<bool>,
    /// Generation counter to prevent stale reconnection timeouts from firing
    reconnect_generation: RwSignal<u32>,
    /// Active WebSocket connection (closures are dropped when connection is replaced)
    connection: SharedConnection,
}

impl Default for WsService {
    fn default() -> Self {
        Self::new()
    }
}

impl WsService {
    pub fn new() -> Self {
        Self {
            state: RwSignal::new(ConnectionState::Disconnected),
            outgoing_queue: RwSignal::new(Vec::new()),
            auth_token: RwSignal::new(None),
            reconnect_attempts: RwSignal::new(0),
            auto_reconnect: RwSignal::new(true),
            effects_initialized: RwSignal::new(false),
            reconnect_generation: RwSignal::new(0),
            connection: Rc::new(RefCell::new(None)),
        }
    }

    /// Get the current reconnect attempt count.
    pub fn reconnect_attempt(&self) -> u32 {
        self.reconnect_attempts.get()
    }

    /// Calculate reconnect delay with exponential backoff.
    #[allow(dead_code)]
    fn reconnect_delay(&self) -> u32 {
        let attempts = self.reconnect_attempts.get();
        let delay = BASE_RECONNECT_DELAY_MS * 2u32.pow(attempts.min(10));
        delay.min(MAX_RECONNECT_DELAY_MS)
    }

    /// Disable auto-reconnect (e.g., on manual disconnect).
    pub fn disable_reconnect(&self) {
        self.auto_reconnect.set(false);
        // Reset counter so next reconnect cycle starts fresh
        self.reconnect_attempts.set(0);
    }

    /// Enable auto-reconnect.
    pub fn enable_reconnect(&self) {
        self.auto_reconnect.set(true);
    }

    /// Get current connection state.
    pub fn connection_state(&self) -> ConnectionState {
        self.state.get()
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.state.get() == ConnectionState::Connected
    }

    /// Queue a message to send.
    fn send(&self, msg: ClientMessage) {
        self.outgoing_queue.update(|queue| queue.push(msg));
    }

    // Convenience methods for common messages

    /// Authenticate with the server.
    pub fn authenticate(&self, token: String) {
        self.send(ClientMessage::Authenticate { token });
    }

    /// Request to move to a position.
    pub fn move_to_position(&self, x: f64, y: f64) {
        self.send(ClientMessage::move_to_position(x, y, 0.0));
    }

    /// Request to move to a location.
    pub fn move_to_location(&self, location_id: Uuid) {
        self.send(ClientMessage::move_to_location(location_id));
    }

    /// Stop current movement.
    pub fn stop_movement(&self) {
        self.send(ClientMessage::stop_movement());
    }

    /// Dock at a station.
    pub fn dock(&self, station_id: Uuid) {
        self.send(ClientMessage::dock(station_id));
    }

    /// Undock from current station.
    pub fn undock(&self) {
        self.send(ClientMessage::undock());
    }

    /// Use a station service.
    pub fn use_service(&self, service: String) {
        self.send(ClientMessage::use_service(service));
    }

    /// Accept a mission.
    pub fn accept_mission(&self, mission_id: Uuid) {
        self.send(ClientMessage::accept_mission(mission_id));
    }

    /// Abandon a mission.
    pub fn abandon_mission(&self, mission_id: Uuid) {
        self.send(ClientMessage::abandon_mission(mission_id));
    }

    /// Make a mission choice.
    pub fn make_mission_choice(&self, mission_id: Uuid, choice_id: String) {
        self.send(ClientMessage::mission_choice(mission_id, choice_id));
    }

    /// Engage a target in combat.
    pub fn engage_target(&self, target_id: Uuid) {
        self.send(ClientMessage::engage_target(target_id));
    }

    /// Disengage from combat.
    pub fn disengage_combat(&self) {
        self.send(ClientMessage::disengage_combat());
    }

    /// Fire a weapon at a target.
    pub fn fire_weapon(&self, weapon_index: usize, target_id: Uuid) {
        self.send(ClientMessage::fire_weapon(weapon_index, target_id));
    }

    /// Send a chat message.
    pub fn send_chat(&self, message: String, channel: ChatChannel) {
        self.send(ClientMessage::SendChat { message, channel });
    }

    /// Create a squadron.
    pub fn create_squadron(&self, name: String, tag: String) {
        self.send(ClientMessage::create_squadron(name, tag));
    }

    /// Leave current squadron.
    pub fn leave_squadron(&self) {
        self.send(ClientMessage::leave_squadron());
    }

    /// Perform a squadron action.
    pub fn squadron_action(&self, action: SquadronAction) {
        // Convert SquadronAction enum to script action
        let (action_type, params) = match action {
            SquadronAction::PromoteToOfficer { player_id } => {
                ("promote_to_officer", serde_json::json!({ "player_id": player_id }))
            }
            SquadronAction::DemoteOfficer { player_id } => {
                ("demote_officer", serde_json::json!({ "player_id": player_id }))
            }
            SquadronAction::KickMember { player_id } => {
                ("kick_member", serde_json::json!({ "player_id": player_id }))
            }
            SquadronAction::TransferLeadership { player_id } => {
                ("transfer_leadership", serde_json::json!({ "player_id": player_id }))
            }
            SquadronAction::SetMotto { motto } => {
                ("set_motto", serde_json::json!({ "motto": motto }))
            }
            SquadronAction::EnableWargames { enabled } => {
                ("enable_wargames", serde_json::json!({ "enabled": enabled }))
            }
            SquadronAction::EnablePrivateering { enabled } => {
                ("enable_privateering", serde_json::json!({ "enabled": enabled }))
            }
            SquadronAction::DeclareWar { squadron_id } => {
                ("declare_war", serde_json::json!({ "squadron_id": squadron_id }))
            }
            SquadronAction::MakePeace { squadron_id } => {
                ("make_peace", serde_json::json!({ "squadron_id": squadron_id }))
            }
            SquadronAction::FormAlliance { squadron_id } => {
                ("form_alliance", serde_json::json!({ "squadron_id": squadron_id }))
            }
        };
        self.send(ClientMessage::squadron_action(action_type, params));
    }

    /// Invite a player to your squadron.
    pub fn invite_to_squadron(&self, player_id: Uuid) {
        self.send(ClientMessage::invite_to_squadron(player_id));
    }

    /// Accept a squadron invitation.
    pub fn accept_squadron_invite(&self, invite_id: Uuid) {
        self.send(ClientMessage::accept_squadron_invite(invite_id));
    }

    /// Decline a squadron invitation.
    pub fn decline_squadron_invite(&self, invite_id: Uuid) {
        self.send(ClientMessage::decline_squadron_invite(invite_id));
    }

    /// Accept an alliance proposal.
    pub fn accept_alliance(&self, proposal_id: Uuid) {
        self.send(ClientMessage::accept_alliance(proposal_id));
    }

    /// Decline an alliance proposal.
    pub fn decline_alliance(&self, proposal_id: Uuid) {
        self.send(ClientMessage::decline_alliance(proposal_id));
    }

    /// Send a quick "Yo" style hail to another ship.
    pub fn hail(&self, target_id: Uuid) {
        self.send(ClientMessage::hail(target_id));
    }

    /// Request to move to another sector.
    pub fn move_to_sector(&self, sector_id: Uuid) {
        self.send(ClientMessage::move_to_sector(sector_id));
    }

    /// Join a specific sector.
    pub fn join_sector(&self, sector_id: Uuid) {
        self.send(ClientMessage::JoinSector { sector_id });
    }

    /// Leave current sector.
    pub fn leave_sector(&self) {
        self.send(ClientMessage::LeaveSector);
    }

    /// Send a ping for latency measurement.
    pub fn ping(&self) {
        let timestamp = js_sys::Date::now() as u64;
        self.send(ClientMessage::Ping { timestamp });
    }

    /// Subscribe to performance metrics updates.
    pub fn subscribe_metrics(&self) {
        self.send(ClientMessage::SubscribeMetrics);
    }

    /// Unsubscribe from performance metrics updates.
    pub fn unsubscribe_metrics(&self) {
        self.send(ClientMessage::UnsubscribeMetrics);
    }

    /// Connect to the game server WebSocket.
    ///
    /// This establishes the WebSocket connection and sets up event handlers.
    /// The connection will automatically authenticate with the provided token.
    pub fn connect(&self, game_state: GameState, token: String) {
        // Store token for potential reconnection
        self.auth_token.set(Some(token.clone()));

        // Reset reconnect attempts on fresh connection
        self.reconnect_attempts.set(0);

        // Enable auto-reconnect
        self.auto_reconnect.set(true);

        self.connect_internal(game_state, token);
    }

    /// Internal connect method used for both initial connection and reconnection.
    fn connect_internal(&self, game_state: GameState, token: String) {
        // Set connecting state (but keep Reconnecting if already reconnecting)
        if self.state.get() != ConnectionState::Reconnecting {
            self.state.set(ConnectionState::Connecting);
        }

        // Determine WebSocket URL based on current location
        let location = match web_sys::window() {
            Some(w) => w.location(),
            None => {
                game_state.set_error("No window available".to_string());
                self.state.set(ConnectionState::Disconnected);
                return;
            }
        };
        let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
        let host = location.host().unwrap_or_else(|_| "localhost:3000".to_string());
        let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
        let ws_url = format!("{}//{}/ws", ws_protocol, host);

        // Create WebSocket
        let ws = match WebSocket::new(&ws_url) {
            Ok(ws) => ws,
            Err(e) => {
                game_state.set_error(format!("Failed to create WebSocket: {:?}", e));
                self.state.set(ConnectionState::Disconnected);
                return;
            }
        };

        // Set binary type to arraybuffer for MessagePack
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Store WebSocket in Rc<RefCell> for closures
        let ws_ref = Rc::new(RefCell::new(Some(ws.clone())));

        // Clone signals for closures
        let state_signal = self.state;
        let outgoing_queue_signal = self.outgoing_queue;

        // onopen handler
        let ws_open = ws_ref.clone();
        let token_clone = token.clone();
        let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
            state_signal.set(ConnectionState::Connected);

            // Note: Don't reset reconnect_attempts here - wait until we receive
            // successful auth/InitialState to avoid resetting on brief connections

            // Send authentication message
            if let Some(ref ws) = *ws_open.borrow() {
                let auth_msg = ClientMessage::Authenticate { token: token_clone.clone() };
                if let Ok(bytes) = serialize_message(&auth_msg) {
                    let _ = ws.send_with_u8_array(&bytes);
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // onmessage handler
        let game_state_msg = game_state;
        let reconnect_attempts_signal = self.reconnect_attempts;
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Get binary data from message
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                let bytes: Vec<u8> = uint8_array.to_vec();

                // Deserialize MessagePack
                if let Ok(server_msg) = deserialize_message::<ServerMessage>(&bytes) {
                    // Reset reconnect attempts on successful InitialState
                    // (This means the connection is fully established)
                    if matches!(server_msg, ServerMessage::InitialState { .. }) {
                        reconnect_attempts_signal.set(0);
                    }
                    handle_server_message(&game_state_msg, server_msg);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // onclose handler - with auto-reconnect support
        let game_state_close = game_state;
        let auth_token_signal = self.auth_token;
        let reconnect_attempts_signal = self.reconnect_attempts;
        let auto_reconnect_signal = self.auto_reconnect;
        let reconnect_generation_signal = self.reconnect_generation;
        let ws_service = *self;
        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            game_state_close.connected.set(false);

            // Check if we should attempt reconnection
            let attempts = reconnect_attempts_signal.get();
            let should_reconnect = auto_reconnect_signal.get()
                && attempts < MAX_RECONNECT_ATTEMPTS
                && auth_token_signal.get().is_some();

            if should_reconnect {
                // Set reconnecting state
                state_signal.set(ConnectionState::Reconnecting);
                reconnect_attempts_signal.set(attempts + 1);

                // Increment generation to invalidate any previously scheduled reconnects
                let generation = reconnect_generation_signal.get();
                reconnect_generation_signal.set(generation + 1);
                let expected_generation = generation + 1;

                // Calculate delay with exponential backoff
                let delay = BASE_RECONNECT_DELAY_MS * 2u32.pow(attempts.min(10));
                let delay = delay.min(MAX_RECONNECT_DELAY_MS);

                // Schedule reconnection
                let game_state_reconnect = game_state_close;
                let ws_service_reconnect = ws_service;
                let token = auth_token_signal.get().unwrap();
                let reconnect_closure = Closure::once(Box::new(move || {
                    // Only reconnect if this is still the active reconnection attempt
                    // (generation matches) and we're still in reconnecting state
                    if ws_service_reconnect.reconnect_generation.get() == expected_generation
                        && ws_service_reconnect.state.get() == ConnectionState::Reconnecting
                    {
                        ws_service_reconnect.connect_internal(game_state_reconnect, token);
                    }
                }) as Box<dyn FnOnce()>);

                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        reconnect_closure.as_ref().unchecked_ref(),
                        delay as i32,
                    );
                }
                reconnect_closure.forget();

                // Notify user
                game_state_close.notification.set(Some(format!(
                    "Connection lost. Reconnecting in {}s (attempt {}/{})",
                    delay / 1000,
                    attempts + 1,
                    MAX_RECONNECT_ATTEMPTS
                )));
            } else {
                // No reconnection - set disconnected
                state_signal.set(ConnectionState::Disconnected);

                if attempts >= MAX_RECONNECT_ATTEMPTS {
                    game_state_close.set_error("Connection lost. Max reconnect attempts reached.".to_string());
                }
            }
        }) as Box<dyn FnMut(web_sys::CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        // onerror handler
        let game_state_err = game_state;
        let onerror = Closure::wrap(Box::new(move |_: web_sys::ErrorEvent| {
            state_signal.set(ConnectionState::Disconnected);
            game_state_err.set_error("WebSocket connection error".to_string());
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // Watch outgoing queue and send all queued messages
        // Note: Multiple effects may exist after reconnections, but only the one with
        // an open WebSocket will actually drain and send. Others skip because WS is closed.
        let ws_send = ws_ref.clone();
        Effect::new(move |_| {
            // First check if WS is open - only drain queue if we can actually send
            // This prevents message loss when multiple effects exist from reconnections
            if let Some(ref ws) = *ws_send.borrow()
                && ws.ready_state() == WebSocket::OPEN
            {
                // Drain and send all queued messages
                let messages: Vec<ClientMessage> = outgoing_queue_signal
                    .try_update(std::mem::take)
                    .unwrap_or_default();

                for msg in messages {
                    if let Ok(bytes) = serialize_message(&msg) {
                        let _ = ws.send_with_u8_array(&bytes);
                    }
                }
            }
        });

        // Update game state connected flag when state changes
        // Only create this effect once to avoid redundant updates
        let effects_initialized = self.effects_initialized;
        if !effects_initialized.get() {
            effects_initialized.set(true);
            let game_state_connected = game_state;
            Effect::new(move |_| {
                let connected = state_signal.get() == ConnectionState::Connected;
                game_state_connected.connected.set(connected);
            });
        }
    }
}

/// Handle incoming server messages.
pub fn handle_server_message(game_state: &GameState, msg: ServerMessage) {
    match msg {
        ServerMessage::AuthResult {
            success,
            player_id,
            error,
        } => {
            if success {
                game_state.player_id.set(player_id);
                game_state.clear_error();
            } else {
                game_state.set_error(error.unwrap_or_else(|| "Authentication failed".to_string()));
            }
        }

        ServerMessage::InitialState {
            player,
            ship,
            sector,
            ships,
            missions,
        } => {
            game_state.handle_initial_state(player, ship, sector, ships, missions);
        }

        ServerMessage::StateUpdate {
            tick,
            ship_updates,
            ship_spawns,
            ship_despawns,
            mission_spawns,
            mission_updates,
            events,
        } => {
            game_state.handle_state_update(tick, ship_updates, ship_spawns, ship_despawns, mission_spawns, mission_updates, events);
        }

        ServerMessage::ResourceUpdate {
            reputation,
            fame,
            ammunition,
            fuel,
            morale,
            experience,
        } => {
            game_state.update_resources(reputation, fame, ammunition, fuel, morale, experience);
        }

        ServerMessage::MissionChoice {
            mission_id,
            description,
            choices,
        } => {
            game_state.handle_mission_choice(mission_id, description, choices);
        }

        ServerMessage::MissionResult {
            mission_id,
            success,
            reputation_change,
            fame_change,
            narrative,
        } => {
            game_state.handle_mission_result(
                mission_id,
                success,
                reputation_change,
                fame_change,
                narrative,
            );
        }

        ServerMessage::CombatUpdate {
            engagement_id,
            round,
            events,
            is_resolved,
            winner,
        } => {
            game_state.handle_combat_update(engagement_id, round, events, is_resolved, winner);
        }

        ServerMessage::ChatMessage {
            sender_id: _,
            sender_name,
            message,
            channel,
            timestamp,
        } => {
            game_state.add_chat_message(sender_name, message, channel, timestamp);
        }

        ServerMessage::Error { code: _, message } => {
            game_state.set_error(message);
        }

        ServerMessage::Pong {
            timestamp: _,
            server_tick,
        } => {
            game_state.server_tick.set(server_tick);
        }

        ServerMessage::Kicked { reason } => {
            game_state.connected.set(false);
            game_state.set_error(format!("Disconnected: {}", reason));
        }

        ServerMessage::SquadronUpdate { squadron, message } => {
            if let Some(sq) = squadron {
                // Determine if current player is leader or officer by comparing usernames
                let current_username = game_state.username.get_untracked();
                let is_leader = sq.leader_name == current_username;
                let is_officer = sq.officer_names.contains(&current_username);

                game_state.update_squadron(Some(SquadronInfo {
                    id: sq.id,
                    name: sq.name,
                    tag: sq.tag,
                    motto: sq.motto,
                    leader_name: sq.leader_name,
                    member_count: sq.member_count,
                    reputation_bonus: sq.reputation_bonus,
                    fame_bonus: sq.fame_bonus,
                    is_at_war: sq.is_at_war,
                    is_leader,
                    is_officer,
                }));
            } else {
                game_state.update_squadron(None);
            }
            if !message.is_empty() {
                game_state.notification.set(Some(message));
            }
        }

        ServerMessage::SquadronInvite {
            invite_id,
            squadron_id,
            squadron_name,
            squadron_tag,
            inviter_name,
        } => {
            // Store the invite for UI to display
            game_state.pending_squadron_invites.update(|invites| {
                // Remove any existing invite from same squadron
                invites.retain(|i| i.squadron_id != squadron_id);
                invites.push(SquadronInviteInfo {
                    invite_id,
                    squadron_id,
                    squadron_name: squadron_name.clone(),
                    squadron_tag: squadron_tag.clone(),
                    inviter_name: inviter_name.clone(),
                });
            });
            game_state.notification.set(Some(format!(
                "Squadron invite from {}: {} [{}]",
                inviter_name, squadron_name, squadron_tag
            )));
        }

        ServerMessage::AllianceProposal {
            proposal_id,
            from_squadron_id,
            from_squadron_name,
            from_squadron_tag,
        } => {
            // Store the proposal for UI to display
            game_state.pending_alliance_proposals.update(|proposals| {
                // Remove any existing proposal from same squadron
                proposals.retain(|p| p.from_squadron_id != from_squadron_id);
                proposals.push(AllianceProposalInfo {
                    proposal_id,
                    from_squadron_id,
                    from_squadron_name: from_squadron_name.clone(),
                    from_squadron_tag: from_squadron_tag.clone(),
                });
            });
            game_state.notification.set(Some(format!(
                "Alliance proposal from {} [{}]",
                from_squadron_name, from_squadron_tag
            )));
        }

        ServerMessage::HailReceived { from_id, from_name } => {
            // Add hail to state - UI will show indicator on ship
            game_state.add_hail(from_id, from_name);
        }

        ServerMessage::TickMetrics(metrics) => {
            game_state.handle_tick_metrics(metrics);
        }

        ServerMessage::TickMetricsHistory(history) => {
            game_state.handle_tick_metrics_history(history);
        }

        ServerMessage::Admin(_) => {
            // Admin messages not handled in regular client
        }

        ServerMessage::Notification {
            message,
            notification_type: _,
        } => {
            // Display notification from scripts
            game_state.notification.set(Some(message));
        }

        ServerMessage::ChoiceRequired {
            choice_id: _,
            description,
            choices: _,
        } => {
            // TODO: Implement generic choice dialog UI
            // For now, show as notification
            game_state.notification.set(Some(format!("Choice required: {}", description)));
        }

        ServerMessage::ScriptActionResult {
            action: _,
            success: _,
            error,
            data: _,
        } => {
            // Script action results - show error if any
            if let Some(err) = error {
                game_state.set_error(err);
            }
        }
    }
}
