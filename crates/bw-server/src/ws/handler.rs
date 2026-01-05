//! WebSocket connection handler

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use bw_shared::{ClientMessage, ServerMessage, deserialize_message, serialize_message};
use crate::{GameState, auth::hash_token, config::config};

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GameState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<GameState>) {
    let (mut sender, mut receiver) = socket.split();

    // Channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Player ID (set after authentication)
    let mut player_id: Option<Uuid> = None;
    let mut sector_id: Option<Uuid> = None;
    let mut username: Option<String> = None;
    // Unique ID for this connection (used to prevent race conditions during reconnect cleanup)
    let mut connection_id: Option<Uuid> = None;

    // Task to forward server messages to client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(bytes) = serialize_message(&msg) {
                if sender.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Binary(data) => {
                if let Ok(client_msg) = deserialize_message::<ClientMessage>(&data) {
                    match client_msg {
                        ClientMessage::Authenticate { token } => {
                            // Validate session token against database
                            let token_hash = hash_token(&token);

                            let auth_result = match state.db.find_session(&token_hash).await {
                                Ok(Some(session)) if !session.is_expired() => {
                                    // Valid session - check player exists
                                    if state.players.contains_key(&session.player_id) {
                                        Some(session.player_id)
                                    } else {
                                        // Player not in active session cache, try to load
                                        if let Ok(Some(player)) = state.db.find_player(session.player_id).await {
                                            // Load player into cache
                                            let pid = player.id;
                                            let ship_id = player.active_ship_id;
                                            let sid = player.patrol_sector_id;
                                            state.player_data.insert(pid, player);

                                            // Create session tracking
                                            state.players.insert(pid, crate::PlayerSession {
                                                player_id: pid,
                                                ship_id,
                                                sector_id: sid,
                                                connection: None,
                                                connection_id: None,
                                                playtest_id: None,
                                            });

                                            Some(pid)
                                        } else {
                                            None
                                        }
                                    }
                                }
                                _ => None,
                            };

                            if let Some(pid) = auth_result {
                                player_id = Some(pid);

                                // Get sector and username from player session
                                if let Some(session) = state.players.get(&pid) {
                                    sector_id = Some(session.sector_id);
                                }
                                if let Some(player) = state.player_data.get(&pid) {
                                    username = Some(player.username.clone());
                                }

                                // Register connection with unique ID for reconnect safety
                                let conn_id = Uuid::new_v4();
                                connection_id = Some(conn_id);
                                if let Some(mut session) = state.players.get_mut(&pid) {
                                    session.connection = Some(tx.clone());
                                    session.connection_id = Some(conn_id);
                                }

                                // Send success
                                let _ = tx.send(ServerMessage::AuthResult {
                                    success: true,
                                    player_id: Some(pid),
                                    error: None,
                                }).await;

                                // Send initial state
                                if let Some(sid) = sector_id {
                                    send_initial_state(&state, &tx, pid, sid).await;
                                }

                                continue;
                            }

                            let _ = tx.send(ServerMessage::AuthResult {
                                success: false,
                                player_id: None,
                                error: Some("Invalid or expired token".to_string()),
                            }).await;
                        }

                        ClientMessage::Ping { timestamp } => {
                            let _ = tx.send(ServerMessage::Pong {
                                timestamp,
                                server_tick: state.get_tick(),
                            }).await;
                        }

                        // Other messages require authentication
                        _ => {
                            let (Some(pid), Some(sid), Some(ref uname)) = (player_id, sector_id, username.as_ref()) else {
                                let _ = tx.send(ServerMessage::Error {
                                    code: "UNAUTHORIZED".to_string(),
                                    message: "Not authenticated".to_string(),
                                }).await;
                                continue;
                            };

                            // Handle game messages
                            handle_game_message(
                                &state,
                                &tx,
                                pid,
                                sid,
                                uname,
                                client_msg,
                            ).await;
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup - only clear connection if it's still ours (prevents race condition on reconnect)
    if let Some(pid) = player_id {
        let should_cleanup = if let Some(mut session) = state.players.get_mut(&pid) {
            // Only clear if this connection owns the session (connection_id matches)
            if session.connection_id == connection_id {
                session.connection = None;
                session.connection_id = None;
                true
            } else {
                // A new connection has taken over - don't clean up their state
                false
            }
        } else {
            false
        };

        if should_cleanup {
            // Remove from sector connections
            if let Some(sid) = sector_id {
                if let Some(sector) = state.sectors.get(&sid) {
                    sector.connections.remove(&pid);
                }
            }

            // Unsubscribe from metrics
            state.metrics.unsubscribe(pid);
        }
    }

    send_task.abort();
}

async fn send_initial_state(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    sector_id: Uuid,
) {
    use bw_shared::dto::*;

    // Get player data
    let session = match state.players.get(&player_id) {
        Some(s) => s,
        None => return,
    };

    // Get ship
    let ship = match state.ships.get(&session.ship_id) {
        Some(s) => s,
        None => return,
    };

    // Get sector
    let sector = match state.sectors.get(&sector_id) {
        Some(s) => s,
        None => return,
    };

    // Get player data for DTO
    let player_ref = state.player_data.get(&player_id);
    let (username, reputation, fame, faction_tag, squadron_tag) = match player_ref.as_ref() {
        Some(p) => (
            p.username.clone(),
            p.resources.reputation,
            p.resources.fame,
            state.factions.get(&p.faction_id).map(|f| f.tag.clone()).unwrap_or_else(|| "COMPACT".to_string()),
            p.squadron_id.and_then(|sid| state.squadrons.get(&sid).map(|s| s.tag.clone())),
        ),
        None => ("Player".to_string(), 100, 0, "COMPACT".to_string(), None),
    };
    drop(player_ref);

    // Check if admin
    let is_admin = config().is_admin(&username);

    // Build DTOs
    let player_dto = PlayerDto {
        id: player_id,
        username,
        reputation,
        fame,
        faction_tag,
        squadron_tag,
        is_online: true,
        is_admin,
    };

    let ship_dto = ShipDto {
        id: ship.id,
        name: ship.name.clone(),
        owner_id: ship.owner_id,
        ship_class: format!("{:?}", ship.ship_class),
        position: PositionDto {
            x: ship.position.x,
            y: ship.position.y,
            z: ship.position.z,
        },
        hull_percent: ship.hull_integrity,
        shield_percent: ship.shield_strength,
        status: format!("{:?}", ship.status),
        faction_tag: ship.faction_id.and_then(|fid| {
            state.factions.get(&fid).map(|f| f.tag.clone())
        }),
        is_player: true,
        is_hostile: false,
    };

    let sector_dto = SectorDto {
        id: sector.sector.id,
        name: sector.sector.name.clone(),
        danger_level: format!("{:?}", sector.sector.danger_level),
        controlling_faction: None,
        locations: sector.sector.locations.iter().map(|loc| LocationDto {
            id: loc.id,
            name: loc.name.clone(),
            location_type: format!("{:?}", loc.location_type),
            position: PositionDto {
                x: loc.position.x,
                y: loc.position.y,
                z: loc.position.z,
            },
            faction_tag: None,
            services: loc.services.iter().map(|s| format!("{:?}", s)).collect(),
        }).collect(),
        adjacent_sectors: vec![],
    };

    // Get other ships in sector
    let ships: Vec<ShipDto> = sector.ship_ids.iter()
        .filter_map(|entry| {
            let ship_id = *entry.key();
            if ship_id == session.ship_id {
                return None; // Skip player's own ship
            }
            state.ships.get(&ship_id).map(|s| ShipDto {
                id: s.id,
                name: s.name.clone(),
                owner_id: s.owner_id,
                ship_class: format!("{:?}", s.ship_class),
                position: PositionDto {
                    x: s.position.x,
                    y: s.position.y,
                    z: s.position.z,
                },
                hull_percent: s.hull_integrity,
                shield_percent: s.shield_strength,
                status: format!("{:?}", s.status),
                faction_tag: s.faction_id.and_then(|fid| {
                    state.factions.get(&fid).map(|f| f.tag.clone())
                }),
                is_player: s.is_player_ship,
                is_hostile: s.ship_class.is_hostile(),
            })
        })
        .collect();

    // Get missions
    let missions: Vec<MissionDto> = sector.missions.iter()
        .map(|m| MissionDto {
            id: m.id,
            title: m.title.clone(),
            description: m.description.clone(),
            mission_type: format!("{:?}", m.mission_type),
            status: format!("{:?}", m.status),
            priority: format!("{:?}", m.priority),
            reputation_reward: m.reputation_reward,
            fame_reward: m.fame_reward,
            is_high_profile: m.is_high_profile,
            expires_in_seconds: m.expires_at.map(|e| {
                let now = chrono::Utc::now();
                if e > now {
                    (e - now).num_seconds() as u32
                } else {
                    0
                }
            }),
            progress: m.progress,
            can_accept: m.can_accept(player_id, None),
        })
        .collect();

    // Send initial state
    let _ = tx.send(ServerMessage::InitialState {
        player: player_dto,
        ship: ship_dto,
        sector: sector_dto,
        ships,
        missions,
    }).await;

    // Register connection with sector
    sector.connections.insert(player_id, tx.clone());
}

async fn handle_game_message(
    state: &Arc<GameState>,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    sector_id: Uuid,
    username: &str,
    msg: ClientMessage,
) {
    use super::admin_handler::handle_admin_message;

    match msg {
        // === Infrastructure Messages (Rust-handled) ===

        ClientMessage::JoinSector { sector_id: target_sector_id } => {
            // Direct sector join (usually after travel completes or for initial join)
            handle_join_sector(state, tx, player_id, sector_id, target_sector_id).await;
        }

        ClientMessage::LeaveSector => {
            // Leave current sector (disconnect from sector broadcast)
            handle_leave_sector(state, tx, player_id, sector_id).await;
        }

        ClientMessage::SendChat { message, channel } => {
            use bw_shared::ChatChannel;

            // Validate message length
            if message.len() > bw_shared::CHAT_MAX_LENGTH {
                let _ = tx.send(ServerMessage::Error {
                    code: "CHAT_TOO_LONG".to_string(),
                    message: format!("Message exceeds {} characters", bw_shared::CHAT_MAX_LENGTH),
                }).await;
                return;
            }

            // Get player username
            let sender_name = state.player_data.get(&player_id)
                .map(|p| p.username.clone())
                .unwrap_or_else(|| "Player".to_string());

            let chat_msg = ServerMessage::ChatMessage {
                sender_id: player_id,
                sender_name,
                message,
                channel,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            match channel {
                ChatChannel::Sector => {
                    // Get the player's current sector from state (not the cached local var
                    // which can become stale after sector changes)
                    let current_sector_id = state.players.get(&player_id)
                        .map(|s| s.sector_id)
                        .unwrap_or(sector_id);

                    if let Some(sector) = state.sectors.get(&current_sector_id) {
                        // Broadcast to others in sector
                        for conn in sector.connections.iter() {
                            if *conn.key() != player_id {
                                let _ = conn.value().send(chat_msg.clone()).await;
                            }
                        }
                    }
                    // Always echo back to sender
                    let _ = tx.send(chat_msg).await;
                }
                _ => {
                    let _ = tx.send(chat_msg).await;
                }
            }
        }

        ClientMessage::SubscribeMetrics => {
            state.metrics.subscribe(player_id);

            // Send current history immediately
            let history = state.metrics.get_history();
            let _ = tx.send(ServerMessage::TickMetricsHistory(history)).await;

            tracing::debug!("Player {} subscribed to metrics", player_id);
        }

        ClientMessage::UnsubscribeMetrics => {
            state.metrics.unsubscribe(player_id);
            tracing::debug!("Player {} unsubscribed from metrics", player_id);
        }

        ClientMessage::Admin(admin_msg) => {
            handle_admin_message(state, tx, player_id, username, admin_msg).await;
        }

        ClientMessage::ScriptAction { action, params } => {
            // Get the player's ship ID
            let ship_id = match state.players.get(&player_id) {
                Some(s) => s.ship_id,
                None => {
                    let _ = tx.send(ServerMessage::ScriptActionResult {
                        action: action.clone(),
                        success: false,
                        error: Some("Session not found".to_string()),
                        data: None,
                    }).await;
                    return;
                }
            };

            // Build action context
            let ctx = bw_scripting::ActionContext {
                player_id,
                ship_id,
                sector_id,
                action: action.clone(),
                params,
            };

            // Dispatch to script handler
            let result = state.action_dispatcher.read().dispatch(ctx);

            // Apply mutations to game state
            if !result.mutations.is_empty() {
                use bw_scripting::StateProvider;
                state.apply_mutations(result.mutations);
            }

            // Send result to client
            let _ = tx.send(ServerMessage::ScriptActionResult {
                action,
                success: result.success,
                error: result.error,
                data: result.data,
            }).await;
        }

        _ => {
            tracing::debug!("Unhandled message from player {}: {:?}", player_id, msg);
        }
    }
}

// ============================================================================
// Sector Travel Handlers
// ============================================================================

/// Handle joining a new sector (transfers player/ship to target sector).
async fn handle_join_sector(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    current_sector_id: Uuid,
    target_sector_id: Uuid,
) {
    use bw_shared::dto::{PlayerDto, ShipDto, SectorDto, LocationDto, PositionDto, MissionDto, AdjacentSectorDto};

    // Don't allow joining the same sector
    if current_sector_id == target_sector_id {
        let _ = tx.send(ServerMessage::Error {
            code: "ALREADY_IN_SECTOR".to_string(),
            message: "You are already in this sector".to_string(),
        }).await;
        return;
    }

    // Get player session
    let (ship_id, _current_sector) = match state.players.get(&player_id) {
        Some(s) => (s.ship_id, s.sector_id),
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "SESSION_NOT_FOUND".to_string(),
                message: "Session not found".to_string(),
            }).await;
            return;
        }
    };

    // Verify target sector exists
    let target_sector = match state.sectors.get(&target_sector_id) {
        Some(s) => s,
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "SECTOR_NOT_FOUND".to_string(),
                message: "Target sector not found".to_string(),
            }).await;
            return;
        }
    };

    // Get ship
    let mut ship = match state.ships.get_mut(&ship_id) {
        Some(s) => s,
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "SHIP_NOT_FOUND".to_string(),
                message: "Ship not found".to_string(),
            }).await;
            return;
        }
    };

    // Remove ship from current sector
    if let Some(current_sector) = state.sectors.get(&current_sector_id) {
        current_sector.ship_ids.remove(&ship_id);
        current_sector.connections.remove(&player_id);

        // Notify other players in current sector of departure
        let despawn_msg = ServerMessage::StateUpdate {
            tick: state.get_tick(),
            ship_updates: vec![],
            ship_spawns: vec![],
            ship_despawns: vec![ship_id],
            mission_updates: vec![],
            events: vec![],
        };
        current_sector.broadcast(despawn_msg).await;
    }

    // Update ship sector and position (spawn at center of new sector)
    ship.sector_id = target_sector_id;
    ship.position = bw_core::models::Position::new(0.0, 0.0, 0.0);
    drop(ship);

    // Add ship to target sector
    target_sector.ship_ids.insert(ship_id, ());
    target_sector.connections.insert(player_id, tx.clone());

    // Update player session
    if let Some(mut session) = state.players.get_mut(&player_id) {
        session.sector_id = target_sector_id;
    }

    // Build DTOs for new sector
    let player = match state.player_data.get(&player_id) {
        Some(p) => p.clone(),
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "PLAYER_NOT_FOUND".to_string(),
                message: "Player data not found".to_string(),
            }).await;
            return;
        }
    };

    let ship = match state.ships.get(&ship_id) {
        Some(s) => s.clone(),
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "SHIP_NOT_FOUND".to_string(),
                message: "Ship not found".to_string(),
            }).await;
            return;
        }
    };

    let player_dto = PlayerDto {
        id: player.id,
        username: player.username.clone(),
        reputation: player.resources.reputation,
        fame: player.resources.fame,
        faction_tag: state.factions.get(&player.faction_id)
            .map(|f| f.tag.clone())
            .unwrap_or_default(),
        squadron_tag: player.squadron_id.and_then(|sq_id| {
            state.squadrons.get(&sq_id).map(|sq| sq.tag.clone())
        }),
        is_online: player.is_online,
        is_admin: config().is_admin(&player.username),
    };

    let ship_dto = ShipDto {
        id: ship.id,
        name: ship.name.clone(),
        owner_id: ship.owner_id,
        ship_class: format!("{:?}", ship.ship_class),
        position: PositionDto {
            x: ship.position.x,
            y: ship.position.y,
            z: ship.position.z,
        },
        hull_percent: ship.hull_integrity,
        shield_percent: ship.shield_strength,
        status: format!("{:?}", ship.status),
        faction_tag: ship.faction_id.and_then(|fid| {
            state.factions.get(&fid).map(|f| f.tag.clone())
        }),
        is_player: true,
        is_hostile: false,
    };

    let sector_dto = SectorDto {
        id: target_sector.sector.id,
        name: target_sector.sector.name.clone(),
        danger_level: format!("{:?}", target_sector.sector.danger_level),
        controlling_faction: None,
        locations: target_sector.sector.locations.iter().map(|loc| LocationDto {
            id: loc.id,
            name: loc.name.clone(),
            location_type: format!("{:?}", loc.location_type),
            position: PositionDto {
                x: loc.position.x,
                y: loc.position.y,
                z: loc.position.z,
            },
            faction_tag: None,
            services: loc.services.iter().map(|s| format!("{:?}", s)).collect(),
        }).collect(),
        // Build adjacent sector DTOs
        adjacent_sectors: target_sector.sector.adjacent_sectors.iter()
            .filter_map(|&adj_id| {
                state.sectors.get(&adj_id).map(|adj| AdjacentSectorDto {
                    id: adj.sector.id,
                    name: adj.sector.name.clone(),
                    danger_level: format!("{:?}", adj.sector.danger_level),
                })
            })
            .collect(),
    };

    // Get other ships in new sector
    let ships: Vec<ShipDto> = target_sector.ship_ids.iter()
        .filter_map(|entry| {
            let other_ship_id = *entry.key();
            if other_ship_id == ship_id {
                return None;
            }
            state.ships.get(&other_ship_id).map(|s| ShipDto {
                id: s.id,
                name: s.name.clone(),
                owner_id: s.owner_id,
                ship_class: format!("{:?}", s.ship_class),
                position: PositionDto {
                    x: s.position.x,
                    y: s.position.y,
                    z: s.position.z,
                },
                hull_percent: s.hull_integrity,
                shield_percent: s.shield_strength,
                status: format!("{:?}", s.status),
                faction_tag: s.faction_id.and_then(|fid| {
                    state.factions.get(&fid).map(|f| f.tag.clone())
                }),
                is_player: s.is_player_ship,
                is_hostile: s.ship_class.is_hostile(),
            })
        })
        .collect();

    // Get missions in new sector
    let missions: Vec<MissionDto> = target_sector.missions.iter()
        .map(|m| MissionDto {
            id: m.id,
            title: m.title.clone(),
            description: m.description.clone(),
            mission_type: format!("{:?}", m.mission_type),
            status: format!("{:?}", m.status),
            priority: format!("{:?}", m.priority),
            reputation_reward: m.reputation_reward,
            fame_reward: m.fame_reward,
            is_high_profile: m.is_high_profile,
            expires_in_seconds: m.expires_at.map(|e| {
                let now = chrono::Utc::now();
                if e > now { (e - now).num_seconds() as u32 } else { 0 }
            }),
            progress: m.progress,
            can_accept: m.can_accept(player_id, None),
        })
        .collect();

    drop(target_sector);

    // Send initial state for new sector
    let _ = tx.send(ServerMessage::InitialState {
        player: player_dto,
        ship: ship_dto,
        sector: sector_dto,
        ships,
        missions,
    }).await;

    // Notify other players in new sector of new ship spawn
    if let Some(new_sector) = state.sectors.get(&target_sector_id) {
        let ship = state.ships.get(&ship_id);
        if let Some(ship) = ship {
            let spawn_dto = ShipDto {
                id: ship.id,
                name: ship.name.clone(),
                owner_id: ship.owner_id,
                ship_class: format!("{:?}", ship.ship_class),
                position: PositionDto {
                    x: ship.position.x,
                    y: ship.position.y,
                    z: ship.position.z,
                },
                hull_percent: ship.hull_integrity,
                shield_percent: ship.shield_strength,
                status: format!("{:?}", ship.status),
                faction_tag: ship.faction_id.and_then(|fid| {
                    state.factions.get(&fid).map(|f| f.tag.clone())
                }),
                is_player: true,
                is_hostile: false,
            };

            let spawn_msg = ServerMessage::StateUpdate {
                tick: state.get_tick(),
                ship_updates: vec![],
                ship_spawns: vec![spawn_dto],
                ship_despawns: vec![],
                mission_updates: vec![],
                events: vec![],
            };

            // Broadcast to all except the joining player
            for conn in new_sector.connections.iter() {
                if *conn.key() != player_id {
                    let _ = conn.value().send(spawn_msg.clone()).await;
                }
            }
        }
    }

    tracing::info!("Player {} joined sector {}", player_id, target_sector_id);
}

/// Handle leaving the current sector.
async fn handle_leave_sector(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    sector_id: Uuid,
) {
    // Get player session
    let ship_id = match state.players.get(&player_id) {
        Some(s) => s.ship_id,
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "SESSION_NOT_FOUND".to_string(),
                message: "Session not found".to_string(),
            }).await;
            return;
        }
    };

    // Remove from sector connections and ship list
    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.connections.remove(&player_id);
        sector.ship_ids.remove(&ship_id);

        // Notify other players of departure
        let despawn_msg = ServerMessage::StateUpdate {
            tick: state.get_tick(),
            ship_updates: vec![],
            ship_spawns: vec![],
            ship_despawns: vec![ship_id],
            mission_updates: vec![],
            events: vec![],
        };
        sector.broadcast(despawn_msg).await;
    }

    tracing::info!("Player {} left sector {}", player_id, sector_id);
}
