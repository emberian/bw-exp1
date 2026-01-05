//! NPC Ship Spawning System
//!
//! Spawns NPC ships in sectors based on danger level and traffic density.

use uuid::Uuid;

use bw_core::models::{DangerLevel, Ship, ShipClass, TrafficDensity};
use bw_shared::dto::{PositionDto, ShipDto};

use crate::{GameState, SectorInstance};

/// Result of spawning NPCs, includes both DTOs for broadcast and IDs for behavior attachment.
pub struct SpawnResult {
    /// DTOs to broadcast to clients
    pub ship_dtos: Vec<ShipDto>,
    /// Ship IDs and their classes for behavior attachment
    pub spawned_ships: Vec<(Uuid, Uuid, ShipClass)>, // (ship_id, sector_id, class)
}

/// Configuration for NPC spawning.
#[derive(Debug, Clone)]
pub struct NpcSpawnConfig {
    /// Base chance to spawn NPC per tick (at 10 TPS)
    pub base_spawn_chance: f32,
    /// Minimum NPCs to maintain per sector
    pub min_npcs_per_sector: usize,
    /// Maximum NPCs per sector
    pub max_npcs_per_sector: usize,
    /// Cooldown ticks between spawn attempts
    pub spawn_cooldown_ticks: u64,
}

impl Default for NpcSpawnConfig {
    fn default() -> Self {
        Self {
            base_spawn_chance: 0.02, // 2% per tick = ~20% per second
            min_npcs_per_sector: 3,
            max_npcs_per_sector: 15,
            spawn_cooldown_ticks: 50, // 5 seconds between spawns
        }
    }
}

/// Try to spawn NPCs in a sector.
pub fn try_spawn_npcs(
    state: &GameState,
    sector: &SectorInstance,
    _tick: u64,
    config: &NpcSpawnConfig,
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
        // Adjust spawn chance by danger level
        let danger_mult = match sector.sector.danger_level {
            DangerLevel::Safe => 0.5,
            DangerLevel::Moderate => 1.0,
            DangerLevel::Dangerous => 1.5,
            DangerLevel::Hostile => 2.0,
        };

        let spawn_chance = config.base_spawn_chance * danger_mult;
        rng.r#gen::<f32>() < spawn_chance
    };

    if !should_spawn {
        return result;
    }

    // Determine NPC type based on sector
    let ship_class = select_npc_class(&sector.sector.danger_level, &sector.sector.traffic_density, &mut rng);
    let faction_id = select_faction_for_class(&ship_class, state);

    // Generate position
    let position = sector.sector.random_position();

    // Create NPC name
    let name = generate_npc_name(&ship_class, &mut rng);

    // Create NPC ship
    let npc_ship = Ship::new_npc_ship(name.clone(), ship_class, sector.sector.id, position, faction_id);

    let ship_id = npc_ship.id;
    let sector_id = sector.sector.id;

    // Create DTO for broadcast
    let ship_dto = ShipDto {
        id: ship_id,
        name: npc_ship.name.clone(),
        owner_id: None,
        ship_class: format!("{:?}", ship_class),
        position: PositionDto {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        hull_percent: npc_ship.hull_integrity,
        shield_percent: npc_ship.shield_strength,
        status: "Idle".to_string(),
        faction_tag: faction_id.and_then(|fid| state.factions.get(&fid).map(|f| f.tag.clone())),
        is_player: false,
        is_hostile: ship_class.is_hostile(),
    };

    // Add to state
    state.ships.insert(ship_id, npc_ship);
    sector.ship_ids.insert(ship_id, ());

    tracing::debug!("Spawned NPC {} ({:?}) in {}", name, ship_class, sector.sector.name);

    result.ship_dtos.push(ship_dto);
    result.spawned_ships.push((ship_id, sector_id, ship_class));
    result
}

/// Select an NPC class based on sector conditions.
fn select_npc_class<R: rand::Rng>(
    danger: &DangerLevel,
    traffic: &TrafficDensity,
    rng: &mut R,
) -> ShipClass {
    // Build weighted list based on conditions
    let mut weights: Vec<(ShipClass, f32)> = Vec::new();

    // Civilian traffic
    match traffic {
        TrafficDensity::Sparse => {
            weights.push((ShipClass::Freighter, 5.0));
            weights.push((ShipClass::MiningVessel, 10.0));
        }
        TrafficDensity::Light => {
            weights.push((ShipClass::Freighter, 15.0));
            weights.push((ShipClass::Transport, 10.0));
            weights.push((ShipClass::MiningVessel, 10.0));
        }
        TrafficDensity::Moderate => {
            weights.push((ShipClass::Freighter, 25.0));
            weights.push((ShipClass::Transport, 20.0));
            weights.push((ShipClass::MiningVessel, 5.0));
        }
        TrafficDensity::Heavy | TrafficDensity::Congested => {
            weights.push((ShipClass::Freighter, 35.0));
            weights.push((ShipClass::Transport, 30.0));
        }
    }

    // Hostile ships based on danger
    match danger {
        DangerLevel::Safe => {
            // Rare hostiles
            weights.push((ShipClass::PirateRaider, 2.0));
        }
        DangerLevel::Moderate => {
            weights.push((ShipClass::PirateRaider, 15.0));
            weights.push((ShipClass::PirateFrigate, 5.0));
            weights.push((ShipClass::DroneSwarm, 5.0));
        }
        DangerLevel::Dangerous => {
            weights.push((ShipClass::PirateRaider, 20.0));
            weights.push((ShipClass::PirateFrigate, 15.0));
            weights.push((ShipClass::TerroristBomber, 5.0));
            weights.push((ShipClass::DroneHarvester, 10.0));
            weights.push((ShipClass::DroneSwarm, 10.0));
            weights.push((ShipClass::SeraSwarm, 5.0));
        }
        DangerLevel::Hostile => {
            weights.push((ShipClass::PirateRaider, 10.0));
            weights.push((ShipClass::PirateFrigate, 20.0));
            weights.push((ShipClass::TerroristBomber, 10.0));
            weights.push((ShipClass::DroneHarvester, 15.0));
            weights.push((ShipClass::DroneSwarm, 15.0));
            weights.push((ShipClass::SeraSwarm, 15.0));
            weights.push((ShipClass::SeraHunter, 10.0));
        }
    }

    // Weighted selection
    let total: f32 = weights.iter().map(|(_, w)| w).sum();
    let roll = rng.r#gen::<f32>() * total;

    let mut accumulated = 0.0;
    for (class, weight) in &weights {
        accumulated += weight;
        if roll < accumulated {
            return *class;
        }
    }

    // Default fallback
    ShipClass::Freighter
}

/// Select faction for NPC class.
fn select_faction_for_class(class: &ShipClass, state: &GameState) -> Option<Uuid> {
    // Find appropriate faction tag
    let faction_tag = match class {
        ShipClass::Freighter | ShipClass::Transport | ShipClass::MiningVessel => {
            Some("FORGEBORN") // Civilian ships affiliated with Forgeborn
        }
        ShipClass::PirateRaider | ShipClass::PirateFrigate => Some("PIRATES"),
        ShipClass::TerroristBomber => None, // Terrorists are unaffiliated
        ShipClass::SeraSwarm | ShipClass::SeraHunter => Some("SERA"),
        ShipClass::DroneHarvester | ShipClass::DroneSwarm => Some("DRONES"),
        _ => None,
    };

    // O(1) lookup via faction_tags index
    faction_tag.and_then(|tag| state.faction_tags.get(tag).map(|id| *id))
}

/// Generate a name for an NPC ship.
fn generate_npc_name<R: rand::Rng>(class: &ShipClass, rng: &mut R) -> String {
    let prefixes = match class {
        ShipClass::Freighter => vec!["Cargo", "Hauler", "Bulk", "Star", "Trade"],
        ShipClass::Transport => vec!["Transit", "Shuttle", "Ferry", "Express", "Liner"],
        ShipClass::MiningVessel => vec!["Drill", "Excavator", "Miner", "Prospect", "Deep"],
        ShipClass::PirateRaider => vec!["Black", "Shadow", "Red", "Rogue", "Swift"],
        ShipClass::PirateFrigate => vec!["Dread", "Crimson", "Iron", "Storm", "Void"],
        ShipClass::TerroristBomber => vec!["Zealot", "Martyr", "Flame", "Fury", "End"],
        ShipClass::SeraSwarm => vec!["Swarm", "Cluster", "Mass", "Wave", "Tide"],
        ShipClass::SeraHunter => vec!["Hunter", "Stalker", "Reaper", "Slayer", "Doom"],
        ShipClass::DroneHarvester => vec!["Harvester", "Collector", "Gatherer", "Scavenger", "Reaper"],
        ShipClass::DroneSwarm => vec!["Drone", "Unit", "Cluster", "Wing", "Pack"],
        _ => vec!["Ship"],
    };

    let suffixes = match class {
        ShipClass::Freighter | ShipClass::Transport | ShipClass::MiningVessel => {
            vec!["I", "II", "III", "IV", "V", "A", "B", "C", "Prime", "Delta"]
        }
        ShipClass::PirateRaider | ShipClass::PirateFrigate => {
            vec![
                "Fang", "Claw", "Blade", "Edge", "Thorn", "Hook", "Spike", "Doom",
            ]
        }
        _ => vec!["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta"],
    };

    let prefix = prefixes[rng.gen_range(0..prefixes.len())];
    let suffix = suffixes[rng.gen_range(0..suffixes.len())];
    let num: u32 = rng.gen_range(100..999);

    format!("{} {} {}", prefix, suffix, num)
}

/// Maximum distance from any player before NPC is despawned.
const NPC_DESPAWN_DISTANCE: f64 = 800.0;

/// Despawn NPC ships that are destroyed or too far from any player.
pub fn cleanup_npcs(
    state: &GameState,
    sector: &SectorInstance,
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
                    if min_distance > NPC_DESPAWN_DISTANCE {
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
