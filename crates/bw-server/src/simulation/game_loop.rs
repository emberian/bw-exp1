//! Main game loop
//!
//! Runs at 10 TPS (100ms per tick).

use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use bw_core::models::{ShipStatus, MissionType, Mission, MissionStatus};
use bw_shared::{ServerMessage, dto::*, TICK_RATE, TICK_DURATION_MS, FAME_DECAY_INTERVAL, FAME_DECAY_RATE};
use crate::GameState;
use tokio::sync::watch;

use super::npc_spawner::{try_spawn_npcs, cleanup_npcs, NpcSpawnConfig};
use bw_scripting::EntityType;
use super::combat_processor::process_sector_combats;

/// Run the main game loop with graceful shutdown support.
pub async fn run_game_loop(state: Arc<GameState>, shutdown: watch::Receiver<bool>) {
    let tick_duration = Duration::from_millis(TICK_DURATION_MS);
    let npc_config = NpcSpawnConfig::default();
    let delta_time = TICK_DURATION_MS as f64 / 1000.0; // 0.1 seconds

    tracing::info!("Game loop started at {} TPS", TICK_RATE);

    loop {
        // Check for shutdown signal
        if *shutdown.borrow() {
            tracing::info!("Game loop received shutdown signal, stopping...");
            break;
        }
        let tick_start = Instant::now();
        let tick = state.increment_tick();

        // === Scripting: Process coroutines ===
        {
            let result = state.coroutine_scheduler.write().tick(tick);
            for (id, error) in result.failed {
                state.log_script_error(format!("Coroutine {} failed: {}", id, error));
            }
        }

        // === Scripting: Update all behaviors ===
        {
            let results = state.behavior_manager.write().update_all(tick, delta_time);
            for result in results {
                if !result.success {
                    if let Some(error) = result.error {
                        state.log_script_error(format!(
                            "Behavior {} error: {}", result.behavior_id, error
                        ));
                    }
                }
            }
        }

        // Process each sector
        for sector_ref in state.sectors.iter() {
            let sector_id = *sector_ref.key();
            process_sector_tick(&state, sector_id, tick, &npc_config).await;
        }

        // Global tick processing
        process_global_tick(&state, tick).await;

        // Sleep for remainder of tick
        let elapsed = tick_start.elapsed();
        if elapsed < tick_duration {
            tokio::time::sleep(tick_duration - elapsed).await;
        } else {
            tracing::warn!("Tick {} took {:?} (over budget)", tick, elapsed);
        }
    }
}

/// Process global game state (fame decay, etc).
async fn process_global_tick(state: &GameState, tick: u64) {
    // Fame decay every FAME_DECAY_INTERVAL ticks
    if tick % FAME_DECAY_INTERVAL as u64 == 0 {
        // Only process online players with fame > 0
        // First collect player IDs that need processing to avoid holding locks
        let online_players_with_fame: Vec<(Uuid, Uuid)> = state.players.iter()
            .filter_map(|session| {
                // Only process if player has fame and is connected
                if session.connection.is_some() {
                    state.player_data.get(&session.player_id)
                        .filter(|p| p.resources.fame > 0)
                        .map(|_| (session.player_id, session.ship_id))
                } else {
                    None
                }
            })
            .collect();

        // Now process each player
        for (player_id, ship_id) in online_players_with_fame {
            // Decay fame
            let updated_resources = if let Some(mut player) = state.player_data.get_mut(&player_id) {
                player.resources.decay_fame(FAME_DECAY_RATE);
                Some((player.resources.reputation, player.resources.fame))
            } else {
                None
            };

            // Send update if we decayed fame
            if let Some((reputation, fame)) = updated_resources {
                if let Some(session) = state.players.get(&player_id) {
                    if let Some(ref conn) = session.connection {
                        if let Some(ship) = state.ships.get(&ship_id) {
                            let _ = conn.send(bw_shared::ServerMessage::ResourceUpdate {
                                reputation,
                                fame,
                                ammunition: ship.resources.ammunition,
                                fuel: ship.resources.fuel,
                                morale: ship.crew.morale,
                                experience: ship.crew.experience,
                            }).await;
                        }
                    }
                }
            }
        }
    }
}

async fn process_sector_tick(state: &GameState, sector_id: Uuid, tick: u64, npc_config: &NpcSpawnConfig) {
    let sector = match state.sectors.get(&sector_id) {
        Some(s) => s,
        None => return,
    };

    let mut ship_updates = Vec::new();
    let mut ship_spawns = Vec::new();
    let mut ship_despawns = Vec::new();
    let mut mission_updates = Vec::new();
    let mut events = Vec::new();

    // Process ship movement
    for ship_entry in sector.ship_ids.iter() {
        let ship_id = *ship_entry.key();

        if let Some(mut ship) = state.ships.get_mut(&ship_id) {
            if let ShipStatus::InTransit { destination, target_id } = &ship.status {
                let dest = *destination;
                let _target = *target_id;

                // Move ship towards destination
                let speed = ship.ship_class.base_stats().speed * ship.resources.fuel_movement_modifier();
                let distance_per_tick = speed as f64 * 0.1; // Adjust for tick rate

                let arrived = bw_core::systems::move_ship_towards(
                    &mut ship,
                    dest,
                    distance_per_tick,
                    0.1,
                );

                // Consume fuel
                let fuel_cost = ship.ship_class.base_stats().fuel_per_sector * 0.001;
                ship.resources.consume_fuel(fuel_cost);

                // Record update
                ship_updates.push(ShipUpdateDto {
                    id: ship_id,
                    position: Some(PositionDto {
                        x: ship.position.x,
                        y: ship.position.y,
                        z: ship.position.z,
                    }),
                    hull_percent: None,
                    shield_percent: None,
                    status: if arrived {
                        ship.status = ShipStatus::Idle;
                        Some("Idle".to_string())
                    } else {
                        None
                    },
                });
            }
        }
    }

    // Process active combats
    let combat_result = process_sector_combats(state, &sector, tick);

    // Send combat updates to participants
    for (player_id, msg) in combat_result.updates {
        if let Some(session) = state.players.get(&player_id) {
            if let Some(ref conn) = session.connection {
                let _ = conn.send(msg).await;
            }
        }
    }

    // Mark destroyed ships for despawn
    ship_despawns.extend(combat_result.destroyed_ships);

    // Spawn NPCs (every 50 ticks = 5 seconds)
    if tick % 50 == 0 {
        let spawn_result = try_spawn_npcs(state, &sector, tick, npc_config);
        ship_spawns.extend(spawn_result.ship_dtos);

        // Attach behaviors to newly spawned NPCs
        for (ship_id, sector_id, ship_class) in spawn_result.spawned_ships {
            // Select behavior script based on ship class
            let script_path = match ship_class {
                bw_core::models::ShipClass::PirateRaider |
                bw_core::models::ShipClass::PirateFrigate |
                bw_core::models::ShipClass::TerroristBomber |
                bw_core::models::ShipClass::SeraSwarm |
                bw_core::models::ShipClass::SeraHunter |
                bw_core::models::ShipClass::DroneSwarm |
                bw_core::models::ShipClass::DroneHarvester => "behaviors/npc_idle.rhai",
                _ => "behaviors/npc_idle.rhai", // Civilians also use idle behavior
            };

            // Try to load the script if not already loaded
            if !state.scripts.has_script(script_path) {
                if let Err(e) = state.scripts.load_script(script_path) {
                    tracing::warn!("Failed to load NPC behavior script {}: {}", script_path, e);
                    continue;
                }
            }

            // Attach behavior
            let result = state.behavior_manager.write().attach(
                ship_id,
                EntityType::Ship,
                script_path,
                Some(sector_id),
            );

            if let Err(e) = result {
                tracing::warn!("Failed to attach behavior to NPC {}: {}", ship_id, e);
            }
        }
    }

    // Cleanup destroyed NPCs
    let despawned = cleanup_npcs(state, &sector);
    for &ship_id in &despawned {
        // Detach any behaviors attached to this ship
        state.behavior_manager.write().detach_for_entity(ship_id);
    }
    ship_despawns.extend(despawned);

    // Spawn random missions (occasionally)
    if tick % 100 == 0 && sector.missions.len() < 5 {
        maybe_spawn_mission(state, &sector, tick).await;
    }

    // Check mission expirations
    let expired_missions: Vec<Uuid> = sector.missions.iter()
        .filter(|m| m.is_expired() && matches!(m.status, MissionStatus::Available))
        .map(|m| m.id)
        .collect();

    for mission_id in expired_missions {
        if let Some(mut mission) = sector.missions.get_mut(&mission_id) {
            mission.status = MissionStatus::Expired;
            // Broadcast mission status change
            mission_updates.push(MissionUpdateDto {
                id: mission_id,
                status: Some("Expired".to_string()),
                progress: None,
            });
            events.push(GameEventDto {
                event_type: "mission_expired".to_string(),
                message: format!("Mission '{}' has expired", mission.title),
                actor_name: None,
                target_name: None,
            });
        }
    }

    // Broadcast state update if there are any changes
    if !ship_updates.is_empty() || !events.is_empty() || !ship_spawns.is_empty() || !ship_despawns.is_empty() || !mission_updates.is_empty() {
        let update = ServerMessage::StateUpdate {
            tick,
            ship_updates,
            ship_spawns,
            ship_despawns,
            mission_updates,
            events,
        };

        sector.broadcast(update).await;
    }
}

async fn maybe_spawn_mission(_state: &GameState, sector: &crate::SectorInstance, _tick: u64) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // 10% chance per check (every 10 seconds)
    if rng.r#gen::<f32>() > 0.1 {
        return;
    }

    // Pick a random mission type
    let mission_types = [
        MissionType::PirateIntercept,
        MissionType::DistressSignal,
        MissionType::Smugglers,
    ];

    let mission_type = mission_types[rng.gen_range(0..mission_types.len())];

    // Create mission
    let mut mission = Mission::new(
        mission_type,
        match mission_type {
            MissionType::PirateIntercept => "Pirate Activity Detected".to_string(),
            MissionType::DistressSignal => "Distress Signal".to_string(),
            MissionType::Smugglers => "Suspicious Vessel".to_string(),
            _ => "Unknown Mission".to_string(),
        },
        sector.sector.id,
        mission_type.default_script_path().to_string(),
    );

    mission.description = match mission_type {
        MissionType::PirateIntercept => "A civilian freighter reports hostile contacts. Investigate and neutralize any threats.".to_string(),
        MissionType::DistressSignal => "An automated distress beacon is broadcasting nearby. The situation is unclear.".to_string(),
        MissionType::Smugglers => "A vessel is exhibiting unusual behavior. It may be worth investigation.".to_string(),
        _ => "Unknown".to_string(),
    };

    // Set random position in sector
    mission.target_position = Some(sector.sector.random_position());

    // Set expiry (5 minutes)
    mission.expires_at = Some(chrono::Utc::now() + chrono::Duration::minutes(5));

    tracing::info!("Spawned mission: {} in {}", mission.title, sector.sector.name);

    sector.missions.insert(mission.id, mission);
}
