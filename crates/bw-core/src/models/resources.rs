//! Player and ship resources
//!
//! The six core resources from the design document:
//! - Reputation: Standing with admiralty (integer, spendable)
//! - Fame: Public notoriety (integer, decays, multiplies rep changes)
//! - Ammunition: Attack capability (percentage)
//! - Fuel: Movement capability (percentage)
//! - Morale: Crew belief (percentage, affects combat)
//! - Experience: Crew skill (integer, always increases)

use serde::{Deserialize, Serialize};

/// Player-level resources (Reputation, Fame)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerResources {
    /// Standing with the admiralty. Can be spent for upgrades/favors.
    /// If it hits 0, the player is disgraced (game over).
    pub reputation: i32,

    /// Public notoriety. Decays over time without activity.
    /// Acts as a multiplier on reputation gains/losses.
    /// High fame = bigger rewards AND bigger penalties.
    pub fame: i32,
}

impl PlayerResources {
    pub fn new(starting_reputation: i32) -> Self {
        Self {
            reputation: starting_reputation,
            fame: 0,
        }
    }

    /// Calculate the fame multiplier for reputation changes.
    /// Fame amplifies both gains and losses.
    pub fn fame_multiplier(&self) -> f32 {
        1.0 + (self.fame as f32 * 0.01).clamp(-0.5, 2.0)
    }

    /// Apply a reputation change, modified by fame.
    pub fn apply_reputation_change(&mut self, base_change: i32) {
        let modified = (base_change as f32 * self.fame_multiplier()).round() as i32;
        self.reputation = (self.reputation + modified).max(0);
    }

    /// Decay fame over time (called periodically for inactive players).
    pub fn decay_fame(&mut self, decay_rate: i32) {
        self.fame = (self.fame - decay_rate).max(0);
    }

    /// Check if player is disgraced (game over condition).
    pub fn is_disgraced(&self) -> bool {
        self.reputation <= 0
    }
}

/// Ship-level resources (Ammunition, Fuel)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipResources {
    /// Percentage of ammunition remaining (0.0 - 100.0).
    /// Below 15%: attack debuff. At 0%: cannot attack.
    pub ammunition: f32,

    /// Percentage of fuel remaining (0.0 - 100.0).
    /// Below 10%: half-speed movement only.
    pub fuel: f32,
}

impl Default for ShipResources {
    fn default() -> Self {
        Self {
            ammunition: 100.0,
            fuel: 100.0,
        }
    }
}

impl ShipResources {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if ammunition is critically low (<15%).
    pub fn is_ammo_critical(&self) -> bool {
        self.ammunition < 15.0
    }

    /// Check if ammunition is depleted.
    pub fn is_ammo_depleted(&self) -> bool {
        self.ammunition <= 0.0
    }

    /// Check if fuel is critically low (<10%).
    pub fn is_fuel_critical(&self) -> bool {
        self.fuel < 10.0
    }

    /// Get the attack modifier based on ammunition level.
    pub fn ammo_attack_modifier(&self) -> f32 {
        if self.is_ammo_depleted() {
            0.0
        } else if self.is_ammo_critical() {
            0.5 // 50% attack power when low
        } else {
            1.0
        }
    }

    /// Get the movement modifier based on fuel level.
    pub fn fuel_movement_modifier(&self) -> f32 {
        if self.is_fuel_critical() {
            0.5 // Half speed when low
        } else {
            1.0
        }
    }

    /// Consume ammunition for an attack.
    pub fn consume_ammo(&mut self, amount: f32) {
        self.ammunition = (self.ammunition - amount).max(0.0);
    }

    /// Consume fuel for movement.
    pub fn consume_fuel(&mut self, amount: f32) {
        self.fuel = (self.fuel - amount).max(0.0);
    }

    /// Refuel to full.
    pub fn refuel(&mut self) {
        self.fuel = 100.0;
    }

    /// Rearm to full.
    pub fn rearm(&mut self) {
        self.ammunition = 100.0;
    }
}

/// Crew-level resources (Morale, Experience)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewResources {
    /// Crew morale as percentage (0.0 - 100.0).
    /// Above 50%: positive combat modifier.
    /// Below 50%: negative combat modifier.
    /// Always boosted by player Fame.
    pub morale: f32,

    /// Crew experience (integer, cumulative).
    /// Increases with both successes and failures.
    /// Successes grant more XP than failures.
    /// Affects attack, defense, and speed.
    /// Reset to 0 when ship is lost.
    pub experience: i32,
}

impl Default for CrewResources {
    fn default() -> Self {
        Self {
            morale: 50.0, // Start neutral
            experience: 0,
        }
    }
}

impl CrewResources {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a green crew (new ship).
    pub fn green_crew() -> Self {
        Self {
            morale: 50.0,
            experience: 0,
        }
    }

    /// Get the combat modifier from morale.
    /// Positive above 50%, negative below.
    pub fn morale_combat_modifier(&self) -> f32 {
        // At 100% morale: +25% combat bonus
        // At 50% morale: 0% modifier
        // At 0% morale: -25% combat penalty
        (self.morale - 50.0) / 200.0
    }

    /// Get the experience-based modifiers.
    pub fn experience_modifiers(&self) -> ExperienceModifiers {
        // Diminishing returns on XP
        let xp_factor = (self.experience as f32).sqrt() / 10.0;

        ExperienceModifiers {
            attack: 1.0 + (xp_factor * 0.1).min(0.5),   // Up to +50%
            defense: 1.0 + (xp_factor * 0.1).min(0.5),  // Up to +50%
            speed: 1.0 + (xp_factor * 0.05).min(0.25),  // Up to +25%
        }
    }

    /// Apply fame bonus to morale (famous captains boost crew morale).
    pub fn apply_fame_morale_bonus(&mut self, fame: i32) {
        let bonus = (fame as f32 * 0.1).min(20.0);
        self.morale = (self.morale + bonus).min(100.0);
    }

    /// Modify morale from mission outcome.
    pub fn apply_morale_change(&mut self, change: f32) {
        self.morale = (self.morale + change).clamp(0.0, 100.0);
    }

    /// Grant experience from mission outcome.
    pub fn grant_experience(&mut self, amount: i32) {
        self.experience += amount.max(0);
    }
}

/// Experience-based stat modifiers.
#[derive(Debug, Clone, Copy)]
pub struct ExperienceModifiers {
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fame_multiplier() {
        let mut resources = PlayerResources::new(100);

        // No fame = 1x multiplier
        assert!((resources.fame_multiplier() - 1.0).abs() < 0.01);

        // 50 fame = 1.5x multiplier
        resources.fame = 50;
        assert!((resources.fame_multiplier() - 1.5).abs() < 0.01);

        // 100 fame = 2x multiplier
        resources.fame = 100;
        assert!((resources.fame_multiplier() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_ammo_critical() {
        let mut resources = ShipResources::new();

        assert!(!resources.is_ammo_critical());
        resources.ammunition = 14.0;
        assert!(resources.is_ammo_critical());
    }

    #[test]
    fn test_morale_modifier() {
        let mut crew = CrewResources::new();

        // 50% morale = no modifier
        assert!((crew.morale_combat_modifier()).abs() < 0.01);

        // 100% morale = +25%
        crew.morale = 100.0;
        assert!((crew.morale_combat_modifier() - 0.25).abs() < 0.01);

        // 0% morale = -25%
        crew.morale = 0.0;
        assert!((crew.morale_combat_modifier() + 0.25).abs() < 0.01);
    }
}
