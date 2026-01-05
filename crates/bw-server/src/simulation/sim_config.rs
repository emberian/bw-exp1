//! Simulation configuration structs.
//!
//! These are loaded as part of ServerConfig from config.toml.

use serde::{Deserialize, Serialize};

/// Simulation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulationConfig {
    pub npc_spawning: NpcSpawningConfig,
    pub combat: CombatConfig,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            npc_spawning: NpcSpawningConfig::default(),
            combat: CombatConfig::default(),
        }
    }
}

/// NPC spawning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NpcSpawningConfig {
    /// Base spawn chance per tick (at 10 TPS, 0.02 = ~20% per second)
    pub base_spawn_chance: f32,
    /// Minimum NPCs to maintain per sector
    pub min_npcs_per_sector: usize,
    /// Maximum NPCs per sector
    pub max_npcs_per_sector: usize,
    /// Cooldown ticks between spawn attempts (50 = 5 seconds at 10 TPS)
    pub spawn_cooldown_ticks: u64,
    /// Distance from all players before NPC despawns
    pub despawn_distance: f64,
    /// Danger level multipliers for spawn chance
    pub danger_multipliers: DangerMultipliers,
}

impl Default for NpcSpawningConfig {
    fn default() -> Self {
        Self {
            base_spawn_chance: 0.02,
            min_npcs_per_sector: 3,
            max_npcs_per_sector: 15,
            spawn_cooldown_ticks: 50,
            despawn_distance: 800.0,
            danger_multipliers: DangerMultipliers::default(),
        }
    }
}

/// Danger level multipliers for spawn probability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DangerMultipliers {
    pub safe: f32,
    pub moderate: f32,
    pub dangerous: f32,
    pub hostile: f32,
}

impl Default for DangerMultipliers {
    fn default() -> Self {
        Self {
            safe: 0.5,
            moderate: 1.0,
            dangerous: 1.5,
            hostile: 2.0,
        }
    }
}

/// Combat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CombatConfig {
    /// Experience granted per successful hit
    pub xp_per_hit: i32,
    /// Experience granted per kill
    pub xp_per_kill: i32,
    /// Maximum flee success chance (0.0 to 1.0)
    pub max_flee_chance: f32,
    /// Divisor for flee chance calculation (flee_chance = speed / divisor)
    pub flee_speed_divisor: f32,
}

impl Default for CombatConfig {
    fn default() -> Self {
        Self {
            xp_per_hit: 1,
            xp_per_kill: 10,
            max_flee_chance: 0.70,
            flee_speed_divisor: 100.0,
        }
    }
}
