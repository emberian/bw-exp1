//! Combat Processing System
//!
//! Processes active combat engagements each tick.

use uuid::Uuid;

use bw_core::models::ShipStatus;
use bw_core::systems::{calculate_attack, CombatEngagement, CombatLogEntry};
use bw_shared::dto::CombatEventDto;
use bw_shared::ServerMessage;

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

    // Remove resolved combats
    for combat_id in &result.resolved {
        sector.combats.remove(combat_id);
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
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut events = Vec::new();
    let mut destroyed = Vec::new();

    combat.round += 1;

    // Side A attacks Side B
    let side_a_ships: Vec<Uuid> = combat.side_a.clone();
    let side_b_ships: Vec<Uuid> = combat.side_b.clone();

    for attacker_id in &side_a_ships {
        if side_b_ships.is_empty() {
            break;
        }

        // Select random target from side B
        let target_idx = rng.gen_range(0..side_b_ships.len());
        let target_id = side_b_ships[target_idx];

        if let Some(event) = resolve_attack(state, *attacker_id, target_id, combat, &mut destroyed) {
            events.push(event);
        }
    }

    // Side B attacks Side A
    let side_a_remaining: Vec<Uuid> = combat.side_a.clone();
    let side_b_remaining: Vec<Uuid> = combat.side_b.clone();

    for attacker_id in &side_b_remaining {
        if side_a_remaining.is_empty() {
            break;
        }

        let target_idx = rng.gen_range(0..side_a_remaining.len());
        let target_id = side_a_remaining[target_idx];

        if let Some(event) = resolve_attack(state, *attacker_id, target_id, combat, &mut destroyed) {
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
            attacker.crew.grant_experience(1); // 1 XP per hit
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
                attacker.name, result.damage, target.name
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
                        ship.crew.grant_experience(10); // 10 XP per kill
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

    // Calculate flee chance based on speed
    let flee_chance = ship.as_ref().map(|s| {
        let stats = s.combat_effectiveness();
        (stats.speed / 100.0).min(0.7) // Max 70% flee chance
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
