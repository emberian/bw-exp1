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
use crate::GameState;

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
                            // Parse token
                            let parts: Vec<&str> = token.split(':').collect();
                            if parts.len() == 2 {
                                if let (Ok(pid), Ok(_sid)) = (
                                    Uuid::parse_str(parts[0]),
                                    Uuid::parse_str(parts[1]),
                                ) {
                                    if state.players.contains_key(&pid) {
                                        player_id = Some(pid);

                                        // Get sector from player session
                                        if let Some(session) = state.players.get(&pid) {
                                            sector_id = Some(session.sector_id);
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
                                }
                            }

                            let _ = tx.send(ServerMessage::AuthResult {
                                success: false,
                                player_id: None,
                                error: Some("Invalid token".to_string()),
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
                            if player_id.is_none() {
                                let _ = tx.send(ServerMessage::Error {
                                    code: "UNAUTHORIZED".to_string(),
                                    message: "Not authenticated".to_string(),
                                }).await;
                                continue;
                            }

                            // Handle game messages
                            handle_game_message(
                                &state,
                                &tx,
                                player_id.unwrap(),
                                sector_id.unwrap(),
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

    // Build DTOs
    let player_dto = PlayerDto {
        id: player_id,
        username,
        reputation,
        fame,
        faction_tag,
        squadron_tag,
        is_online: true,
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
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    sector_id: Uuid,
    msg: ClientMessage,
) {
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
            if let Some(session) = state.players.get(&player_id) {
                if let Some(mut ship) = state.ships.get_mut(&session.ship_id) {
                    use bw_core::models::{Position, ShipStatus};

                    // Check if can move
                    if !ship.can_move() {
                        let _ = tx.send(ServerMessage::Error {
                            code: "CANNOT_MOVE".to_string(),
                            message: "Ship cannot move in current state".to_string(),
                        }).await;
                        return;
                    }

                    let destination = Position::new(x, y, z);
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

                    if flee_from_combat(state, &sector, ship_id) {
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

        ClientMessage::JoinSector { .. } | ClientMessage::LeaveSector | ClientMessage::MoveToSector { .. } => {
            tracing::warn!("Multi-sector travel not yet implemented for player {}", player_id);
            let _ = tx.send(ServerMessage::Error {
                code: "NOT_IMPLEMENTED".to_string(),
                message: "Sector travel coming soon".to_string(),
            }).await;
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

        _ => {
            tracing::debug!("Unhandled message from player {}: {:?}", player_id, msg);
        }
    }
}
