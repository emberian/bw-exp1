//! Playtest simulation tick
//!
//! Runs a simplified game loop for playtest instances.

use std::sync::Arc;
use std::time::{Duration, Instant};

use uuid::Uuid;

use bw_core::models::{MissionStatus, ShipStatus};
use bw_shared::{ServerMessage, dto::*, TICK_DURATION_MS};

use super::instance::{PlaytestInstance, PlaytestSectorInstance};

/// Run the playtest simulation loop.
///
/// This runs independently of the main game loop, processing only
/// the playtest's forked state. The loop runs until the task is aborted
/// (when the playtest is destroyed).
pub async fn run_playtest_loop(playtest: Arc<PlaytestInstance>) {
    tracing::info!(
        playtest_id = %playtest.id,
        name = %playtest.name,
        "Playtest simulation loop started"
    );

    loop {
        // Check if paused
        if playtest.is_paused() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        let tick_start = Instant::now();
        let tick = playtest.increment_tick();

        // Apply time scale to delta time
        let time_scale = playtest.get_time_scale();
        let delta_time = (TICK_DURATION_MS as f64 / 1000.0) * time_scale as f64;

        // Update all entity behaviors (NPC AI scripts)
        {
            let results = playtest.behavior_manager.write().update_all(tick, delta_time);
            // Log any behavior errors
            for result in results.iter().filter(|r| !r.success) {
                if let Some(ref error) = result.error {
                    tracing::warn!(
                        playtest_id = %playtest.id,
                        behavior_id = %result.behavior_id,
                        error = %error,
                        "Playtest behavior update failed"
                    );
                }
            }
        }

        // Process each sector
        for sector_ref in playtest.sectors.iter() {
            let sector_id = *sector_ref.key();
            process_playtest_sector_tick(
                &playtest,
                sector_id,
                tick,
                delta_time,
            )
            .await;
        }

        // Adjust sleep time based on time_scale
        let target_duration = Duration::from_millis(
            (TICK_DURATION_MS as f32 / time_scale).max(10.0) as u64
        );

        let elapsed = tick_start.elapsed();
        if elapsed < target_duration {
            tokio::time::sleep(target_duration - elapsed).await;
        }
    }
}

/// Process a single sector tick for a playtest.
async fn process_playtest_sector_tick(
    playtest: &PlaytestInstance,
    sector_id: Uuid,
    tick: u64,
    delta_time: f64,
) {
    let sector = match playtest.sectors.get(&sector_id) {
        Some(s) => s,
        None => return,
    };

    let mut ship_updates = Vec::new();
    let mut ship_despawns = Vec::new();
    let mut mission_updates = Vec::new();
    let mut events = Vec::new();

    // === Movement Processing ===
    for ship_entry in sector.ship_ids.iter() {
        let ship_id = *ship_entry.key();

        if let Some(mut ship) = playtest.ships.get_mut(&ship_id) {
            if let ShipStatus::InTransit { destination, target_id: _ } = &ship.status {
                let dest = *destination;

                // Calculate movement
                let speed = ship.ship_class.base_stats().speed * ship.resources.fuel_movement_modifier();
                let distance_per_tick = speed as f64 * delta_time;

                let arrived = bw_game::systems::move_ship_towards(
                    &mut ship,
                    dest,
                    distance_per_tick,
                    delta_time,
                );

                // Consume fuel (scaled by time)
                let fuel_cost = ship.ship_class.base_stats().fuel_per_sector * 0.001 * delta_time as f32;
                ship.resources.consume_fuel(fuel_cost);

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

    // === Combat Processing ===
    process_playtest_combats(playtest, &sector, tick, &mut ship_updates, &mut ship_despawns, &mut events).await;

    // === Mission Expiration ===
    let expired_missions: Vec<Uuid> = sector.missions.iter()
        .filter(|m| m.is_expired() && matches!(m.status, MissionStatus::Available))
        .map(|m| m.id)
        .collect();

    for mission_id in expired_missions {
        if let Some(mut mission) = sector.missions.get_mut(&mission_id) {
            mission.status = MissionStatus::Expired;
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

    // === Broadcast Updates ===
    if !ship_updates.is_empty() || !events.is_empty() || !ship_despawns.is_empty() || !mission_updates.is_empty() {
        let update = ServerMessage::StateUpdate {
            tick,
            ship_updates,
            ship_spawns: Vec::new(),
            ship_despawns,
            mission_spawns: Vec::new(),
            mission_updates,
            events,
        };

        sector.broadcast(update).await;
    }
}

/// Combat action computed in read-only phase, applied later.
struct CombatAction {
    attacker_id: Uuid,
    attacker_name: String,
    target_id: Uuid,
    target_name: String,
    damage: f32,
}

/// Ships that need status updates after combat processing.
#[allow(dead_code)] // Destroyed variant kept for future expansion
enum CombatStatusChange {
    ExitCombat(Uuid),
    Destroyed(Uuid),
}

/// Process combat in a playtest sector (simplified version).
/// Uses two-phase approach to avoid deadlocks:
/// 1. Read phase: collect all combat actions
/// 2. Write phase: apply all changes
async fn process_playtest_combats(
    playtest: &PlaytestInstance,
    sector: &PlaytestSectorInstance,
    tick: u64,
    ship_updates: &mut Vec<ShipUpdateDto>,
    ship_despawns: &mut Vec<Uuid>,
    events: &mut Vec<GameEventDto>,
) {
    // Only process damage every 10 ticks
    if tick % 10 != 0 {
        return;
    }

    // === READ PHASE: Collect combat actions without holding mutable refs ===
    let mut actions: Vec<CombatAction> = Vec::new();
    let mut status_changes: Vec<CombatStatusChange> = Vec::new();

    // Find ships in combat and their targets
    let combat_data: Vec<(Uuid, Option<Uuid>, String, f32)> = sector.ship_ids.iter()
        .filter_map(|entry| {
            let ship_id = *entry.key();
            playtest.ships.get(&ship_id).and_then(|ship| {
                if matches!(ship.status, ShipStatus::InCombat { .. }) {
                    Some((ship_id, ship.locked_target, ship.name.clone(), ship.ship_class.base_stats().attack))
                } else {
                    None
                }
            })
        })
        .collect();

    // Determine actions for each combat ship
    for (ship_id, locked_target, attacker_name, damage) in combat_data {
        let Some(target_id) = locked_target else {
            // No target, mark for exit combat
            status_changes.push(CombatStatusChange::ExitCombat(ship_id));
            continue;
        };

        // Check if target exists and is in same sector
        let target_valid = playtest.ships.get(&target_id)
            .map(|t| t.sector_id == sector.sector.id)
            .unwrap_or(false);

        if !target_valid {
            // Target gone or in different sector
            status_changes.push(CombatStatusChange::ExitCombat(ship_id));
            continue;
        }

        // Get target name for the action
        let target_name = playtest.ships.get(&target_id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        actions.push(CombatAction {
            attacker_id: ship_id,
            attacker_name,
            target_id,
            target_name,
            damage,
        });
    }

    // === WRITE PHASE: Apply all changes ===

    // Apply status changes (exit combat)
    for change in status_changes {
        match change {
            CombatStatusChange::ExitCombat(ship_id) => {
                if let Some(mut ship) = playtest.ships.get_mut(&ship_id) {
                    ship.status = ShipStatus::Idle;
                    ship.locked_target = None;
                    ship_updates.push(ShipUpdateDto {
                        id: ship_id,
                        position: None,
                        hull_percent: None,
                        shield_percent: None,
                        status: Some("Idle".to_string()),
                    });
                }
            }
            CombatStatusChange::Destroyed(_) => {
                // Handled below with damage
            }
        }
    }

    // Apply damage actions
    for action in actions {
        // Apply damage to target
        let (target_hull, target_shield) = {
            let Some(mut target) = playtest.ships.get_mut(&action.target_id) else {
                continue;
            };

            let shield_damage = action.damage.min(target.shield_strength);
            target.shield_strength -= shield_damage;
            let hull_damage = action.damage - shield_damage;
            target.hull_integrity -= hull_damage;

            (target.hull_integrity, target.shield_strength)
        }; // Mutable ref released here

        // Record the hit
        ship_updates.push(ShipUpdateDto {
            id: action.target_id,
            position: None,
            hull_percent: Some(target_hull),
            shield_percent: Some(target_shield),
            status: None,
        });

        events.push(GameEventDto {
            event_type: "combat_hit".to_string(),
            message: format!("Hit {} for {:.0} damage", action.target_name, action.damage),
            actor_name: Some(action.attacker_name.clone()),
            target_name: Some(action.target_name.clone()),
        });

        // Check for destruction
        if target_hull <= 0.0 {
            ship_despawns.push(action.target_id);
            playtest.ships.remove(&action.target_id);
            sector.ship_ids.remove(&action.target_id);

            // Track deletion if it was from live state
            if !playtest.created_ship_ids.contains_key(&action.target_id) {
                playtest.deleted_ship_ids.insert(action.target_id, ());
            }

            events.push(GameEventDto {
                event_type: "ship_destroyed".to_string(),
                message: format!("{} was destroyed", action.target_name),
                actor_name: Some(action.attacker_name),
                target_name: Some(action.target_name),
            });

            // Attacker exits combat after destroying target
            if let Some(mut attacker) = playtest.ships.get_mut(&action.attacker_id) {
                attacker.status = ShipStatus::Idle;
                attacker.locked_target = None;
            }
        }
    }
}

/// Spawn a task to run the playtest simulation loop.
///
/// Returns the JoinHandle which should be stored and aborted when the playtest is destroyed.
pub fn spawn_playtest_loop(playtest: Arc<PlaytestInstance>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_playtest_loop(playtest).await;
    })
}
