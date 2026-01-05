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

                                // Register connection
                                if let Some(mut session) = state.players.get_mut(&pid) {
                                    session.connection = Some(tx.clone());
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

    // Cleanup
    if let Some(pid) = player_id {
        if let Some(mut session) = state.players.get_mut(&pid) {
            session.connection = None;
        }

        // Remove from sector connections
        if let Some(sid) = sector_id {
            if let Some(sector) = state.sectors.get(&sid) {
                sector.connections.remove(&pid);
            }
        }

        // Unsubscribe from metrics
        state.metrics.unsubscribe(pid);
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
    use crate::simulation::{
        dock_at_station, undock, use_service, start_mission, process_mission_choice,
        apply_resource_changes, start_combat, flee_from_combat,
        create_squadron, invite_to_squadron, accept_squadron_invite, decline_squadron_invite,
        leave_squadron, process_squadron_action, get_squadron_info,
        accept_alliance, decline_alliance,
    };
    use bw_core::models::{StationService, Ship, ShipClass, Position};

    match msg {
        ClientMessage::MoveToPosition { x, y, z } => {
            use bw_core::models::{Position, ShipStatus};

            let destination = Position::new(x, y, z);

            // Validate position is within sector bounds
            let bounds_valid = state.sectors.get(&sector_id)
                .map(|s| s.sector.contains(&destination))
                .unwrap_or(false);

            if !bounds_valid {
                let _ = tx.send(ServerMessage::Error {
                    code: "INVALID_POSITION".to_string(),
                    message: "Destination is outside sector bounds".to_string(),
                }).await;
                return;
            }

            if let Some(session) = state.players.get(&player_id) {
                if let Some(mut ship) = state.ships.get_mut(&session.ship_id) {
                    // Check if can move
                    if !ship.can_move() {
                        let _ = tx.send(ServerMessage::Error {
                            code: "CANNOT_MOVE".to_string(),
                            message: "Ship cannot move in current state".to_string(),
                        }).await;
                        return;
                    }

                    ship.status = ShipStatus::InTransit {
                        destination,
                        target_id: None,
                    };

                    tracing::debug!("Player {} moving to ({}, {}, {})", player_id, x, y, z);
                }
            }
        }

        ClientMessage::StopMovement => {
            if let Some(session) = state.players.get(&player_id) {
                if let Some(mut ship) = state.ships.get_mut(&session.ship_id) {
                    use bw_core::models::ShipStatus;
                    if matches!(ship.status, ShipStatus::InTransit { .. }) {
                        ship.status = ShipStatus::Idle;
                    }
                }
            }
        }

        ClientMessage::MoveToLocation { location_id } => {
            if let Some(sector) = state.sectors.get(&sector_id) {
                // Find the location in the sector
                if let Some(location) = sector.sector.locations.iter().find(|l| l.id == location_id) {
                    let destination = location.position;

                    if let Some(session) = state.players.get(&player_id) {
                        if let Some(mut ship) = state.ships.get_mut(&session.ship_id) {
                            use bw_core::models::ShipStatus;

                            if !ship.can_move() {
                                let _ = tx.send(ServerMessage::Error {
                                    code: "CANNOT_MOVE".to_string(),
                                    message: "Ship cannot move in current state".to_string(),
                                }).await;
                                return;
                            }

                            ship.status = ShipStatus::InTransit {
                                destination,
                                target_id: Some(location_id),
                            };

                            tracing::debug!("Player {} moving to location {}", player_id, location_id);
                        }
                    }
                } else {
                    let _ = tx.send(ServerMessage::Error {
                        code: "LOCATION_NOT_FOUND".to_string(),
                        message: "Location not found in this sector".to_string(),
                    }).await;
                }
            }
        }

        ClientMessage::DockAtStation { station_id } => {
            match dock_at_station(state, player_id, station_id) {
                Ok(msg) => {
                    let _ = tx.send(ServerMessage::ChatMessage {
                        sender_id: player_id,
                        sender_name: "System".to_string(),
                        message: msg,
                        channel: bw_shared::ChatChannel::System,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    }).await;
                }
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error {
                        code: "DOCK_FAILED".to_string(),
                        message: e,
                    }).await;
                }
            }
        }

        ClientMessage::Undock => {
            match undock(state, player_id) {
                Ok(msg) => {
                    let _ = tx.send(ServerMessage::ChatMessage {
                        sender_id: player_id,
                        sender_name: "System".to_string(),
                        message: msg,
                        channel: bw_shared::ChatChannel::System,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    }).await;
                }
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error {
                        code: "UNDOCK_FAILED".to_string(),
                        message: e,
                    }).await;
                }
            }
        }

        ClientMessage::UseService { service } => {
            // Parse service string to enum
            let service_enum = match service.to_uppercase().as_str() {
                "REFUEL" => StationService::Refuel,
                "REARM" => StationService::Rearm,
                "REPAIR" => StationService::Repair,
                "SHORELEAVE" | "SHORE_LEAVE" => StationService::ShoreLeave,
                "TRADE" => StationService::Trade,
                "UPGRADE" => StationService::ShipUpgrade,
                "MISSIONBOARD" | "MISSION_BOARD" => StationService::MissionBoard,
                _ => {
                    let _ = tx.send(ServerMessage::Error {
                        code: "INVALID_SERVICE".to_string(),
                        message: format!("Unknown service: {}", service),
                    }).await;
                    return;
                }
            };

            let result = use_service(state, player_id, service_enum);

            if result.success {
                let _ = tx.send(ServerMessage::ChatMessage {
                    sender_id: player_id,
                    sender_name: "System".to_string(),
                    message: result.message,
                    channel: bw_shared::ChatChannel::System,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                }).await;

                if let Some(update) = result.resource_update {
                    let _ = tx.send(update).await;
                }
            } else {
                let _ = tx.send(ServerMessage::Error {
                    code: "SERVICE_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::AcceptMission { mission_id } => {
            match start_mission(state, player_id, mission_id) {
                Ok(result) => {
                    // Apply any immediate resource changes first
                    if let Some(update) = apply_resource_changes(state, player_id, &result) {
                        let _ = tx.send(update).await;
                    }

                    // Send mission choice
                    let _ = tx.send(ServerMessage::MissionChoice {
                        mission_id,
                        description: result.narrative,
                        choices: result.choices,
                    }).await;

                    tracing::info!("Player {} started mission {}", player_id, mission_id);
                }
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error {
                        code: "MISSION_FAILED".to_string(),
                        message: e,
                    }).await;
                }
            }
        }

        ClientMessage::MakeMissionChoice { mission_id, choice_id } => {
            match process_mission_choice(state, player_id, mission_id, &choice_id) {
                Ok(result) => {
                    // Apply resource changes first
                    if let Some(update) = apply_resource_changes(state, player_id, &result) {
                        let _ = tx.send(update).await;
                    }

                    // Handle combat spawn
                    if let Some(ref combat_spawn) = result.spawn_combat {
                        tracing::info!("Mission spawned combat: {} x{}", combat_spawn.enemy_type, combat_spawn.enemy_count);

                        // Map enemy type to ship class
                        let ship_class = match combat_spawn.enemy_type.to_lowercase().as_str() {
                            "pirate" | "raider" | "pirate_raider" => ShipClass::PirateRaider,
                            "frigate" | "pirate_frigate" => ShipClass::PirateFrigate,
                            "terrorist" | "bomber" | "terrorist_bomber" => ShipClass::TerroristBomber,
                            "drone" | "drone_swarm" => ShipClass::DroneSwarm,
                            "sera" | "sera_swarm" => ShipClass::SeraSwarm,
                            "sera_hunter" => ShipClass::SeraHunter,
                            _ => ShipClass::PirateRaider,
                        };

                        // Get player ship position for spawning nearby
                        if let Some(session) = state.players.get(&player_id) {
                            let player_ship_id = session.ship_id;
                            drop(session);

                            if let Some(player_ship) = state.ships.get(&player_ship_id) {
                                let player_pos = player_ship.position;
                                drop(player_ship);

                                // Spawn enemies
                                for i in 0..combat_spawn.enemy_count {
                                    // Offset spawn position slightly from player
                                    let offset_x = 50.0 + (i as f64 * 20.0);
                                    let offset_y = 50.0;
                                    let spawn_pos = Position::new(
                                        player_pos.x + offset_x,
                                        player_pos.y + offset_y,
                                        player_pos.z,
                                    );

                                    // Generate enemy name
                                    let enemy_name = format!("{} #{}", combat_spawn.enemy_name, i + 1);

                                    // Create enemy ship
                                    let enemy_ship = Ship::new_npc_ship(
                                        enemy_name,
                                        ship_class,
                                        sector_id,
                                        spawn_pos,
                                        None, // No faction for mission enemies
                                    );
                                    let enemy_ship_id = enemy_ship.id;

                                    // Add to state
                                    state.ships.insert(enemy_ship_id, enemy_ship);

                                    // Add to sector
                                    if let Some(sector) = state.sectors.get(&sector_id) {
                                        sector.ship_ids.insert(enemy_ship_id, ());

                                        // Start combat with first enemy
                                        if i == 0 {
                                            if let Some(engagement) = start_combat(state, &sector, player_ship_id, enemy_ship_id) {
                                                let _ = tx.send(ServerMessage::CombatUpdate {
                                                    engagement_id: engagement.id,
                                                    round: 0,
                                                    events: vec![],
                                                    is_resolved: false,
                                                    winner: None,
                                                }).await;
                                                tracing::info!("Mission combat started: engagement {}", engagement.id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if result.is_complete {
                        // Send completion
                        let _ = tx.send(ServerMessage::MissionResult {
                            mission_id,
                            success: result.success,
                            reputation_change: result.reputation_change,
                            fame_change: result.fame_change,
                            narrative: result.narrative,
                        }).await;
                    } else {
                        // Send next choice
                        let _ = tx.send(ServerMessage::MissionChoice {
                            mission_id,
                            description: result.narrative,
                            choices: result.choices,
                        }).await;
                    }
                }
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error {
                        code: "CHOICE_FAILED".to_string(),
                        message: e,
                    }).await;
                }
            }
        }

        ClientMessage::AbandonMission { mission_id } => {
            if let Some(sector) = state.sectors.get(&sector_id) {
                if let Some(mut mission) = sector.missions.get_mut(&mission_id) {
                    if mission.assigned_to == Some(player_id) {
                        mission.abandon();
                        let _ = tx.send(ServerMessage::MissionResult {
                            mission_id,
                            success: false,
                            reputation_change: -10, // Penalty
                            fame_change: 0,
                            narrative: "You abandoned the mission.".to_string(),
                        }).await;
                        tracing::info!("Player {} abandoned mission {}", player_id, mission_id);
                    }
                }
            }
        }

        ClientMessage::EngageTarget { target_id } => {
            if let Some(sector) = state.sectors.get(&sector_id) {
                // Verify target ship exists in the same sector
                if !sector.ship_ids.contains_key(&target_id) {
                    let _ = tx.send(ServerMessage::Error {
                        code: "TARGET_NOT_IN_SECTOR".to_string(),
                        message: "Target is not in your sector".to_string(),
                    }).await;
                    return;
                }

                if let Some(session) = state.players.get(&player_id) {
                    let ship_id = session.ship_id;
                    drop(session);

                    if let Some(engagement) = start_combat(state, &sector, ship_id, target_id) {
                        let _ = tx.send(ServerMessage::CombatUpdate {
                            engagement_id: engagement.id,
                            round: 0,
                            events: vec![],
                            is_resolved: false,
                            winner: None,
                        }).await;
                        tracing::info!("Player {} engaged target {}", player_id, target_id);
                    } else {
                        let _ = tx.send(ServerMessage::Error {
                            code: "ENGAGE_FAILED".to_string(),
                            message: "Could not engage target".to_string(),
                        }).await;
                    }
                }
            }
        }

        ClientMessage::DisengageCombat => {
            if let Some(sector) = state.sectors.get(&sector_id) {
                if let Some(session) = state.players.get(&player_id) {
                    let ship_id = session.ship_id;
                    drop(session);

                    if flee_from_combat(state, &sector, ship_id, &state.scripts) {
                        let _ = tx.send(ServerMessage::ChatMessage {
                            sender_id: player_id,
                            sender_name: "System".to_string(),
                            message: "Successfully disengaged from combat".to_string(),
                            channel: bw_shared::ChatChannel::System,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        }).await;
                    } else {
                        let _ = tx.send(ServerMessage::Error {
                            code: "FLEE_FAILED".to_string(),
                            message: "Failed to disengage from combat".to_string(),
                        }).await;
                    }
                }
            }
        }

        ClientMessage::FireWeapon { weapon_index, target_id } => {
            use bw_core::models::ShipStatus;

            if let Some(session) = state.players.get(&player_id) {
                let ship_id = session.ship_id;
                drop(session);

                // Check if player is in combat
                let ship = state.ships.get(&ship_id);
                let engagement_id = match ship.as_ref().and_then(|s| {
                    if let ShipStatus::InCombat { engagement_id } = s.status {
                        Some(engagement_id)
                    } else {
                        None
                    }
                }) {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerMessage::Error {
                            code: "NOT_IN_COMBAT".to_string(),
                            message: "Must be in combat to fire weapons".to_string(),
                        }).await;
                        return;
                    }
                };
                drop(ship);

                // Check weapon index is valid
                let weapon_count = state.ships.get(&ship_id)
                    .map(|s| s.weapons.len())
                    .unwrap_or(0);

                if weapon_index >= weapon_count {
                    let _ = tx.send(ServerMessage::Error {
                        code: "INVALID_WEAPON".to_string(),
                        message: "Invalid weapon index".to_string(),
                    }).await;
                    return;
                }

                // Verify target is in combat with us
                if let Some(sector) = state.sectors.get(&sector_id) {
                    if let Some(combat) = sector.combats.get(&engagement_id) {
                        if !combat.all_participants().contains(&target_id) {
                            let _ = tx.send(ServerMessage::Error {
                                code: "INVALID_TARGET".to_string(),
                                message: "Target is not in this combat".to_string(),
                            }).await;
                            return;
                        }
                    }
                }

                // Fire weapon is handled by combat system automatically
                // For manual fire, just log it (combat processor handles actual attacks)
                tracing::debug!("Player {} fired weapon {} at target {}", player_id, weapon_index, target_id);
            }
        }

        ClientMessage::JoinSector { sector_id: target_sector_id } => {
            // Direct sector join (usually after travel completes or for initial join)
            handle_join_sector(state, tx, player_id, sector_id, target_sector_id).await;
        }

        ClientMessage::LeaveSector => {
            // Leave current sector (disconnect from sector broadcast)
            handle_leave_sector(state, tx, player_id, sector_id).await;
        }

        ClientMessage::MoveToSector { sector_id: target_sector_id } => {
            // Initiate travel to another sector via jumpgate
            handle_move_to_sector(state, tx, player_id, sector_id, target_sector_id).await;
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
                    if let Some(sector) = state.sectors.get(&sector_id) {
                        sector.broadcast(chat_msg).await;
                    }
                }
                _ => {
                    let _ = tx.send(chat_msg).await;
                }
            }
        }

        ClientMessage::CreateSquadron { name, tag } => {
            let result = create_squadron(state, player_id, name, tag);
            let message = result.message.clone();

            let _ = tx.send(ServerMessage::SquadronUpdate {
                squadron: result.squadron,
                message,
            }).await;

            if !result.success {
                let _ = tx.send(ServerMessage::Error {
                    code: "SQUADRON_CREATE_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::InviteToSquadron { player_id: target_id } => {
            let result = invite_to_squadron(state, player_id, target_id);

            if result.success {
                // Notify inviter
                let _ = tx.send(ServerMessage::SquadronUpdate {
                    squadron: get_squadron_info(state, player_id),
                    message: result.message.clone(),
                }).await;

                // Send invitation to the target player if they're online
                if let Some(invite) = result.invite {
                    if let Some(session) = state.players.get(&target_id) {
                        if let Some(ref conn) = session.connection {
                            // Get squadron tag
                            let squadron_tag = state.squadrons.get(&invite.squadron_id)
                                .map(|s| s.tag.clone())
                                .unwrap_or_default();

                            let _ = conn.send(ServerMessage::SquadronInvite {
                                invite_id: invite.id,
                                squadron_id: invite.squadron_id,
                                squadron_name: invite.squadron_name.clone(),
                                squadron_tag,
                                inviter_name: invite.inviter_name.clone(),
                            }).await;
                        }
                    }
                }
            } else {
                let _ = tx.send(ServerMessage::Error {
                    code: "SQUADRON_INVITE_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::AcceptSquadronInvite { invite_id } => {
            let result = accept_squadron_invite(state, player_id, invite_id);

            if result.success {
                let _ = tx.send(ServerMessage::SquadronUpdate {
                    squadron: get_squadron_info(state, player_id),
                    message: result.message,
                }).await;
            } else {
                let _ = tx.send(ServerMessage::Error {
                    code: "INVITE_ACCEPT_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::DeclineSquadronInvite { invite_id } => {
            let result = decline_squadron_invite(state, player_id, invite_id);

            if result.success {
                let _ = tx.send(ServerMessage::SquadronUpdate {
                    squadron: None,
                    message: result.message,
                }).await;
            } else {
                let _ = tx.send(ServerMessage::Error {
                    code: "INVITE_DECLINE_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::AcceptAlliance { proposal_id } => {
            let result = accept_alliance(state, player_id, proposal_id);

            if result.success {
                let _ = tx.send(ServerMessage::SquadronUpdate {
                    squadron: get_squadron_info(state, player_id),
                    message: result.message,
                }).await;
            } else {
                let _ = tx.send(ServerMessage::Error {
                    code: "ALLIANCE_ACCEPT_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::DeclineAlliance { proposal_id } => {
            let result = decline_alliance(state, player_id, proposal_id);

            if result.success {
                let _ = tx.send(ServerMessage::SquadronUpdate {
                    squadron: get_squadron_info(state, player_id),
                    message: result.message,
                }).await;
            } else {
                let _ = tx.send(ServerMessage::Error {
                    code: "ALLIANCE_DECLINE_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::LeaveSquadron => {
            let result = leave_squadron(state, player_id);
            let message = result.message.clone();

            let _ = tx.send(ServerMessage::SquadronUpdate {
                squadron: None,
                message,
            }).await;

            if !result.success {
                let _ = tx.send(ServerMessage::Error {
                    code: "SQUADRON_LEAVE_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::SquadronAction { action } => {
            let result = process_squadron_action(state, player_id, action);
            let message = result.message.clone();

            let _ = tx.send(ServerMessage::SquadronUpdate {
                squadron: get_squadron_info(state, player_id),
                message,
            }).await;

            if !result.success {
                let _ = tx.send(ServerMessage::Error {
                    code: "SQUADRON_ACTION_FAILED".to_string(),
                    message: result.message,
                }).await;
            }
        }

        ClientMessage::Hail { target_id } => {
            // Get the sender's name
            let from_name = state.player_data.get(&player_id)
                .map(|p| p.username.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            // Find the target ship's owner and send them the hail
            if let Some(ship) = state.ships.get(&target_id) {
                if let Some(owner_id) = ship.owner_id {
                    // Find the owner's connection and send the hail notification
                    if let Some(session) = state.players.get(&owner_id) {
                        if let Some(ref conn) = session.connection {
                            let _ = conn.send(ServerMessage::HailReceived {
                                from_id: player_id,
                                from_name,
                            }).await;
                        }
                    }
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

/// Handle initiating travel to another sector.
async fn handle_move_to_sector(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    current_sector_id: Uuid,
    target_sector_id: Uuid,
) {
    use bw_core::models::{ShipStatus, LocationType, Position};

    // Don't allow traveling to the same sector
    if current_sector_id == target_sector_id {
        let _ = tx.send(ServerMessage::Error {
            code: "ALREADY_IN_SECTOR".to_string(),
            message: "You are already in this sector".to_string(),
        }).await;
        return;
    }

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

    // Verify target sector exists
    if !state.sectors.contains_key(&target_sector_id) {
        let _ = tx.send(ServerMessage::Error {
            code: "SECTOR_NOT_FOUND".to_string(),
            message: "Target sector not found".to_string(),
        }).await;
        return;
    }

    // Get current sector to check for jumpgate
    let current_sector = match state.sectors.get(&current_sector_id) {
        Some(s) => s,
        None => {
            let _ = tx.send(ServerMessage::Error {
                code: "SECTOR_NOT_FOUND".to_string(),
                message: "Current sector not found".to_string(),
            }).await;
            return;
        }
    };

    // Find jumpgate in current sector
    let jumpgate = current_sector.sector.locations.iter()
        .find(|loc| matches!(loc.location_type, LocationType::Jumpgate));

    if jumpgate.is_none() {
        let _ = tx.send(ServerMessage::Error {
            code: "NO_JUMPGATE".to_string(),
            message: "No jumpgate found in this sector".to_string(),
        }).await;
        return;
    }

    // Verify target sector is adjacent (connected via jumpgate)
    if !current_sector.sector.adjacent_sectors.contains(&target_sector_id) {
        let _ = tx.send(ServerMessage::Error {
            code: "SECTOR_NOT_ADJACENT".to_string(),
            message: "Target sector is not accessible from this jumpgate".to_string(),
        }).await;
        return;
    }

    drop(current_sector);

    // Get and update ship status
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

    // Check ship is not in combat or docked
    match &ship.status {
        ShipStatus::InCombat { .. } => {
            let _ = tx.send(ServerMessage::Error {
                code: "IN_COMBAT".to_string(),
                message: "Cannot travel while in combat".to_string(),
            }).await;
            return;
        }
        ShipStatus::Docked { .. } => {
            let _ = tx.send(ServerMessage::Error {
                code: "DOCKED".to_string(),
                message: "Undock before traveling to another sector".to_string(),
            }).await;
            return;
        }
        ShipStatus::InTransit { .. } => {
            let _ = tx.send(ServerMessage::Error {
                code: "ALREADY_TRAVELING".to_string(),
                message: "Already traveling".to_string(),
            }).await;
            return;
        }
        _ => {}
    }

    // Set ship status to in transit
    ship.status = ShipStatus::InTransit {
        destination: Position::new(0.0, 0.0, 0.0),
        target_id: Some(target_sector_id),
    };
    drop(ship);

    tracing::info!("Player {} initiating travel from {} to {}", player_id, current_sector_id, target_sector_id);

    // For now, travel is instant - just join the target sector
    // In the future, could add travel time based on distance
    handle_join_sector(state, tx, player_id, current_sector_id, target_sector_id).await;

    // Reset ship status after arrival
    if let Some(mut ship) = state.ships.get_mut(&ship_id) {
        ship.status = ShipStatus::Idle;
    }
}
