//! Combat Processing System
//!
//! Processes active combat engagements each tick.
//! Configuration is loaded from config.toml [simulation.combat].

use uuid::Uuid;

use bw_core::models::ShipStatus;
use bw_core::systems::{calculate_attack, CombatEngagement, CombatLogEntry};
use bw_shared::dto::CombatEventDto;
use bw_shared::ServerMessage;

use crate::config::config;
use crate::{GameState, SectorInstance};

/// Result of processing combat for a tick.
#[derive(Debug, Default)]
pub struct CombatTickResult {
    /// Combat update messages to broadcast
    pub updates: Vec<(Uuid, ServerMessage)>, // (player_id, message)
    /// Engagements that resolved this tick
    pub resolved: Vec<Uuid>,
    /// Ships that were destroyed
    pub destroyed_ships: Vec<Uuid>,
}

/// Process all active combats in a sector for one tick.
pub fn process_sector_combats(
    state: &GameState,
    sector: &SectorInstance,
    tick: u64,
) -> CombatTickResult {
    let mut result = CombatTickResult::default();

    // Process each active combat
    let combat_ids: Vec<Uuid> = sector.combats.iter().map(|e| *e.key()).collect();

    for combat_id in combat_ids {
        if let Some(mut combat) = sector.combats.get_mut(&combat_id) {
            if combat.is_resolved {
                continue;
            }

            let combat_result = process_combat_round(state, &mut combat, tick);

            // Collect updates for participants
            for ship_id in combat.all_participants() {
                if let Some(ship) = state.ships.get(&ship_id) {
                    if ship.is_player_ship {
                        if let Some(owner_id) = ship.owner_id {
                            let msg = ServerMessage::CombatUpdate {
                                engagement_id: combat_id,
                                round: combat.round,
                                events: combat_result.events.clone(),
                                is_resolved: combat.is_resolved,
                                winner: combat.winner.map(|w| format!("{:?}", w)),
                            };
                            result.updates.push((owner_id, msg));
                        }
                    }
                }
            }

            result.destroyed_ships.extend(combat_result.destroyed);

            if combat.is_resolved {
                result.resolved.push(combat_id);
            }
        }
    }

    // Remove resolved combats and reset surviving ship statuses
    for combat_id in &result.resolved {
        // Get surviving participants before removing combat
        if let Some((_, combat)) = sector.combats.remove(combat_id) {
            // Reset status of all surviving ships to Idle
            for ship_id in combat.all_participants() {
                if let Some(mut ship) = state.ships.get_mut(&ship_id) {
                    if matches!(ship.status, ShipStatus::InCombat { .. }) {
                        ship.status = ShipStatus::Idle;
                    }
                }
            }
        }
    }

    result
}

/// Result of a single combat round.
#[derive(Debug)]
struct CombatRoundResult {
    events: Vec<CombatEventDto>,
    destroyed: Vec<Uuid>,
}

/// Process a single round of combat.
fn process_combat_round(
    state: &GameState,
    combat: &mut CombatEngagement,
    _tick: u64,
) -> CombatRoundResult {
    use rand::seq::SliceRandom;
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut events = Vec::new();
    let mut destroyed = Vec::new();

    combat.round += 1;

    // Build list of all attackers with their target pools
    // Each ship attacks one random enemy from the opposing side
    let mut attack_pairs: Vec<(Uuid, Uuid)> = Vec::new();

    // Side A picks targets from Side B
    for &attacker_id in &combat.side_a {
        if combat.side_b.is_empty() {
            continue;
        }
        let target_idx = rng.gen_range(0..combat.side_b.len());
        attack_pairs.push((attacker_id, combat.side_b[target_idx]));
    }

    // Side B picks targets from Side A
    for &attacker_id in &combat.side_b {
        if combat.side_a.is_empty() {
            continue;
        }
        let target_idx = rng.gen_range(0..combat.side_a.len());
        attack_pairs.push((attacker_id, combat.side_a[target_idx]));
    }

    // Shuffle attack order for fairness (no side always goes first)
    attack_pairs.shuffle(&mut rng);

    // Execute attacks in shuffled order, skipping if target already destroyed this round
    for (attacker_id, target_id) in attack_pairs {
        // Skip if target was already destroyed this round
        if destroyed.contains(&target_id) {
            continue;
        }

        // Skip if attacker was destroyed this round
        if destroyed.contains(&attacker_id) {
            continue;
        }

        if let Some(event) = resolve_attack(state, attacker_id, target_id, combat, &mut destroyed) {
            events.push(event);
        }
    }

    CombatRoundResult { events, destroyed }
}

/// Resolve a single attack between two ships.
fn resolve_attack(
    state: &GameState,
    attacker_id: Uuid,
    target_id: Uuid,
    combat: &mut CombatEngagement,
    destroyed: &mut Vec<Uuid>,
) -> Option<CombatEventDto> {
    // Guard against self-attack (would cause double mutable borrow panic)
    if attacker_id == target_id {
        return None;
    }

    let mut attacker = state.ships.get_mut(&attacker_id)?;
    let mut target = state.ships.get_mut(&target_id)?;

    // Check if attacker can attack
    if !attacker.can_attack() {
        return None;
    }

    // Get combat stats
    let attacker_stats = attacker.combat_effectiveness();
    let defender_stats = target.combat_effectiveness();

    // Use first weapon
    let weapon = attacker.weapons.first()?;
    let weapon_damage = weapon.damage_base;
    let weapon_accuracy = weapon.accuracy_base;
    let weapon_type = weapon.weapon_type;
    let ammo_cost = weapon.ammo_cost;

    // Calculate attack
    let result = calculate_attack(&attacker_stats, &defender_stats, weapon_damage, weapon_accuracy);

    // Consume ammo
    attacker.resources.consume_ammo(ammo_cost + result.ammo_consumed);

    // Apply damage
    let target_destroyed = if result.hit {
        target.apply_damage(result.damage);

        // Grant experience for successful hit (if attacker is player ship)
        if attacker.is_player_ship {
            let combat_config = &config().get().simulation.combat;
            attacker.crew.grant_experience(combat_config.xp_per_hit);
        }

        matches!(target.status, ShipStatus::Destroyed)
    } else {
        false
    };

    // Log entry
    let message = if result.hit {
        if result.critical_hit {
            format!(
                "CRITICAL HIT! {} deals {:.1} damage to {}!",
                attacker.name, result.damage, target.name
            )
        } else {
            format!(
                "{} hits {} for {:.1} damage.",
                attacker.name, target.name, result.damage
            )
        }
    } else {
        format!("{} misses {}.", attacker.name, target.name)
    };

    let log_entry = CombatLogEntry {
        round: combat.round,
        attacker_id,
        target_id,
        weapon_type,
        damage_dealt: result.damage,
        hit: result.hit,
        target_destroyed,
        message: message.clone(),
    };
    combat.log_event(log_entry);

    // Handle destruction
    if target_destroyed {
        destroyed.push(target_id);
        let target_name = target.name.clone();
        let target_is_player = target.is_player_ship;
        let target_owner_id = target.owner_id;
        let attacker_is_player = attacker.is_player_ship;
        let attacker_owner_id = attacker.owner_id;
        drop(target);
        drop(attacker);
        combat.remove_ship(target_id);
        tracing::info!("Ship {} destroyed in combat", target_name);

        // Update player stats
        // If attacker is player, increment ships_destroyed
        if attacker_is_player {
            if let Some(owner_id) = attacker_owner_id {
                if let Some(mut player) = state.player_data.get_mut(&owner_id) {
                    player.stats.ships_destroyed += 1;

                    // Grant experience to crew for killing enemy
                    if let Some(mut ship) = state.ships.get_mut(&attacker_id) {
                        let combat_config = &config().get().simulation.combat;
                        ship.crew.grant_experience(combat_config.xp_per_kill);
                    }
                }
            }
        }

        // If target is player, increment times_destroyed
        if target_is_player {
            if let Some(owner_id) = target_owner_id {
                if let Some(mut player) = state.player_data.get_mut(&owner_id) {
                    player.stats.times_destroyed += 1;
                }
            }
        }
    }

    Some(CombatEventDto {
        event_type: if target_destroyed {
            "destruction".to_string()
        } else if result.hit {
            "hit".to_string()
        } else {
            "miss".to_string()
        },
        attacker_name: state.ships.get(&attacker_id).map(|s| s.name.clone()).unwrap_or_default(),
        target_name: state.ships.get(&target_id).map(|s| s.name.clone()).unwrap_or_default(),
        damage: if result.hit { Some(result.damage) } else { None },
        hit: result.hit,
        message,
    })
}

/// Maximum engagement distance for combat.
const MAX_COMBAT_RANGE: f64 = 200.0;

/// Start a new combat engagement.
pub fn start_combat(
    state: &GameState,
    sector: &SectorInstance,
    initiator_id: Uuid,
    target_id: Uuid,
) -> Option<CombatEngagement> {
    // Get ships
    let initiator = state.ships.get(&initiator_id)?;
    let target = state.ships.get(&target_id)?;

    // Check distance - must be within combat range
    let distance = initiator.position.distance_to(&target.position);
    if distance > MAX_COMBAT_RANGE {
        tracing::debug!(
            "Combat initiation failed: distance {} exceeds max range {}",
            distance, MAX_COMBAT_RANGE
        );
        return None;
    }

    // Check if either is already in combat
    if matches!(initiator.status, ShipStatus::InCombat { .. }) {
        return None;
    }
    if matches!(target.status, ShipStatus::InCombat { .. }) {
        return None;
    }

    // Create engagement
    let engagement = CombatEngagement::new(
        sector.sector.id,
        vec![initiator_id],
        vec![target_id],
    );

    let engagement_id = engagement.id;

    // Update ship statuses
    drop(initiator);
    drop(target);

    if let Some(mut ship) = state.ships.get_mut(&initiator_id) {
        ship.status = ShipStatus::InCombat { engagement_id };
    }
    if let Some(mut ship) = state.ships.get_mut(&target_id) {
        ship.status = ShipStatus::InCombat { engagement_id };
    }

    // Add to sector
    sector.combats.insert(engagement_id, engagement.clone());

    tracing::info!("Combat started: {} vs {}", initiator_id, target_id);

    Some(engagement)
}

/// Handle a ship fleeing from combat.
pub fn flee_from_combat(
    state: &GameState,
    sector: &SectorInstance,
    ship_id: Uuid,
) -> bool {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Find the ship's combat
    let ship = state.ships.get(&ship_id);
    let engagement_id = match ship.as_ref().and_then(|s| {
        if let ShipStatus::InCombat { engagement_id } = s.status {
            Some(engagement_id)
        } else {
            None
        }
    }) {
        Some(id) => id,
        None => return false,
    };

    // Calculate flee chance based on speed (configurable)
    let combat_config = &config().get().simulation.combat;
    let flee_chance = ship.as_ref().map(|s| {
        let stats = s.combat_effectiveness();
        (stats.speed / combat_config.flee_speed_divisor).min(combat_config.max_flee_chance)
    }).unwrap_or(0.3);

    drop(ship);

    if rng.r#gen::<f32>() > flee_chance {
        return false; // Failed to flee
    }

    // Remove from combat
    if let Some(mut combat) = sector.combats.get_mut(&engagement_id) {
        combat.remove_ship(ship_id);
    }

    // Update ship status
    if let Some(mut ship) = state.ships.get_mut(&ship_id) {
        ship.status = ShipStatus::Idle;
    }

    tracing::debug!("Ship {} fled from combat", ship_id);
    true
}
