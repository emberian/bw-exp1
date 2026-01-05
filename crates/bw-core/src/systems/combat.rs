//! Combat system
//!
//! Handles combat resolution between ships.
//! Actual damage formulas are in Rhai scripts for moddability.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{CombatStats, WeaponType};

/// A combat engagement between ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEngagement {
    /// Unique identifier
    pub id: Uuid,

    /// Sector where combat is happening
    pub sector_id: Uuid,

    /// Ships on side A (usually player/allies)
    pub side_a: Vec<Uuid>,

    /// Ships on side B (usually enemies)
    pub side_b: Vec<Uuid>,

    /// Current combat round
    pub round: u32,

    /// Combat log for this engagement
    pub log: Vec<CombatLogEntry>,

    /// Whether combat is resolved
    pub is_resolved: bool,

    /// Winning side (if resolved)
    pub winner: Option<CombatSide>,
}

impl CombatEngagement {
    pub fn new(sector_id: Uuid, side_a: Vec<Uuid>, side_b: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            sector_id,
            side_a,
            side_b,
            round: 0,
            log: Vec::new(),
            is_resolved: false,
            winner: None,
        }
    }

    /// Add a log entry.
    pub fn log_event(&mut self, entry: CombatLogEntry) {
        self.log.push(entry);
    }

    /// Get all participating ships.
    pub fn all_participants(&self) -> Vec<Uuid> {
        let mut all = self.side_a.clone();
        all.extend(&self.side_b);
        all
    }

    /// Check which side a ship is on.
    pub fn get_side(&self, ship_id: Uuid) -> Option<CombatSide> {
        if self.side_a.contains(&ship_id) {
            Some(CombatSide::A)
        } else if self.side_b.contains(&ship_id) {
            Some(CombatSide::B)
        } else {
            None
        }
    }

    /// Remove a ship from combat (destroyed or fled).
    pub fn remove_ship(&mut self, ship_id: Uuid) {
        self.side_a.retain(|&id| id != ship_id);
        self.side_b.retain(|&id| id != ship_id);

        // Check for resolution
        if self.side_a.is_empty() {
            self.is_resolved = true;
            self.winner = Some(CombatSide::B);
        } else if self.side_b.is_empty() {
            self.is_resolved = true;
            self.winner = Some(CombatSide::A);
        }
    }
}

/// Combat side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatSide {
    A,
    B,
}

/// A log entry for combat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLogEntry {
    pub round: u32,
    pub attacker_id: Uuid,
    pub target_id: Uuid,
    pub weapon_type: WeaponType,
    pub damage_dealt: f32,
    pub hit: bool,
    pub target_destroyed: bool,
    pub message: String,
}

/// Result of an attack calculation.
#[derive(Debug, Clone)]
pub struct AttackResult {
    pub hit: bool,
    pub damage: f32,
    pub ammo_consumed: f32,
    pub critical_hit: bool,
}

/// Calculate base attack (can be overridden by Rhai).
pub fn calculate_attack(
    attacker_stats: &CombatStats,
    defender_stats: &CombatStats,
    weapon_damage: f32,
    weapon_accuracy: f32,
) -> AttackResult {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Base hit chance modified by attacker speed vs defender speed
    let speed_modifier = attacker_stats.speed / defender_stats.speed.max(1.0);
    let hit_chance = (weapon_accuracy * speed_modifier).clamp(0.1, 0.95);

    let hit = rng.r#gen::<f32>() < hit_chance;

    if !hit {
        return AttackResult {
            hit: false,
            damage: 0.0,
            ammo_consumed: 1.0, // Still use ammo on miss
            critical_hit: false,
        };
    }

    // Damage calculation
    let base_damage = weapon_damage * attacker_stats.attack / 100.0;
    let defense_reduction = defender_stats.defense / 200.0; // Max 50% reduction
    let final_damage = base_damage * (1.0 - defense_reduction);

    // Critical hit chance (based on experience via attack stat)
    let crit_chance = 0.05 + (attacker_stats.attack / 1000.0).min(0.15);
    let critical = rng.r#gen::<f32>() < crit_chance;
    let damage = if critical { final_damage * 1.5 } else { final_damage };

    AttackResult {
        hit: true,
        damage,
        ammo_consumed: 1.0,
        critical_hit: critical,
    }
}
