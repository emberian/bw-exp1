//! Combat Processing System
//!
//! Processes active combat engagements each tick.
//! Configuration is loaded from config.toml [simulation.combat].
//! Combat logic (target selection, damage calc) is driven by Rhai scripts.

use std::sync::Arc;

use rhai::Dynamic;
use uuid::Uuid;

use bw_core::models::ShipStatus;
use bw_core::systems::{CombatEngagement, CombatLogEntry, AttackResult};
use bw_scripting::ScriptEngine;
use bw_shared::dto::CombatEventDto;
use bw_shared::ServerMessage;

use crate::config::config;
use crate::{GameState, SectorInstance};
use super::script_hooks::ScriptHooks;

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
    scripts: &Arc<ScriptEngine>,
) -> CombatTickResult {
    let mut result = CombatTickResult::default();
    let hooks = ScriptHooks::new(scripts.clone());

    // Process each active combat
    let combat_ids: Vec<Uuid> = sector.combats.iter().map(|e| *e.key()).collect();

    for combat_id in combat_ids {
        if let Some(mut combat) = sector.combats.get_mut(&combat_id) {
            if combat.is_resolved {
                continue;
            }

            let combat_result = process_combat_round(state, &mut combat, tick, &hooks);

            // Collect updates for participants (persistence is automatic via dirty tracking)
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
            // Persistence is automatic via TrackedDashMap dirty tracking
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
    hooks: &ScriptHooks,
) -> CombatRoundResult {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut events = Vec::new();
    let mut destroyed = Vec::new();

    combat.round += 1;

    // Build list of all attackers with their target pools
    // Target selection is done via script
    let mut attack_pairs: Vec<(Uuid, Uuid)> = Vec::new();

    // Side A picks targets from Side B
    for &attacker_id in &combat.side_a {
        if combat.side_b.is_empty() {
            continue;
        }
        if let Some(target_id) = script_select_target(hooks, state, attacker_id, &combat.side_b, combat.round) {
            attack_pairs.push((attacker_id, target_id));
        }
    }

    // Side B picks targets from Side A
    for &attacker_id in &combat.side_b {
        if combat.side_a.is_empty() {
            continue;
        }
        if let Some(target_id) = script_select_target(hooks, state, attacker_id, &combat.side_a, combat.round) {
            attack_pairs.push((attacker_id, target_id));
        }
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

        if let Some(event) = resolve_attack(state, attacker_id, target_id, combat, &mut destroyed, hooks) {
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
    hooks: &ScriptHooks,
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
    let attacker_stance = format!("{:?}", attacker.combat_stance).to_lowercase();
    let defender_stance = format!("{:?}", target.combat_stance).to_lowercase();

    // Use first weapon
    let weapon = attacker.weapons.first()?;
    let weapon_damage = weapon.damage_base;
    let weapon_accuracy = weapon.accuracy_base;
    let weapon_type = weapon.weapon_type;
    let ammo_cost = weapon.ammo_cost;

    // Calculate attack via script
    let result = script_calculate_attack(
        hooks,
        &attacker_stats,
        &defender_stats,
        weapon_damage,
        weapon_accuracy,
        &attacker_stance,
        &defender_stance,
    )?;

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
    scripts: &Arc<ScriptEngine>,
) -> bool {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let hooks = ScriptHooks::new(scripts.clone());

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

    // Calculate flee chance via script
    let flee_chance = ship.as_ref().and_then(|s| {
        let stats = s.combat_effectiveness();
        script_calculate_flee_chance(&hooks, stats.speed)
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

// =============================================================================
// Script Integration
// =============================================================================

use bw_core::models::CombatStats;

/// Call script to select a target from available enemies.
fn script_select_target(
    hooks: &ScriptHooks,
    state: &GameState,
    attacker_id: Uuid,
    targets: &[Uuid],
    combat_round: u32,
) -> Option<Uuid> {
    // Build attacker data
    let attacker = state.ships.get(&attacker_id)?;
    let attacker_stats = attacker.combat_effectiveness();

    let mut attacker_map = rhai::Map::new();
    attacker_map.insert("id".into(), Dynamic::from(attacker_id.to_string()));
    attacker_map.insert("name".into(), Dynamic::from(attacker.name.clone()));
    attacker_map.insert("hull".into(), Dynamic::from(attacker.hull_integrity as f64));
    attacker_map.insert("shields".into(), Dynamic::from(attacker.shield_strength as f64));
    attacker_map.insert("speed".into(), Dynamic::from(attacker_stats.speed as f64));
    attacker_map.insert("is_player_ship".into(), Dynamic::from(attacker.is_player_ship));
    attacker_map.insert("locked_target".into(), Dynamic::from(
        attacker.locked_target.map(|id| id.to_string()).unwrap_or_default()
    ));
    attacker_map.insert("combat_stance".into(), Dynamic::from(format!("{:?}", attacker.combat_stance).to_lowercase()));
    drop(attacker);

    // Build targets array
    let mut targets_arr: Vec<Dynamic> = Vec::new();
    for &target_id in targets {
        if let Some(target) = state.ships.get(&target_id) {
            let target_stats = target.combat_effectiveness();
            let mut target_map = rhai::Map::new();
            target_map.insert("id".into(), Dynamic::from(target_id.to_string()));
            target_map.insert("name".into(), Dynamic::from(target.name.clone()));
            target_map.insert("hull".into(), Dynamic::from(target.hull_integrity as f64));
            target_map.insert("shields".into(), Dynamic::from(target.shield_strength as f64));
            target_map.insert("speed".into(), Dynamic::from(target_stats.speed as f64));
            targets_arr.push(Dynamic::from(target_map));
        }
    }

    // Build context
    let mut ctx = rhai::Map::new();
    ctx.insert("attacker".into(), Dynamic::from(attacker_map));
    ctx.insert("targets".into(), Dynamic::from(targets_arr));
    ctx.insert("combat_round".into(), Dynamic::from(combat_round as i64));

    // Call script
    let result = hooks.try_call("combat/rules.rhai", "select_target", ctx.into())?;

    // Parse result (should be target UUID string or unit)
    if result.is_unit() {
        return None;
    }

    let target_str = result.into_string().ok()?;
    Uuid::parse_str(&target_str).ok()
}

/// Call script to calculate attack damage/hit/crit.
fn script_calculate_attack(
    hooks: &ScriptHooks,
    attacker_stats: &CombatStats,
    defender_stats: &CombatStats,
    weapon_damage: f32,
    weapon_accuracy: f32,
    attacker_stance: &str,
    defender_stance: &str,
) -> Option<AttackResult> {
    // Build context
    let mut ctx = rhai::Map::new();

    let mut attacker_map = rhai::Map::new();
    attacker_map.insert("attack".into(), Dynamic::from(attacker_stats.attack as f64));
    attacker_map.insert("defense".into(), Dynamic::from(attacker_stats.defense as f64));
    attacker_map.insert("speed".into(), Dynamic::from(attacker_stats.speed as f64));
    attacker_map.insert("combat_stance".into(), Dynamic::from(attacker_stance.to_string()));

    let mut defender_map = rhai::Map::new();
    defender_map.insert("attack".into(), Dynamic::from(defender_stats.attack as f64));
    defender_map.insert("defense".into(), Dynamic::from(defender_stats.defense as f64));
    defender_map.insert("speed".into(), Dynamic::from(defender_stats.speed as f64));
    defender_map.insert("combat_stance".into(), Dynamic::from(defender_stance.to_string()));

    ctx.insert("attacker".into(), Dynamic::from(attacker_map));
    ctx.insert("defender".into(), Dynamic::from(defender_map));
    ctx.insert("weapon_damage".into(), Dynamic::from(weapon_damage as f64));
    ctx.insert("weapon_accuracy".into(), Dynamic::from(weapon_accuracy as f64));

    // Call script
    let result = hooks.try_call("combat/rules.rhai", "calculate_attack", ctx.into())?;

    // Parse result map
    let result_map = result.try_cast::<rhai::Map>()?;

    let hit = result_map.get("hit")?.as_bool().ok().unwrap_or(false);
    let damage = result_map.get("damage")?.as_float().ok().unwrap_or(0.0) as f32;
    let critical_hit = result_map.get("critical_hit").and_then(|v| v.as_bool().ok()).unwrap_or(false);
    let ammo_consumed = result_map.get("ammo_consumed").and_then(|v| v.as_float().ok()).unwrap_or(1.0) as f32;

    Some(AttackResult {
        hit,
        damage,
        ammo_consumed,
        critical_hit,
    })
}

/// Call script to calculate flee chance.
fn script_calculate_flee_chance(hooks: &ScriptHooks, speed: f32) -> Option<f32> {
    let combat_config = &config().get().simulation.combat;

    // Build context
    let mut ctx = rhai::Map::new();
    ctx.insert("speed".into(), Dynamic::from(speed as f64));
    ctx.insert("max_flee_chance".into(), Dynamic::from(combat_config.max_flee_chance as f64));
    ctx.insert("flee_speed_divisor".into(), Dynamic::from(combat_config.flee_speed_divisor as f64));

    // Call script
    let result = hooks.try_call("combat/rules.rhai", "calculate_flee_chance", ctx.into())?;

    Some(result.as_float().ok()? as f32)
}
