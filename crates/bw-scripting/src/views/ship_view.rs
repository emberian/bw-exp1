//! ShipView facade for scripts
//!
//! Provides fluent property access for ships:
//! ```rhai
//! if ship.hull < 25.0 {
//!     ship.notify("Critical damage!");
//! }
//! ship.add_cargo("fuel", 10, 50);
//! ```

use rhai::{Dynamic, Engine, Array, Map, CustomType, TypeBuilder};
use uuid::Uuid;

use crate::context::with_accessor;
use crate::state::{ShipChanges, ShipStatusChange, CargoChange, UpgradeInstall};

/// A facade view of a ship for scripts.
///
/// Provides natural property access that reads from snapshots
/// and queues mutations for later application.
#[derive(Debug, Clone)]
pub struct ShipView {
    pub(crate) id: Uuid,
}

impl ShipView {
    /// Create a new ship view.
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    /// Create from a string UUID.
    pub fn from_string(id: &str) -> Option<Self> {
        Uuid::parse_str(id).ok().map(Self::new)
    }

    // =========================================================================
    // Getters - read from snapshot via accessor
    // =========================================================================

    /// Get ship ID as string.
    pub fn get_id(&mut self) -> String {
        self.id.to_string()
    }

    // -------------------------------------------------------------------------
    // Generated getters using macros
    // -------------------------------------------------------------------------

    ship_getters! {
        /// Get ship name.
        pub fn get_name(&mut self) -> String { get_string, name };

        /// Get ship class.
        pub fn get_ship_class(&mut self) -> String { get_string, ship_class };

        /// Get current sector ID.
        pub fn get_sector_id(&mut self) -> String { get_uuid_string, sector_id };

        /// Get owner ID (player UUID or empty for NPCs).
        pub fn get_owner_id(&mut self) -> String { get_option_uuid_string, owner_id };

        /// Get hull integrity (0-100).
        pub fn get_hull(&mut self) -> f64 { get_f64, hull };

        /// Get shield strength (0-100).
        pub fn get_shields(&mut self) -> f64 { get_f64, shields };

        /// Get ammunition (0-100).
        pub fn get_ammunition(&mut self) -> f64 { get_f64, ammunition };

        /// Get fuel (0-100).
        pub fn get_fuel(&mut self) -> f64 { get_f64, fuel };

        /// Get crew morale (0-100).
        pub fn get_morale(&mut self) -> f64 { get_f64, morale }
    }

    /// Get crew experience.
    pub fn get_experience(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.experience as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get ship status (idle, in_transit, in_combat, docked, disabled, destroyed).
    pub fn get_status(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.status.clone())
                .unwrap_or_else(|| "unknown".to_string())
        }).unwrap_or_else(|| "unknown".to_string())
    }

    /// Check if this is a player ship.
    pub fn get_is_player_ship(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.is_player_ship)
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Get faction ID.
    pub fn get_faction_id(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .and_then(|s| s.faction_id)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Check if ship can attack.
    pub fn get_can_attack(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.can_attack)
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Check if ship can move.
    pub fn get_can_move(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.can_move)
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Get attack stat.
    pub fn get_attack(&mut self) -> f64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.attack as f64)
                .unwrap_or(0.0)
        }).unwrap_or(0.0)
    }

    /// Get defense stat.
    pub fn get_defense(&mut self) -> f64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.defense as f64)
                .unwrap_or(0.0)
        }).unwrap_or(0.0)
    }

    /// Get speed stat.
    pub fn get_speed(&mut self) -> f64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.speed as f64)
                .unwrap_or(0.0)
        }).unwrap_or(0.0)
    }

    /// Get sensor range.
    pub fn get_sensor_range(&mut self) -> f64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.sensor_range as f64)
                .unwrap_or(0.0)
        }).unwrap_or(0.0)
    }

    /// Get combat stance.
    pub fn get_combat_stance(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.combat_stance.clone())
                .unwrap_or_else(|| "balanced".to_string())
        }).unwrap_or_else(|| "balanced".to_string())
    }

    /// Get locked target ID.
    pub fn get_locked_target(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .and_then(|s| s.locked_target)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get position as a map with x, y, z.
    pub fn get_position(&mut self) -> Map {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| {
                    let mut map = Map::new();
                    map.insert("x".into(), Dynamic::from(s.position.x));
                    map.insert("y".into(), Dynamic::from(s.position.y));
                    map.insert("z".into(), Dynamic::from(s.position.z));
                    map
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get cargo capacity.
    pub fn get_cargo_capacity(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.cargo_capacity as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get cargo used.
    pub fn get_cargo_used(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.cargo_used as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get available cargo space.
    pub fn get_cargo_free(&mut self) -> i64 {
        self.get_cargo_capacity() - self.get_cargo_used()
    }

    /// Get cargo as array of maps.
    pub fn get_cargo(&mut self) -> Array {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| {
                    s.cargo.iter()
                        .map(|c| c.to_dynamic())
                        .collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get upgrades as array of maps.
    pub fn get_upgrades(&mut self) -> Array {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| {
                    s.upgrades.iter()
                        .map(|u| u.to_dynamic())
                        .collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    // =========================================================================
    // Setters - queue mutations
    // =========================================================================

    // -------------------------------------------------------------------------
    // Generated setters using macros
    // -------------------------------------------------------------------------

    ship_setters! {
        /// Set hull integrity (clamped 0-100).
        pub fn set_hull(&mut self, value: f64) { clamp_f32, hull };

        /// Set shield strength (clamped 0-100).
        pub fn set_shields(&mut self, value: f64) { clamp_f32, shields }
    }

    /// Set fuel (clamped 0-100).
    pub fn set_fuel(&mut self, value: f64) {
        let clamped = value.clamp(0.0, 100.0) as f32;
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                fuel: Some(clamped),
                ..Default::default()
            });
        });
    }

    /// Set ammunition (clamped 0-100).
    pub fn set_ammunition(&mut self, value: f64) {
        let clamped = value.clamp(0.0, 100.0) as f32;
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                ammunition: Some(clamped),
                ..Default::default()
            });
        });
    }

    /// Set crew morale (clamped 0-100).
    pub fn set_morale(&mut self, value: f64) {
        let clamped = value.clamp(0.0, 100.0) as f32;
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                morale: Some(clamped),
                ..Default::default()
            });
        });
    }

    /// Set crew experience.
    pub fn set_experience(&mut self, value: i64) {
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                experience: Some(value as i32),
                ..Default::default()
            });
        });
    }

    /// Set combat stance.
    pub fn set_combat_stance(&mut self, stance: String) {
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                combat_stance: Some(stance),
                ..Default::default()
            });
        });
    }

    // =========================================================================
    // Methods - safe operations that queue mutations
    // =========================================================================

    /// Apply damage to the ship (reduces hull, returns amount of damage dealt).
    pub fn damage(&mut self, amount: f64) -> f64 {
        let current = self.get_hull();
        let new_hull = (current - amount).max(0.0);
        self.set_hull(new_hull);
        current - new_hull
    }

    /// Repair the ship (increases hull, returns amount healed).
    pub fn repair(&mut self, amount: f64) -> f64 {
        let current = self.get_hull();
        let new_hull = (current + amount).min(100.0);
        self.set_hull(new_hull);
        new_hull - current
    }

    /// Consume fuel (returns true if enough fuel was available).
    pub fn consume_fuel(&mut self, amount: f64) -> bool {
        let current = self.get_fuel();
        if current >= amount {
            self.set_fuel(current - amount);
            true
        } else {
            false
        }
    }

    /// Consume ammunition (returns true if enough ammo was available).
    pub fn consume_ammo(&mut self, amount: f64) -> bool {
        let current = self.get_ammunition();
        if current >= amount {
            self.set_ammunition(current - amount);
            true
        } else {
            false
        }
    }

    /// Lock onto a target.
    pub fn lock_target(&mut self, target_id: String) {
        if let Ok(uuid) = Uuid::parse_str(&target_id) {
            with_accessor(|accessor| {
                let _ = accessor.modify_ship(self.id, ShipChanges {
                    locked_target: Some(Some(uuid)),
                    ..Default::default()
                });
            });
        }
    }

    /// Clear target lock.
    pub fn clear_target(&mut self) {
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                locked_target: Some(None),
                ..Default::default()
            });
        });
    }

    /// Add cargo to the ship.
    pub fn add_cargo(&mut self, cargo_type: String, quantity: i64, price: i64) -> bool {
        if quantity <= 0 {
            return false;
        }
        with_accessor(|accessor| {
            accessor.modify_ship(self.id, ShipChanges {
                add_cargo: Some(CargoChange {
                    cargo_type,
                    quantity: quantity as u32,
                    purchase_price: price,
                }),
                ..Default::default()
            }).is_ok()
        }).unwrap_or(false)
    }

    /// Remove cargo from the ship.
    pub fn remove_cargo(&mut self, cargo_type: String, quantity: i64) -> bool {
        if quantity <= 0 {
            return false;
        }
        with_accessor(|accessor| {
            accessor.modify_ship(self.id, ShipChanges {
                remove_cargo: Some(CargoChange {
                    cargo_type,
                    quantity: quantity as u32,
                    purchase_price: 0,
                }),
                ..Default::default()
            }).is_ok()
        }).unwrap_or(false)
    }

    /// Get quantity of a specific cargo type.
    pub fn cargo_quantity(&mut self, cargo_type: String) -> i64 {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.cargo.iter()
                        .find(|c| c.cargo_type == cargo_type)
                        .map(|c| c.quantity as i64)
                })
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Install an upgrade.
    pub fn install_upgrade(&mut self, upgrade_id: String, slot: String) -> bool {
        with_accessor(|accessor| {
            accessor.modify_ship(self.id, ShipChanges {
                install_upgrade: Some(UpgradeInstall { upgrade_id, slot }),
                ..Default::default()
            }).is_ok()
        }).unwrap_or(false)
    }

    /// Remove an upgrade by slot.
    pub fn remove_upgrade(&mut self, slot: String) -> bool {
        with_accessor(|accessor| {
            accessor.modify_ship(self.id, ShipChanges {
                remove_upgrade_slot: Some(slot),
                ..Default::default()
            }).is_ok()
        }).unwrap_or(false)
    }

    /// Check if ship has a specific upgrade.
    pub fn has_upgrade(&mut self, upgrade_id: String) -> bool {
        with_accessor(|accessor| {
            accessor.get_ship(self.id)
                .ok()
                .flatten()
                .map(|s| s.upgrades.iter().any(|u| u.upgrade_id == upgrade_id))
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Move to a position.
    pub fn move_to(&mut self, x: f64, y: f64, z: f64) {
        use bw_core::models::Position;
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                status: Some(ShipStatusChange::InTransit {
                    destination: Position::new(x, y, z),
                    target_id: None,
                }),
                ..Default::default()
            });
        });
    }

    /// Set status to idle.
    pub fn stop(&mut self) {
        with_accessor(|accessor| {
            let _ = accessor.modify_ship(self.id, ShipChanges {
                status: Some(ShipStatusChange::Idle),
                ..Default::default()
            });
        });
    }

    /// Check if ship exists.
    pub fn exists(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_ship(self.id).ok().flatten().is_some()
        }).unwrap_or(false)
    }

    /// Check if ship is destroyed (hull <= 0).
    pub fn is_destroyed(&mut self) -> bool {
        self.get_hull() <= 0.0
    }
}

// Implement CustomType for automatic registration
impl CustomType for ShipView {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("ShipView")
            .with_get("id", Self::get_id)
            .with_get("name", Self::get_name)
            .with_get("owner_id", Self::get_owner_id)
            .with_get("ship_class", Self::get_ship_class)
            .with_get("sector_id", Self::get_sector_id)
            .with_get_set("hull", Self::get_hull, Self::set_hull)
            .with_get_set("shields", Self::get_shields, Self::set_shields)
            .with_get_set("ammunition", Self::get_ammunition, Self::set_ammunition)
            .with_get_set("fuel", Self::get_fuel, Self::set_fuel)
            .with_get_set("morale", Self::get_morale, Self::set_morale)
            .with_get_set("experience", Self::get_experience, Self::set_experience)
            .with_get("status", Self::get_status)
            .with_get("is_player_ship", Self::get_is_player_ship)
            .with_get("faction_id", Self::get_faction_id)
            .with_get("can_attack", Self::get_can_attack)
            .with_get("can_move", Self::get_can_move)
            .with_get("attack", Self::get_attack)
            .with_get("defense", Self::get_defense)
            .with_get("speed", Self::get_speed)
            .with_get("sensor_range", Self::get_sensor_range)
            .with_get_set("combat_stance", Self::get_combat_stance, Self::set_combat_stance)
            .with_get("locked_target", Self::get_locked_target)
            .with_get("position", Self::get_position)
            .with_get("cargo_capacity", Self::get_cargo_capacity)
            .with_get("cargo_used", Self::get_cargo_used)
            .with_get("cargo_free", Self::get_cargo_free)
            .with_get("cargo", Self::get_cargo)
            .with_get("upgrades", Self::get_upgrades);
    }
}

/// Register ShipView with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // Register the type with automatic getters/setters
    engine.build_type::<ShipView>();

    // Register methods
    engine.register_fn("damage", ShipView::damage);
    engine.register_fn("repair", ShipView::repair);
    engine.register_fn("consume_fuel", ShipView::consume_fuel);
    engine.register_fn("consume_ammo", ShipView::consume_ammo);
    engine.register_fn("lock_target", ShipView::lock_target);
    engine.register_fn("clear_target", ShipView::clear_target);
    engine.register_fn("add_cargo", ShipView::add_cargo);
    engine.register_fn("remove_cargo", ShipView::remove_cargo);
    engine.register_fn("cargo_quantity", ShipView::cargo_quantity);
    engine.register_fn("install_upgrade", ShipView::install_upgrade);
    engine.register_fn("remove_upgrade", ShipView::remove_upgrade);
    engine.register_fn("has_upgrade", ShipView::has_upgrade);
    engine.register_fn("move_to", ShipView::move_to);
    engine.register_fn("stop", ShipView::stop);
    engine.register_fn("exists", ShipView::exists);
    engine.register_fn("is_destroyed", ShipView::is_destroyed);

    // Constructor from string ID
    engine.register_fn("ship", |id: String| -> Dynamic {
        match ShipView::from_string(&id) {
            Some(view) => Dynamic::from(view),
            None => Dynamic::UNIT,
        }
    });
}
