//! WebSocket client for game server communication
//!
//! Handles connection, message serialization, and dispatch to GameState.
//! Uses signals for thread-safe communication in Leptos context.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use bw_shared::messages::*;
use bw_shared::{ChatChannel, deserialize_message, serialize_message};

use crate::state::{GameState, SquadronInfo, SquadronInviteInfo, AllianceProposalInfo};

/// WebSocket connection state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Message to send via WebSocket (queued in signal)
#[derive(Clone, Debug)]
pub struct OutgoingMessage(pub ClientMessage);

/// Maximum reconnect attempts before giving up.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
/// Base delay for reconnection (in milliseconds).
const BASE_RECONNECT_DELAY_MS: u32 = 1000;
/// Maximum delay between reconnect attempts.
const MAX_RECONNECT_DELAY_MS: u32 = 30000;

/// WebSocket service for game communication.
/// Uses signals for thread-safe state management.
#[derive(Clone, Copy)]
pub struct WsService {
    pub state: RwSignal<ConnectionState>,
    pub outgoing: RwSignal<Option<OutgoingMessage>>,
    /// Stored auth token for reconnection
    auth_token: RwSignal<Option<String>>,
    /// Reconnect attempt counter
    reconnect_attempts: RwSignal<u32>,
    /// Whether auto-reconnect is enabled
    auto_reconnect: RwSignal<bool>,
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
            outgoing: RwSignal::new(None),
            auth_token: RwSignal::new(None),
            reconnect_attempts: RwSignal::new(0),
            auto_reconnect: RwSignal::new(true),
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
        self.outgoing.set(Some(OutgoingMessage(msg)));
    }

    // Convenience methods for common messages

    /// Authenticate with the server.
    pub fn authenticate(&self, token: String) {
        self.send(ClientMessage::Authenticate { token });
    }

    /// Request to move to a position.
    pub fn move_to_position(&self, x: f64, y: f64) {
        self.send(ClientMessage::MoveToPosition { x, y, z: 0.0 });
    }

    /// Request to move to a location.
    pub fn move_to_location(&self, location_id: Uuid) {
        self.send(ClientMessage::MoveToLocation { location_id });
    }

    /// Stop current movement.
    pub fn stop_movement(&self) {
        self.send(ClientMessage::StopMovement);
    }

    /// Dock at a station.
    pub fn dock(&self, station_id: Uuid) {
        self.send(ClientMessage::DockAtStation { station_id });
    }

    /// Undock from current station.
    pub fn undock(&self) {
        self.send(ClientMessage::Undock);
    }

    /// Use a station service.
    pub fn use_service(&self, service: String) {
        self.send(ClientMessage::UseService { service });
    }

    /// Accept a mission.
    pub fn accept_mission(&self, mission_id: Uuid) {
        self.send(ClientMessage::AcceptMission { mission_id });
    }

    /// Abandon a mission.
    pub fn abandon_mission(&self, mission_id: Uuid) {
        self.send(ClientMessage::AbandonMission { mission_id });
    }

    /// Make a mission choice.
    pub fn make_mission_choice(&self, mission_id: Uuid, choice_id: String) {
        self.send(ClientMessage::MakeMissionChoice {
            mission_id,
            choice_id,
        });
    }

    /// Engage a target in combat.
    pub fn engage_target(&self, target_id: Uuid) {
        self.send(ClientMessage::EngageTarget { target_id });
    }

    /// Disengage from combat.
    pub fn disengage_combat(&self) {
        self.send(ClientMessage::DisengageCombat);
    }

    /// Fire a weapon at a target.
    pub fn fire_weapon(&self, weapon_index: usize, target_id: Uuid) {
        self.send(ClientMessage::FireWeapon {
            weapon_index,
            target_id,
        });
    }

    /// Send a chat message.
    pub fn send_chat(&self, message: String, channel: ChatChannel) {
        self.send(ClientMessage::SendChat { message, channel });
    }

    /// Create a squadron.
    pub fn create_squadron(&self, name: String, tag: String) {
        self.send(ClientMessage::CreateSquadron { name, tag });
    }

    /// Leave current squadron.
    pub fn leave_squadron(&self) {
        self.send(ClientMessage::LeaveSquadron);
    }

    /// Perform a squadron action.
    pub fn squadron_action(&self, action: SquadronAction) {
        self.send(ClientMessage::SquadronAction { action });
    }

    /// Invite a player to your squadron.
    pub fn invite_to_squadron(&self, player_id: Uuid) {
        self.send(ClientMessage::InviteToSquadron { player_id });
    }

    /// Accept a squadron invitation.
    pub fn accept_squadron_invite(&self, invite_id: Uuid) {
        self.send(ClientMessage::AcceptSquadronInvite { invite_id });
    }

    /// Decline a squadron invitation.
    pub fn decline_squadron_invite(&self, invite_id: Uuid) {
        self.send(ClientMessage::DeclineSquadronInvite { invite_id });
    }

    /// Accept an alliance proposal.
    pub fn accept_alliance(&self, proposal_id: Uuid) {
        self.send(ClientMessage::AcceptAlliance { proposal_id });
    }

    /// Decline an alliance proposal.
    pub fn decline_alliance(&self, proposal_id: Uuid) {
        self.send(ClientMessage::DeclineAlliance { proposal_id });
    }

    /// Request to move to another sector.
    pub fn move_to_sector(&self, sector_id: Uuid) {
        self.send(ClientMessage::MoveToSector { sector_id });
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
        // Set connecting state
        self.state.set(ConnectionState::Connecting);

        // Determine WebSocket URL based on current location
        let location = web_sys::window()
            .expect("no window")
            .location();
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
        let outgoing_signal = self.outgoing;

        // onopen handler
        let ws_open = ws_ref.clone();
        let token_clone = token.clone();
        let reconnect_attempts_signal = self.reconnect_attempts;
        let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
            state_signal.set(ConnectionState::Connected);

            // Reset reconnect attempts on successful connection
            reconnect_attempts_signal.set(0);

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
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Get binary data from message
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                let bytes: Vec<u8> = uint8_array.to_vec();

                // Deserialize MessagePack
                if let Ok(server_msg) = deserialize_message::<ServerMessage>(&bytes) {
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

                // Calculate delay with exponential backoff
                let delay = BASE_RECONNECT_DELAY_MS * 2u32.pow(attempts.min(10));
                let delay = delay.min(MAX_RECONNECT_DELAY_MS);

                // Schedule reconnection
                let game_state_reconnect = game_state_close;
                let ws_service_reconnect = ws_service;
                let token = auth_token_signal.get().unwrap();
                let reconnect_closure = Closure::once(Box::new(move || {
                    // Only reconnect if still in reconnecting state
                    if ws_service_reconnect.state.get() == ConnectionState::Reconnecting {
                        ws_service_reconnect.connect_internal(game_state_reconnect, token);
                    }
                }) as Box<dyn FnOnce()>);

                let _ = web_sys::window()
                    .expect("no window")
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        reconnect_closure.as_ref().unchecked_ref(),
                        delay as i32,
                    );
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

        // Watch outgoing signal and send messages
        let ws_send = ws_ref.clone();
        Effect::new(move |_| {
            if let Some(OutgoingMessage(msg)) = outgoing_signal.get() {
                if let Some(ref ws) = *ws_send.borrow()
                    && ws.ready_state() == WebSocket::OPEN
                        && let Ok(bytes) = serialize_message(&msg) {
                            let _ = ws.send_with_u8_array(&bytes);
                        }
                // Clear the outgoing signal after sending
                outgoing_signal.set(None);
            }
        });

        // Update game state connected flag when state changes
        let game_state_connected = game_state;
        Effect::new(move |_| {
            let connected = state_signal.get() == ConnectionState::Connected;
            game_state_connected.connected.set(connected);
        });
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
            mission_updates,
            events,
        } => {
            game_state.handle_state_update(tick, ship_updates, ship_spawns, ship_despawns, mission_updates, events);
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
                    is_leader: false,
                    is_officer: false,
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
    }
}
