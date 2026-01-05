//! Reputation and Fame system
//!
//! Implements the reputation/fame mechanics from the design document:
//! - Reputation: Standing with admiralty, spendable, 0 = game over
//! - Fame: Public notoriety, decays, multiplies reputation changes

use bw_core::models::{Player, PlayerResources};

/// Configuration for reputation system.
pub struct ReputationConfig {
    /// Fame decay per tick (when player is inactive)
    pub fame_decay_rate: i32,
    /// Ticks between fame decay
    pub fame_decay_interval: u32,
    /// Minimum reputation to request shore leave
    pub shore_leave_cost: i32,
    /// Minimum reputation to request resupply
    pub resupply_cost: i32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            fame_decay_rate: 1,
            fame_decay_interval: 100, // Every 100 ticks (10 seconds at 10 TPS)
            shore_leave_cost: 10,
            resupply_cost: 5,
        }
    }
}

/// Apply reputation change with fame multiplier.
pub fn apply_reputation_change(resources: &mut PlayerResources, base_change: i32) -> i32 {
    let multiplier = resources.fame_multiplier();
    let actual_change = (base_change as f32 * multiplier).round() as i32;
    resources.reputation = (resources.reputation + actual_change).max(0);
    actual_change
}

/// Apply fame change (no multiplier).
pub fn apply_fame_change(resources: &mut PlayerResources, change: i32) {
    resources.fame = (resources.fame + change).max(0);
}

/// Decay fame for inactive player.
pub fn decay_fame(resources: &mut PlayerResources, config: &ReputationConfig) {
    resources.fame = (resources.fame - config.fame_decay_rate).max(0);
}

/// Check if player can afford a reputation cost.
pub fn can_afford(resources: &PlayerResources, cost: i32) -> bool {
    resources.reputation >= cost
}

/// Spend reputation (for upgrades, favors, etc.).
pub fn spend_reputation(resources: &mut PlayerResources, cost: i32) -> bool {
    if can_afford(resources, cost) {
        resources.reputation -= cost;
        true
    } else {
        false
    }
}

/// Calculate mission reward with all modifiers.
pub fn calculate_mission_reward(
    base_reputation: i32,
    base_fame: i32,
    player_fame: i32,
    sector_danger_multiplier: f32,
    is_high_profile: bool,
) -> MissionReward {
    // Fame multiplier on reputation
    let fame_mult = 1.0 + (player_fame as f32 * 0.01).clamp(-0.5, 2.0);

    // Danger zone bonus
    let danger_mult = sector_danger_multiplier;

    // High profile bonus
    let profile_mult = if is_high_profile { 1.5 } else { 1.0 };

    MissionReward {
        reputation: (base_reputation as f32 * fame_mult * danger_mult * profile_mult).round() as i32,
        fame: (base_fame as f32 * danger_mult * profile_mult).round() as i32,
    }
}

/// Calculated mission reward.
pub struct MissionReward {
    pub reputation: i32,
    pub fame: i32,
}

/// Result of a mission completion on player resources.
pub fn apply_mission_result(player: &mut Player, success: bool, reward: &MissionReward) {
    if success {
        apply_reputation_change(&mut player.resources, reward.reputation);
        apply_fame_change(&mut player.resources, reward.fame);
        player.record_mission_success();
    } else {
        // Penalty is negative of reward (but smaller)
        let penalty = -((reward.reputation as f32 * 0.5).round() as i32);
        apply_reputation_change(&mut player.resources, penalty);
        // Fame still increases slightly on failure (notoriety)
        apply_fame_change(&mut player.resources, reward.fame / 3);
        player.record_mission_failure();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fame_multiplier() {
        let mut resources = PlayerResources::new(100);

        // Base change
        let change = apply_reputation_change(&mut resources, 10);
        assert_eq!(change, 10);
        assert_eq!(resources.reputation, 110);

        // With fame
        resources.fame = 50;
        let change = apply_reputation_change(&mut resources, 10);
        assert_eq!(change, 15); // 1.5x multiplier
    }

    #[test]
    fn test_spending() {
        let mut resources = PlayerResources::new(100);

        assert!(can_afford(&resources, 50));
        assert!(spend_reputation(&mut resources, 50));
        assert_eq!(resources.reputation, 50);

        assert!(!can_afford(&resources, 100));
        assert!(!spend_reputation(&mut resources, 100));
        assert_eq!(resources.reputation, 50);
    }
}
