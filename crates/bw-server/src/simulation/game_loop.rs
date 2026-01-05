//! Main game loop
//!
//! Runs at 10 TPS (100ms per tick).

use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use bw_core::models::{MissionStatus, MissionType, Mission, ShipStatus};
use bw_scripting::EntityType;
use bw_shared::{ServerMessage, dto::*, FAME_DECAY_INTERVAL, FAME_DECAY_RATE, TICK_DURATION_MS, TICK_RATE};
use tokio::sync::watch;

use super::combat_processor::process_sector_combats;
use super::metrics::{SectorMetricsBuilder, TickMetricsBuilder};
use super::npc_spawner::{cleanup_npcs, try_spawn_npcs};
use super::script_hooks::ScriptHooks;
use super::sim_config::NpcSpawningConfig;
use crate::config::config;
use crate::GameState;

/// Run the main game loop with graceful shutdown support.
pub async fn run_game_loop(state: Arc<GameState>, shutdown: watch::Receiver<bool>) {
    let tick_duration = Duration::from_millis(TICK_DURATION_MS);
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

        // Initialize metrics builder
        let mut metrics = TickMetricsBuilder::new(tick);

        // === Scripting: Process coroutines ===
        metrics.start_phase("coroutines");
        {
            let result = state.coroutine_scheduler.write().tick(tick);
            for (id, error) in result.failed {
                state.log_script_error(format!("Coroutine {} failed: {}", id, error));
            }
        }
        metrics.end_phase();

        // === Scripting: Update all behaviors ===
        metrics.start_phase("behaviors");
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
        metrics.end_phase();

        // Process each sector (with per-sector metrics)
        // Get simulation config from ConfigManager (supports hot-reload)
        let sim_config = config().get();
        let npc_config = &sim_config.simulation.npc_spawning;

        metrics.start_phase("sectors_total");
        for sector_ref in state.sectors.iter() {
            let sector_id = *sector_ref.key();
            let sector_name = sector_ref.sector.name.clone();
            let sector_timing = process_sector_tick_with_metrics(
                &state,
                sector_id,
                sector_name,
                tick,
                npc_config,
            )
            .await;
            metrics.add_sector_timing(sector_timing);
        }
        metrics.end_phase();

        // Global tick processing
        metrics.start_phase("global");
        process_global_tick(&state, tick).await;
        metrics.end_phase();

        // Persist dirty entities (automatic change detection)
        metrics.start_phase("persistence");
        persist_dirty_entities(&state);
        metrics.end_phase();

        // Finalize and store metrics
        let tick_metrics = metrics.finish();

        // Log warning if over budget (keep existing behavior)
        if tick_metrics.over_budget {
            tracing::warn!(
                "Tick {} took {}us (over budget by {}us)",
                tick,
                tick_metrics.total_us,
                tick_metrics.total_us.saturating_sub(tick_metrics.budget_us)
            );
        }

        // Store metrics
        state.metrics.record(tick_metrics.clone());

        // Send to subscribers
        broadcast_metrics(&state, tick_metrics).await;

        // Sleep for remainder of tick
        let elapsed = tick_start.elapsed();
        if elapsed < tick_duration {
            tokio::time::sleep(tick_duration - elapsed).await;
        }
    }
}

/// Broadcast metrics to subscribed players.
async fn broadcast_metrics(state: &GameState, metrics: TickMetricsDto) {
    let subscribers = state.metrics.get_subscribers();
    if subscribers.is_empty() {
        return;
    }

    let tick = metrics.tick;
    let msg = ServerMessage::TickMetrics(metrics);

    // Also send history every 10 ticks (1 second) for updated stats
    let history_msg = if tick % 10 == 0 {
        Some(ServerMessage::TickMetricsHistory(state.metrics.get_history()))
    } else {
        None
    };

    for player_id in subscribers {
        if let Some(session) = state.players.get(&player_id) {
            if let Some(ref conn) = session.connection {
                let _ = conn.send(msg.clone()).await;
                if let Some(ref hist) = history_msg {
                    let _ = conn.send(hist.clone()).await;
                }
            }
        }
    }
}

/// Persist all dirty entities to the database.
///
/// This is called once per tick after all mutations have been applied.
/// Only entities that have actually changed (detected via hash comparison) are persisted.
fn persist_dirty_entities(state: &GameState) {
    // Persist dirty ships (only player ships)
    for ship_id in state.ships.drain_dirty() {
        if let Some(ship) = state.ships.get(&ship_id) {
            if ship.is_player_ship {
                state.persist.persist_ship((*ship).clone());
            }
        }
    }

    // Persist dirty players
    for player_id in state.player_data.drain_dirty() {
        if let Some(player) = state.player_data.get(&player_id) {
            state.persist.persist_player((*player).clone());
        }
    }

    // Persist dirty squadrons
    for squadron_id in state.squadrons.drain_dirty() {
        if let Some(squadron) = state.squadrons.get(&squadron_id) {
            state.persist.persist_squadron((*squadron).clone());
        }
    }
}

/// Process global game state (fame decay, session cleanup, etc).
async fn process_global_tick(state: &GameState, tick: u64) {
    // Session cleanup every 600 ticks (1 minute at 10 TPS)
    if tick % 600 == 0 {
        match state.db.sessions().delete_expired().await {
            Ok(count) if count > 0 => {
                tracing::info!("Cleaned up {} expired sessions", count);
            }
            Err(e) => {
                tracing::warn!("Failed to cleanup expired sessions: {}", e);
            }
            _ => {}
        }
    }

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
            // Decay fame (persistence is automatic via dirty tracking)
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

/// Process a sector tick with detailed timing metrics.
async fn process_sector_tick_with_metrics(
    state: &GameState,
    sector_id: Uuid,
    sector_name: String,
    tick: u64,
    npc_config: &NpcSpawningConfig,
) -> SectorTimingDto {
    let mut sector_metrics = SectorMetricsBuilder::new(sector_id, sector_name);

    let sector = match state.sectors.get(&sector_id) {
        Some(s) => s,
        None => return sector_metrics.finish(),
    };

    let mut ship_updates = Vec::new();
    let mut ship_spawns = Vec::new();
    let mut ship_despawns = Vec::new();
    let mut mission_updates = Vec::new();
    let mut events = Vec::new();

    // === Movement ===
    sector_metrics.start_phase("movement");
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
                // Persistence is automatic via TrackedDashMap dirty tracking
            }
        }
    }
    sector_metrics.end_phase();

    // === Combat ===
    sector_metrics.start_phase("combat");
    let combat_result = process_sector_combats(state, &sector, tick, &state.scripts);

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
    sector_metrics.end_phase();

    // === NPC Spawning (every 50 ticks) ===
    if tick % 50 == 0 {
        sector_metrics.start_phase("npc_spawn");
        let hooks = ScriptHooks::new(state.scripts.clone());
        let spawn_result = try_spawn_npcs(state, &sector, tick, npc_config, Some(&hooks));
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
        sector_metrics.end_phase();
    } else {
        sector_metrics.record_skipped("npc_spawn");
    }

    // === NPC Cleanup ===
    sector_metrics.start_phase("npc_cleanup");
    let despawned = cleanup_npcs(state, &sector, npc_config.despawn_distance);
    for &ship_id in &despawned {
        // Detach any behaviors attached to this ship
        state.behavior_manager.write().detach_for_entity(ship_id);
    }
    ship_despawns.extend(despawned);
    sector_metrics.end_phase();

    // === Mission Spawning (every 100 ticks) ===
    if tick % 100 == 0 && sector.missions.len() < 5 {
        sector_metrics.start_phase("mission_spawn");
        maybe_spawn_mission(state, &sector, tick).await;
        sector_metrics.end_phase();
    } else {
        sector_metrics.record_skipped("mission_spawn");
    }

    // === Mission Expiration ===
    sector_metrics.start_phase("mission_expire");
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
    sector_metrics.end_phase();

    // === Broadcasting ===
    sector_metrics.start_phase("broadcast");
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
    sector_metrics.end_phase();

    sector_metrics.finish()
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
