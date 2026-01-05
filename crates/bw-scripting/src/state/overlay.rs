//! Mutation Overlay for Read-Your-Own-Writes
//!
//! When scripts modify state, the changes are queued for later application.
//! The `MutationOverlay` tracks these pending changes and merges them with
//! reads, allowing scripts to see their own modifications within the same
//! execution.
//!
//! # Example
//!
//! Without overlay (old behavior):
//! ```rhai,ignore
//! ship.hull = 50.0;      // Queued, not applied
//! if ship.hull < 60.0 {  // Still reads original 100.0!
//!     flee();            // Never runs
//! }
//! ```
//!
//! With overlay (new behavior):
//! ```rhai,ignore
//! ship.hull = 50.0;      // Queued AND tracked in overlay
//! if ship.hull < 60.0 {  // Reads 50.0 from overlay
//!     flee();            // Runs correctly
//! }
//! ```

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{ShipSnapshot, PlayerSnapshot, ShipChanges, PlayerChanges};

/// Tracks pending mutations for read-your-own-writes semantics.
#[derive(Debug, Default)]
pub struct MutationOverlay {
    /// Accumulated ship changes indexed by ship ID.
    ship_changes: HashMap<Uuid, ShipChanges>,
    /// Accumulated player changes indexed by player ID.
    player_changes: HashMap<Uuid, PlayerChanges>,
    /// Entities marked for destruction (should be filtered from reads).
    pending_destroys: HashSet<Uuid>,
}

impl MutationOverlay {
    /// Create a new empty overlay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a ship mutation.
    ///
    /// Multiple mutations to the same ship are merged, with later values
    /// overwriting earlier ones for the same field.
    pub fn record_ship_change(&mut self, ship_id: Uuid, changes: &ShipChanges) {
        let entry = self.ship_changes.entry(ship_id).or_default();
        entry.merge(changes);
    }

    /// Record a player mutation.
    pub fn record_player_change(&mut self, player_id: Uuid, changes: &PlayerChanges) {
        let entry = self.player_changes.entry(player_id).or_default();
        entry.merge(changes);
    }

    /// Mark an entity as destroyed.
    pub fn record_destroy(&mut self, entity_id: Uuid) {
        self.pending_destroys.insert(entity_id);
    }

    /// Check if an entity is pending destruction.
    pub fn is_destroyed(&self, entity_id: Uuid) -> bool {
        self.pending_destroys.contains(&entity_id)
    }

    /// Apply pending changes to a ship snapshot.
    ///
    /// Returns a new snapshot with changes merged in, or None if the ship
    /// is pending destruction.
    pub fn apply_to_ship(&self, snapshot: ShipSnapshot) -> Option<ShipSnapshot> {
        if self.pending_destroys.contains(&snapshot.id) {
            return None;
        }

        let Some(changes) = self.ship_changes.get(&snapshot.id) else {
            return Some(snapshot);
        };

        let mut s = snapshot;

        // Apply numeric field changes
        if let Some(hull) = changes.hull {
            s.hull = hull;
        }
        if let Some(shields) = changes.shields {
            s.shields = shields;
        }
        if let Some(ammunition) = changes.ammunition {
            s.ammunition = ammunition;
        }
        if let Some(fuel) = changes.fuel {
            s.fuel = fuel;
        }
        if let Some(morale) = changes.morale {
            s.morale = morale;
        }
        if let Some(experience) = changes.experience {
            s.experience = experience;
        }

        // Apply position change
        if let Some(ref position) = changes.position {
            s.position = super::PositionSnapshot {
                x: position.x,
                y: position.y,
                z: position.z,
            };
        }

        // Apply status change
        if let Some(ref status_change) = changes.status {
            s.status = match status_change {
                super::ShipStatusChange::Idle => "idle".to_string(),
                super::ShipStatusChange::InTransit { .. } => "in_transit".to_string(),
                super::ShipStatusChange::Disabled => "disabled".to_string(),
            };
        }

        // Apply combat stance change
        if let Some(ref stance) = changes.combat_stance {
            s.combat_stance = stance.clone();
        }

        // Apply locked target change
        if let Some(locked_target) = changes.locked_target {
            s.locked_target = locked_target;
        }

        Some(s)
    }

    /// Apply pending changes to a player snapshot.
    pub fn apply_to_player(&self, snapshot: PlayerSnapshot) -> Option<PlayerSnapshot> {
        if self.pending_destroys.contains(&snapshot.id) {
            return None;
        }

        let Some(changes) = self.player_changes.get(&snapshot.id) else {
            return Some(snapshot);
        };

        let mut p = snapshot;

        // Apply absolute changes
        if let Some(reputation) = changes.reputation {
            p.reputation = reputation;
        }
        if let Some(fame) = changes.fame {
            p.fame = fame;
        }
        if let Some(credits) = changes.credits {
            p.credits = credits;
        }

        // Apply delta changes
        if let Some(delta) = changes.reputation_delta {
            p.reputation += delta;
        }
        if let Some(delta) = changes.fame_delta {
            p.fame += delta;
        }
        if let Some(delta) = changes.credits_delta {
            p.credits += delta;
        }

        Some(p)
    }

    /// Check if there are any pending changes for a ship.
    pub fn has_ship_changes(&self, ship_id: Uuid) -> bool {
        self.ship_changes.contains_key(&ship_id)
    }

    /// Check if there are any pending changes for a player.
    pub fn has_player_changes(&self, player_id: Uuid) -> bool {
        self.player_changes.contains_key(&player_id)
    }

    /// Clear all pending overlays.
    pub fn clear(&mut self) {
        self.ship_changes.clear();
        self.player_changes.clear();
        self.pending_destroys.clear();
    }

    /// Get the count of tracked entities.
    pub fn tracked_count(&self) -> usize {
        self.ship_changes.len() + self.player_changes.len() + self.pending_destroys.len()
    }
}

// ============================================================================
// Merge implementations for Changes structs
// ============================================================================

impl ShipChanges {
    /// Merge another set of changes into this one.
    ///
    /// Later values overwrite earlier ones for the same field.
    pub fn merge(&mut self, other: &ShipChanges) {
        if other.hull.is_some() {
            self.hull = other.hull;
        }
        if other.shields.is_some() {
            self.shields = other.shields;
        }
        if other.ammunition.is_some() {
            self.ammunition = other.ammunition;
        }
        if other.fuel.is_some() {
            self.fuel = other.fuel;
        }
        if other.morale.is_some() {
            self.morale = other.morale;
        }
        if other.experience.is_some() {
            self.experience = other.experience;
        }
        if other.position.is_some() {
            self.position = other.position.clone();
        }
        if other.status.is_some() {
            self.status = other.status.clone();
        }
        if other.combat_stance.is_some() {
            self.combat_stance = other.combat_stance.clone();
        }
        if other.locked_target.is_some() {
            self.locked_target = other.locked_target;
        }
        // Note: cargo and upgrade changes are operations, not state.
        // They're handled by the mutation application, not overlay reads.
        if other.add_cargo.is_some() {
            self.add_cargo = other.add_cargo.clone();
        }
        if other.remove_cargo.is_some() {
            self.remove_cargo = other.remove_cargo.clone();
        }
        if other.install_upgrade.is_some() {
            self.install_upgrade = other.install_upgrade.clone();
        }
        if other.remove_upgrade_slot.is_some() {
            self.remove_upgrade_slot = other.remove_upgrade_slot.clone();
        }
    }
}

impl PlayerChanges {
    /// Merge another set of changes into this one.
    pub fn merge(&mut self, other: &PlayerChanges) {
        if other.reputation.is_some() {
            self.reputation = other.reputation;
        }
        if other.fame.is_some() {
            self.fame = other.fame;
        }
        if other.credits.is_some() {
            self.credits = other.credits;
        }
        // Deltas accumulate
        if let Some(delta) = other.reputation_delta {
            self.reputation_delta = Some(self.reputation_delta.unwrap_or(0) + delta);
        }
        if let Some(delta) = other.fame_delta {
            self.fame_delta = Some(self.fame_delta.unwrap_or(0) + delta);
        }
        if let Some(delta) = other.credits_delta {
            self.credits_delta = Some(self.credits_delta.unwrap_or(0) + delta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ship_snapshot(id: Uuid) -> ShipSnapshot {
        ShipSnapshot {
            id,
            name: "Test Ship".to_string(),
            owner_id: None,
            ship_class: "Corvette".to_string(),
            sector_id: Uuid::new_v4(),
            position: super::super::PositionSnapshot { x: 0.0, y: 0.0, z: 0.0 },
            hull: 100.0,
            shields: 100.0,
            ammunition: 100.0,
            fuel: 100.0,
            morale: 100.0,
            experience: 0,
            status: "idle".to_string(),
            is_player_ship: false,
            faction_id: None,
            can_attack: true,
            can_move: true,
            attack: 10.0,
            defense: 10.0,
            speed: 50.0,
            sensor_range: 100.0,
            combat_stance: "balanced".to_string(),
            locked_target: None,
            cargo: vec![],
            cargo_capacity: 100,
            cargo_used: 0,
            upgrades: vec![],
        }
    }

    #[test]
    fn test_overlay_apply_ship_changes() {
        let mut overlay = MutationOverlay::new();
        let ship_id = Uuid::new_v4();

        overlay.record_ship_change(ship_id, &ShipChanges {
            hull: Some(50.0),
            shields: Some(25.0),
            ..Default::default()
        });

        let snapshot = make_ship_snapshot(ship_id);
        let result = overlay.apply_to_ship(snapshot).unwrap();

        assert_eq!(result.hull, 50.0);
        assert_eq!(result.shields, 25.0);
        assert_eq!(result.fuel, 100.0); // Unchanged
    }

    #[test]
    fn test_overlay_merge_multiple_changes() {
        let mut overlay = MutationOverlay::new();
        let ship_id = Uuid::new_v4();

        // First change
        overlay.record_ship_change(ship_id, &ShipChanges {
            hull: Some(50.0),
            ..Default::default()
        });

        // Second change overwrites hull, adds shields
        overlay.record_ship_change(ship_id, &ShipChanges {
            hull: Some(30.0),
            shields: Some(20.0),
            ..Default::default()
        });

        let snapshot = make_ship_snapshot(ship_id);
        let result = overlay.apply_to_ship(snapshot).unwrap();

        assert_eq!(result.hull, 30.0); // Overwritten
        assert_eq!(result.shields, 20.0);
    }

    #[test]
    fn test_overlay_destroyed_entity() {
        let mut overlay = MutationOverlay::new();
        let ship_id = Uuid::new_v4();

        overlay.record_destroy(ship_id);

        let snapshot = make_ship_snapshot(ship_id);
        let result = overlay.apply_to_ship(snapshot);

        assert!(result.is_none());
        assert!(overlay.is_destroyed(ship_id));
    }

    #[test]
    fn test_player_delta_accumulation() {
        let mut changes = PlayerChanges {
            credits_delta: Some(100),
            ..Default::default()
        };

        changes.merge(&PlayerChanges {
            credits_delta: Some(50),
            ..Default::default()
        });

        assert_eq!(changes.credits_delta, Some(150));
    }
}
