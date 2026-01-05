//! NPC Ship Spawning System
//!
//! Spawns NPC ships in sectors based on danger level and traffic density.
//! Configuration is loaded from config.toml [simulation.npc_spawning].
//! Complex logic (class selection, naming) can be overridden via Rhai scripts.

use uuid::Uuid;

use bw_core::models::{DangerLevel, Ship, ShipClass};
use bw_shared::dto::{PositionDto, ShipDto};
use rhai::Dynamic;

use crate::{GameState, SectorInstance};
use super::script_hooks::ScriptHooks;
use super::sim_config::NpcSpawningConfig;

/// Result of spawning NPCs, includes both DTOs for broadcast and IDs for behavior attachment.
pub struct SpawnResult {
    /// DTOs to broadcast to clients
    pub ship_dtos: Vec<ShipDto>,
    /// Ship IDs and their classes for behavior attachment
    pub spawned_ships: Vec<(Uuid, Uuid, ShipClass)>, // (ship_id, sector_id, class)
}

/// Try to spawn NPCs in a sector.
///
/// If `hooks` is provided, scripts can override class selection and naming.
pub fn try_spawn_npcs(
    state: &GameState,
    sector: &SectorInstance,
    _tick: u64,
    config: &NpcSpawningConfig,
    hooks: Option<&ScriptHooks>,
) -> SpawnResult {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut result = SpawnResult {
        ship_dtos: Vec::new(),
        spawned_ships: Vec::new(),
    };

    // Count current NPCs
    let npc_count = sector
        .ship_ids
        .iter()
        .filter(|entry| {
            state
                .ships
                .get(entry.key())
                .map(|s| !s.is_player_ship)
                .unwrap_or(false)
        })
        .count();

    // Check if we need more NPCs
    if npc_count >= config.max_npcs_per_sector {
        return result;
    }

    // Force spawn if below minimum
    let should_spawn = if npc_count < config.min_npcs_per_sector {
        true
    } else {
        // Adjust spawn chance by danger level (from config)
        let danger_mult = match sector.sector.danger_level {
            DangerLevel::Safe => config.danger_multipliers.safe,
            DangerLevel::Moderate => config.danger_multipliers.moderate,
            DangerLevel::Dangerous => config.danger_multipliers.dangerous,
            DangerLevel::Hostile => config.danger_multipliers.hostile,
        };

        let spawn_chance = config.base_spawn_chance * danger_mult;
        rng.r#gen::<f32>() < spawn_chance
    };

    if !should_spawn {
        return result;
    }

    // Determine NPC type via script
    let Some(hooks) = hooks else {
        tracing::warn!("NPC spawning requires ScriptHooks - skipping spawn");
        return result;
    };

    let Some((ship_class, faction_id)) = script_select_npc(hooks, state, sector) else {
        tracing::debug!("Script did not return valid NPC class - skipping spawn");
        return result;
    };

    // Generate position
    let position = sector.sector.random_position();

    // Create NPC name via script
    let name = script_generate_name(hooks, &ship_class)
        .unwrap_or_else(|| format!("{:?}-{}", ship_class, rng.gen_range(100..999)));

    // Create NPC ship
    let npc_ship = Ship::new_npc_ship(name.clone(), ship_class, sector.sector.id, position, faction_id);

    let ship_id = npc_ship.id;
    let sector_id = sector.sector.id;

    // Create DTO for broadcast (before inserting ship into state)
    let faction_tag = faction_id.and_then(|fid| state.factions.get(&fid).map(|f| f.tag.clone()));
    let ship_dto = ShipDto::from_core(&npc_ship, faction_tag);

    // Add to state
    state.ships.insert(ship_id, npc_ship);
    sector.ship_ids.insert(ship_id, ());

    tracing::debug!("Spawned NPC {} ({:?}) in {}", name, ship_class, sector.sector.name);

    result.ship_dtos.push(ship_dto);
    result.spawned_ships.push((ship_id, sector_id, ship_class));
    result
}

/// Despawn NPC ships that are destroyed or too far from any player.
pub fn cleanup_npcs(
    state: &GameState,
    sector: &SectorInstance,
    despawn_distance: f64,
) -> Vec<Uuid> {
    let mut despawned = Vec::new();

    // Collect player positions in this sector
    let player_positions: Vec<bw_core::models::Position> = sector
        .ship_ids
        .iter()
        .filter_map(|entry| {
            state.ships.get(entry.key()).and_then(|ship| {
                if ship.is_player_ship {
                    Some(ship.position)
                } else {
                    None
                }
            })
        })
        .collect();

    // Find NPCs to despawn (destroyed or too far from any player)
    let npcs_to_despawn: Vec<Uuid> = sector
        .ship_ids
        .iter()
        .filter_map(|entry| {
            let ship_id = *entry.key();
            state.ships.get(&ship_id).and_then(|ship| {
                if ship.is_player_ship {
                    return None;
                }

                // Always despawn destroyed ships
                if matches!(ship.status, bw_core::models::ShipStatus::Destroyed) {
                    return Some(ship_id);
                }

                // Never despawn ships that are in active combat
                if matches!(ship.status, bw_core::models::ShipStatus::InCombat { .. }) {
                    return None;
                }

                // If there are players in the sector, check distance
                if !player_positions.is_empty() {
                    let min_distance = player_positions
                        .iter()
                        .map(|p| ship.position.distance_to(p))
                        .fold(f64::MAX, f64::min);

                    // Despawn if too far from all players
                    if min_distance > despawn_distance {
                        return Some(ship_id);
                    }
                }

                None
            })
        })
        .collect();

    for ship_id in npcs_to_despawn {
        sector.ship_ids.remove(&ship_id);
        state.ships.remove(&ship_id);
        despawned.push(ship_id);
        tracing::debug!("Despawned NPC {}", ship_id);
    }

    despawned
}

/// Remove a single NPC ship from the game.
/// Used for cleanup when behavior attachment fails.
pub fn remove_npc_ship(state: &GameState, sector: &SectorInstance, ship_id: Uuid) {
    sector.ship_ids.remove(&ship_id);
    state.ships.remove(&ship_id);
    tracing::debug!("Removed NPC {} due to failed behavior attachment", ship_id);
}

// =============================================================================
// Script Integration
// =============================================================================

/// Call the script for NPC class/faction selection.
fn script_select_npc(
    hooks: &ScriptHooks,
    state: &GameState,
    sector: &SectorInstance,
) -> Option<(ShipClass, Option<Uuid>)> {
    // Build context for script
    let mut ctx = rhai::Map::new();
    ctx.insert("danger_level".into(), Dynamic::from(format!("{:?}", sector.sector.danger_level)));
    ctx.insert("traffic_density".into(), Dynamic::from(format!("{:?}", sector.sector.traffic_density)));
    ctx.insert("sector_id".into(), Dynamic::from(sector.sector.id.to_string()));

    // Call the script
    let result = hooks.try_call("spawning/npc_selection.rhai", "select_npc_class", ctx.into())?;

    // Parse the result map
    let result_map = result.try_cast::<rhai::Map>()?;

    let ship_class_str = result_map.get("ship_class")?.clone().into_string().ok()?;
    let ship_class = parse_ship_class(&ship_class_str)?;

    // Get faction from script result
    let faction_id = if let Some(faction_val) = result_map.get("faction_tag") {
        if faction_val.is_unit() {
            None
        } else if let Ok(tag) = faction_val.clone().into_string() {
            state.faction_tags.get(&tag).map(|id| *id)
        } else {
            None
        }
    } else {
        None
    };

    Some((ship_class, faction_id))
}

/// Call the script for NPC name generation.
fn script_generate_name(hooks: &ScriptHooks, ship_class: &ShipClass) -> Option<String> {
    // Build context for script
    let mut ctx = rhai::Map::new();
    ctx.insert("ship_class".into(), Dynamic::from(format!("{:?}", ship_class)));

    // Call the script
    let result = hooks.try_call("spawning/npc_naming.rhai", "generate_name", ctx.into())?;

    result.into_string().ok()
}

/// Parse a ship class string into a ShipClass enum.
fn parse_ship_class(s: &str) -> Option<ShipClass> {
    match s {
        "Freighter" => Some(ShipClass::Freighter),
        "Transport" => Some(ShipClass::Transport),
        "MiningVessel" => Some(ShipClass::MiningVessel),
        "PirateRaider" => Some(ShipClass::PirateRaider),
        "PirateFrigate" => Some(ShipClass::PirateFrigate),
        "TerroristBomber" => Some(ShipClass::TerroristBomber),
        "SeraSwarm" => Some(ShipClass::SeraSwarm),
        "SeraHunter" => Some(ShipClass::SeraHunter),
        "DroneHarvester" => Some(ShipClass::DroneHarvester),
        "DroneSwarm" => Some(ShipClass::DroneSwarm),
        _ => None,
    }
}
