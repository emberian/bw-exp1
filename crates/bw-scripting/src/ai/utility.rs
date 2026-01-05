//! Utility AI Scoring System
//!
//! Provides a decision-making system based on scoring multiple options
//! and selecting the best one.
//!
//! # How It Works
//!
//! 1. Each option has multiple "considerations" (scoring functions)
//! 2. Each consideration returns a score from 0.0 to 1.0
//! 3. Scores are combined (multiplied) and weighted
//! 4. The option with the highest score wins
//!
//! # Example
//!
//! ```rhai
//! // Consider attacking: high when target is weak and we have ammo
//! fn target_health(ctx) {
//!     1.0 - (ctx.target.hull / ctx.target.max_hull)
//! }
//!
//! fn my_ammo(ctx) {
//!     ctx.ship.ammo / ctx.ship.max_ammo
//! }
//!
//! // Consider retreating: high when we're hurt and outnumbered
//! fn my_health_low(ctx) {
//!     if ctx.ship.hull < 30.0 { 0.9 } else { 0.1 }
//! }
//!
//! fn outnumbered(ctx) {
//!     let enemies = ctx.enemies.len();
//!     let allies = ctx.allies.len();
//!     if enemies > allies * 2 { 0.8 } else { 0.2 }
//! }
//! ```

use super::{UtilityOption, BtStatus};

/// Result of utility AI scoring.
#[derive(Debug, Clone)]
pub struct UtilityScore {
    /// The option that was scored.
    pub option: UtilityOption,
    /// Individual consideration scores.
    pub consideration_scores: Vec<f64>,
    /// Combined score (product of considerations).
    pub raw_score: f64,
    /// Final score after weight is applied.
    pub final_score: f64,
}

impl UtilityScore {
    /// Create a new utility score.
    pub fn new(option: UtilityOption, consideration_scores: Vec<f64>) -> Self {
        // Combine scores by multiplication (standard utility AI approach)
        // This ensures that a 0 in any consideration gives 0 final score
        let raw_score = consideration_scores
            .iter()
            .fold(1.0, |acc, &score| acc * score.clamp(0.0, 1.0));

        let final_score = raw_score * option.weight;

        Self {
            option,
            consideration_scores,
            raw_score,
            final_score,
        }
    }

    /// Create a failed score (for when a consideration throws an error).
    pub fn failed(option: UtilityOption) -> Self {
        Self {
            option,
            consideration_scores: vec![],
            raw_score: 0.0,
            final_score: 0.0,
        }
    }
}

/// Result of selecting the best utility option.
#[derive(Debug, Clone)]
pub struct UtilityDecision {
    /// The chosen option (if any scored > 0).
    pub chosen: Option<UtilityScore>,
    /// All options with their scores.
    pub all_scores: Vec<UtilityScore>,
    /// Status of the decision process.
    pub status: BtStatus,
}

impl UtilityDecision {
    /// Create a decision from a list of scores.
    pub fn from_scores(scores: Vec<UtilityScore>) -> Self {
        // Find the highest scoring option
        let chosen = scores
            .iter()
            .filter(|s| s.final_score > 0.0)
            .max_by(|a, b| {
                a.final_score
                    .partial_cmp(&b.final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();

        let status = if chosen.is_some() {
            BtStatus::Success
        } else {
            BtStatus::Failure
        };

        Self {
            chosen,
            all_scores: scores,
            status,
        }
    }

    /// Create a decision with no options.
    pub fn none() -> Self {
        Self {
            chosen: None,
            all_scores: vec![],
            status: BtStatus::Failure,
        }
    }

    /// Get the action name of the chosen option.
    pub fn action(&self) -> Option<&str> {
        self.chosen.as_ref().map(|s| s.option.action.as_str())
    }
}

/// Response curve functions for shaping consideration scores.
pub mod curves {
    /// Linear response curve (no transformation).
    pub fn linear(x: f64) -> f64 {
        x.clamp(0.0, 1.0)
    }

    /// Quadratic response curve (slow start, fast finish).
    pub fn quadratic(x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        x * x
    }

    /// Inverse quadratic (fast start, slow finish).
    pub fn inverse_quadratic(x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        1.0 - (1.0 - x) * (1.0 - x)
    }

    /// S-curve / logistic (slow start and finish, fast middle).
    pub fn logistic(x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        let k = 10.0; // Steepness
        let mid = 0.5;
        1.0 / (1.0 + (-k * (x - mid)).exp())
    }

    /// Exponential decay (starts high, drops quickly).
    pub fn exponential_decay(x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        (-3.0 * x).exp()
    }

    /// Step function (binary threshold at midpoint).
    pub fn step(x: f64) -> f64 {
        if x >= 0.5 { 1.0 } else { 0.0 }
    }

    /// Threshold with custom cutoff.
    pub fn threshold(x: f64, cutoff: f64) -> f64 {
        if x >= cutoff { 1.0 } else { 0.0 }
    }

    /// Inverse (1 - x).
    pub fn inverse(x: f64) -> f64 {
        1.0 - x.clamp(0.0, 1.0)
    }

    /// Custom polynomial response.
    pub fn polynomial(x: f64, exponent: f64) -> f64 {
        x.clamp(0.0, 1.0).powf(exponent)
    }
}

/// Standard consideration helpers for common game scenarios.
pub mod considerations {
    /// Health percentage (0 = dead, 1 = full health).
    pub fn health_percent(current: f64, max: f64) -> f64 {
        if max <= 0.0 { return 0.0; }
        (current / max).clamp(0.0, 1.0)
    }

    /// Inverse health (high when low health).
    pub fn damage_percent(current: f64, max: f64) -> f64 {
        1.0 - health_percent(current, max)
    }

    /// Distance score (closer = higher score).
    pub fn proximity(distance: f64, max_range: f64) -> f64 {
        if max_range <= 0.0 { return 0.0; }
        (1.0 - distance / max_range).clamp(0.0, 1.0)
    }

    /// Resource availability (0 = empty, 1 = full).
    pub fn resource_percent(current: f64, max: f64) -> f64 {
        if max <= 0.0 { return 0.0; }
        (current / max).clamp(0.0, 1.0)
    }

    /// Low resource urgency (high when resource is low).
    pub fn resource_urgency(current: f64, max: f64, threshold: f64) -> f64 {
        let percent = resource_percent(current, max);
        if percent < threshold {
            1.0 - (percent / threshold)
        } else {
            0.0
        }
    }

    /// Numeric advantage (positive when we have more, negative when outnumbered).
    pub fn numeric_advantage(allies: usize, enemies: usize) -> f64 {
        let total = (allies + enemies) as f64;
        if total == 0.0 { return 0.5; }
        (allies as f64 / total).clamp(0.0, 1.0)
    }

    /// Outnumbered score (high when facing more enemies).
    pub fn outnumbered(allies: usize, enemies: usize) -> f64 {
        1.0 - numeric_advantage(allies, enemies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utility_score_calculation() {
        let option = UtilityOption::new(
            "attack",
            vec!["target_weak".into()],
            1.5,
        );

        let score = UtilityScore::new(option, vec![0.8]);
        assert!((score.raw_score - 0.8).abs() < 0.001);
        assert!((score.final_score - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_utility_score_zero_consideration() {
        let option = UtilityOption::new(
            "attack",
            vec!["has_ammo".into(), "can_see_target".into()],
            1.0,
        );

        // If any consideration is 0, the whole thing is 0
        let score = UtilityScore::new(option, vec![0.9, 0.0]);
        assert_eq!(score.final_score, 0.0);
    }

    #[test]
    fn test_utility_decision() {
        let options = vec![
            UtilityScore::new(
                UtilityOption::new("attack", vec![], 1.0),
                vec![0.5],
            ),
            UtilityScore::new(
                UtilityOption::new("retreat", vec![], 1.0),
                vec![0.8],
            ),
            UtilityScore::new(
                UtilityOption::new("wait", vec![], 1.0),
                vec![0.3],
            ),
        ];

        let decision = UtilityDecision::from_scores(options);
        assert_eq!(decision.action(), Some("retreat"));
        assert_eq!(decision.status, BtStatus::Success);
    }

    #[test]
    fn test_curves() {
        // Linear
        assert_eq!(curves::linear(0.5), 0.5);

        // Quadratic
        assert_eq!(curves::quadratic(0.5), 0.25);

        // Inverse quadratic
        assert_eq!(curves::inverse_quadratic(0.5), 0.75);

        // Step
        assert_eq!(curves::step(0.3), 0.0);
        assert_eq!(curves::step(0.7), 1.0);

        // Inverse
        assert_eq!(curves::inverse(0.3), 0.7);
    }

    #[test]
    fn test_considerations() {
        // Health percent
        assert_eq!(considerations::health_percent(50.0, 100.0), 0.5);
        assert_eq!(considerations::damage_percent(50.0, 100.0), 0.5);

        // Proximity
        assert_eq!(considerations::proximity(50.0, 100.0), 0.5);
        assert_eq!(considerations::proximity(0.0, 100.0), 1.0);
        assert_eq!(considerations::proximity(100.0, 100.0), 0.0);

        // Numeric advantage
        assert_eq!(considerations::numeric_advantage(2, 2), 0.5);
        assert!(considerations::numeric_advantage(3, 1) > 0.5);
        assert!(considerations::numeric_advantage(1, 3) < 0.5);
    }
}
