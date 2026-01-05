//! Mission generation system
//!
//! Spawns random missions based on sector conditions, player state, and time.

use bw_core::models::{DangerLevel, Mission, MissionType, Sector, TrafficDensity};
use rand::Rng;
use uuid::Uuid;

/// Configuration for mission generation.
#[derive(Debug, Clone)]
pub struct MissionGeneratorConfig {
    /// Base chance of spawning a mission per tick (10 ticks/second)
    pub base_spawn_chance: f32,

    /// Minimum time between missions (seconds)
    pub min_cooldown: f32,

    /// Maximum active missions per sector
    pub max_active_missions: usize,

    /// Weights for each mission type
    pub type_weights: MissionTypeWeights,
}

impl Default for MissionGeneratorConfig {
    fn default() -> Self {
        Self {
            base_spawn_chance: 0.001, // ~1% per second at 10 TPS
            min_cooldown: 30.0,
            max_active_missions: 3,
            type_weights: MissionTypeWeights::default(),
        }
    }
}

/// Relative weights for mission types.
#[derive(Debug, Clone)]
pub struct MissionTypeWeights {
    pub pirate_attack: f32,
    pub distress_signal: f32,
    pub smugglers: f32,
    pub asteroid_threat: f32,
    pub terrorist_plot: f32,
}

impl Default for MissionTypeWeights {
    fn default() -> Self {
        Self {
            pirate_attack: 30.0,
            distress_signal: 25.0,
            smugglers: 20.0,
            asteroid_threat: 15.0,
            terrorist_plot: 10.0,
        }
    }
}

impl MissionTypeWeights {
    /// Get total weight.
    pub fn total(&self) -> f32 {
        self.pirate_attack
            + self.distress_signal
            + self.smugglers
            + self.asteroid_threat
            + self.terrorist_plot
    }

    /// Adjust weights based on sector danger level.
    pub fn adjusted_for_danger(&self, danger: DangerLevel) -> Self {
        match danger {
            DangerLevel::Safe => Self {
                pirate_attack: self.pirate_attack * 0.2,
                distress_signal: self.distress_signal * 1.5,
                smugglers: self.smugglers * 0.5,
                asteroid_threat: self.asteroid_threat * 1.0,
                terrorist_plot: self.terrorist_plot * 0.3,
            },
            DangerLevel::Moderate => self.clone(),
            DangerLevel::Dangerous => Self {
                pirate_attack: self.pirate_attack * 2.0,
                distress_signal: self.distress_signal * 1.2,
                smugglers: self.smugglers * 1.5,
                asteroid_threat: self.asteroid_threat * 1.0,
                terrorist_plot: self.terrorist_plot * 1.5,
            },
            DangerLevel::Hostile => Self {
                pirate_attack: self.pirate_attack * 3.0,
                distress_signal: self.distress_signal * 0.5,
                smugglers: self.smugglers * 0.5,
                asteroid_threat: self.asteroid_threat * 1.5,
                terrorist_plot: self.terrorist_plot * 2.0,
            },
        }
    }

    /// Adjust weights based on traffic density.
    pub fn adjusted_for_traffic(&self, traffic: TrafficDensity) -> Self {
        match traffic {
            TrafficDensity::Sparse => Self {
                pirate_attack: self.pirate_attack * 0.5,
                distress_signal: self.distress_signal * 0.5,
                smugglers: self.smugglers * 1.5, // Smugglers like quiet routes
                asteroid_threat: self.asteroid_threat * 1.0,
                terrorist_plot: self.terrorist_plot * 0.3,
            },
            TrafficDensity::Light => Self {
                pirate_attack: self.pirate_attack * 0.8,
                distress_signal: self.distress_signal * 0.8,
                smugglers: self.smugglers * 1.2,
                asteroid_threat: self.asteroid_threat * 1.0,
                terrorist_plot: self.terrorist_plot * 0.5,
            },
            TrafficDensity::Moderate => self.clone(),
            TrafficDensity::Heavy => Self {
                pirate_attack: self.pirate_attack * 1.2,
                distress_signal: self.distress_signal * 1.5,
                smugglers: self.smugglers * 0.8,
                asteroid_threat: self.asteroid_threat * 1.2,
                terrorist_plot: self.terrorist_plot * 1.5,
            },
            TrafficDensity::Congested => Self {
                pirate_attack: self.pirate_attack * 1.5,
                distress_signal: self.distress_signal * 2.0,
                smugglers: self.smugglers * 0.5,
                asteroid_threat: self.asteroid_threat * 1.5,
                terrorist_plot: self.terrorist_plot * 2.0,
            },
        }
    }
}

/// The mission generator.
pub struct MissionGenerator {
    config: MissionGeneratorConfig,
    last_spawn_time: std::collections::HashMap<Uuid, f64>,
}

impl MissionGenerator {
    pub fn new(config: MissionGeneratorConfig) -> Self {
        Self {
            config,
            last_spawn_time: std::collections::HashMap::new(),
        }
    }

    /// Try to generate a mission for a sector.
    /// Returns None if no mission should spawn.
    pub fn try_generate(
        &mut self,
        sector: &Sector,
        active_mission_count: usize,
        current_time: f64,
    ) -> Option<Mission> {
        // Check max missions
        if active_mission_count >= self.config.max_active_missions {
            return None;
        }

        // Check cooldown
        if let Some(&last_time) = self.last_spawn_time.get(&sector.id) {
            if current_time - last_time < self.config.min_cooldown as f64 {
                return None;
            }
        }

        // Roll for spawn
        let mut rng = rand::thread_rng();
        let spawn_roll: f32 = rng.r#gen();

        // Adjust spawn chance by danger level
        let danger_mult = match sector.danger_level {
            DangerLevel::Safe => 0.5,
            DangerLevel::Moderate => 1.0,
            DangerLevel::Dangerous => 1.5,
            DangerLevel::Hostile => 2.0,
        };

        let adjusted_chance = self.config.base_spawn_chance * danger_mult;

        if spawn_roll > adjusted_chance {
            return None;
        }

        // Generate mission type
        let mission_type = self.select_mission_type(sector, &mut rng);

        // Create mission
        let mission = self.create_mission(mission_type, sector);

        // Update spawn time
        self.last_spawn_time.insert(sector.id, current_time);

        Some(mission)
    }

    /// Select a mission type based on weighted probabilities.
    fn select_mission_type<R: Rng>(&self, sector: &Sector, rng: &mut R) -> MissionType {
        // Get adjusted weights
        let weights = self
            .config
            .type_weights
            .adjusted_for_danger(sector.danger_level)
            .adjusted_for_traffic(sector.traffic_density);

        let total = weights.total();
        let roll: f32 = rng.r#gen::<f32>() * total;

        let mut accumulated = 0.0;

        accumulated += weights.pirate_attack;
        if roll < accumulated {
            return MissionType::PirateIntercept;
        }

        accumulated += weights.distress_signal;
        if roll < accumulated {
            return MissionType::DistressSignal;
        }

        accumulated += weights.smugglers;
        if roll < accumulated {
            return MissionType::Smugglers;
        }

        accumulated += weights.asteroid_threat;
        if roll < accumulated {
            return MissionType::AsteroidThreat;
        }

        MissionType::TerroristPlot
    }

    /// Create a mission of the given type.
    fn create_mission(&self, mission_type: MissionType, sector: &Sector) -> Mission {
        let title = match mission_type {
            MissionType::PirateIntercept => "Pirate Activity Detected",
            MissionType::DistressSignal => "Distress Signal",
            MissionType::Smugglers => "Suspicious Vessel",
            MissionType::AsteroidThreat => "Collision Alert",
            MissionType::TerroristPlot => "Security Alert",
            _ => "Unknown Mission",
        };

        let script_path = mission_type.default_script_path();

        Mission::new(
            mission_type,
            title.to_string(),
            sector.id,
            script_path.to_string(),
        )
    }

    /// Force spawn a specific mission type (for testing or commands).
    pub fn force_spawn(&mut self, mission_type: MissionType, sector: &Sector) -> Mission {
        self.create_mission(mission_type, sector)
    }

    /// Get statistics about recent spawns.
    pub fn spawn_stats(&self) -> MissionSpawnStats {
        MissionSpawnStats {
            sectors_with_recent_spawns: self.last_spawn_time.len(),
        }
    }
}

/// Statistics about mission spawning.
#[derive(Debug, Clone)]
pub struct MissionSpawnStats {
    pub sectors_with_recent_spawns: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_adjustment() {
        let weights = MissionTypeWeights::default();

        let safe_weights = weights.adjusted_for_danger(DangerLevel::Safe);
        assert!(safe_weights.pirate_attack < weights.pirate_attack);

        let dangerous_weights = weights.adjusted_for_danger(DangerLevel::Dangerous);
        assert!(dangerous_weights.pirate_attack > weights.pirate_attack);
    }

    #[test]
    fn test_total_weight() {
        let weights = MissionTypeWeights::default();
        assert!(weights.total() > 0.0);
    }
}
