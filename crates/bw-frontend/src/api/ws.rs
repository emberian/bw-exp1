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

use crate::state::{GameState, SquadronInfo};

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

/// WebSocket service for game communication.
/// Uses signals for thread-safe state management.
#[derive(Clone, Copy)]
pub struct WsService {
    pub state: RwSignal<ConnectionState>,
    pub outgoing: RwSignal<Option<OutgoingMessage>>,
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
        }
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
        let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
            state_signal.set(ConnectionState::Connected);

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

        // onclose handler
        let game_state_close = game_state;
        let onclose = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
            state_signal.set(ConnectionState::Disconnected);
            game_state_close.connected.set(false);
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
            mission_updates: _,
            events: _,
        } => {
            game_state.handle_state_update(tick, ship_updates, ship_spawns, ship_despawns);
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
            invite_id: _,
            squadron_id: _,
            squadron_name,
            squadron_tag,
            inviter_name,
        } => {
            game_state.notification.set(Some(format!(
                "Squadron invite from {}: {} [{}]",
                inviter_name, squadron_name, squadron_tag
            )));
        }

        ServerMessage::AllianceProposal {
            proposal_id: _,
            from_squadron_id: _,
            from_squadron_name,
            from_squadron_tag,
        } => {
            game_state.notification.set(Some(format!(
                "Alliance proposal from {} [{}]",
                from_squadron_name, from_squadron_tag
            )));
        }
    }
}
