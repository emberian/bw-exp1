//! State API bindings for Rhai
//!
//! Exposes state query and mutation functions to scripts.
//! Uses the unified `ScriptExecutionContext` for state access during script execution.

use std::sync::Arc;
use rhai::{Engine, Dynamic, Map, Array, EvalAltResult, Position as RhaiPos};
use uuid::Uuid;

use bw_core::models::Position;
use bw_game::state::{StateAccessor, ShipChanges, PlayerChanges, ShipSpawnConfig, EntityType, ShipStatusChange, CargoChange, UpgradeInstall};
use bw_game::state::{PropertyWatch, WatchCondition, WatchRegistry};
use crate::errors::{ScriptError, push_error};
use crate::context::{
    with_context, with_accessor,
    current_script_path as ctx_script_path,
};

/// Helper to create a Rhai runtime error.
fn rhai_error(msg: impl Into<String>) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(msg.into().into(), RhaiPos::NONE))
}

/// Helper to record a UUID parsing error and return None.
fn parse_uuid_with_error(value: &str, function: &str) -> Option<Uuid> {
    match Uuid::parse_str(value) {
        Ok(id) => Some(id),
        Err(_) => {
            push_error(ScriptError::new(function, format!("Invalid UUID: {}", value)));
            None
        }
    }
}

/// Get accessor from the unified execution context.
fn get_accessor<T>(f: impl FnOnce(&StateAccessor) -> T) -> Option<T> {
    with_accessor(f)
}

/// Get watch registry from the unified execution context.
fn get_watch_registry() -> Option<Arc<WatchRegistry>> {
    with_context(|ctx| ctx.watch_registry.clone()).flatten()
}

/// Get script path from the unified execution context.
fn get_script_path() -> String {
    ctx_script_path().unwrap_or_else(|| "unknown".to_string())
}

/// Register state API functions with the engine.
pub fn register(engine: &mut Engine) {
    // === Ship queries ===

    // query_ship(ship_id: String) -> Map
    // Returns ship data or throws error if not found
    engine.register_fn("query_ship", |ship_id: String| -> Result<Dynamic, Box<EvalAltResult>> {
        let id = Uuid::parse_str(&ship_id)
            .map_err(|_| rhai_error(format!("Invalid UUID: {}", ship_id)))?;

        get_accessor(|accessor| {
            match accessor.get_ship(id) {
                Ok(Some(ship)) => Ok(ship.to_dynamic()),
                Ok(None) => Err(rhai_error(format!("Ship not found: {}", ship_id))),
                Err(e) => Err(rhai_error(format!("Access error: {}", e))),
            }
        }).unwrap_or_else(|| Err(rhai_error("No state accessor available")))
    });

    // query_ships_in_sector(sector_id: String) -> Array
    // Returns array of ship data
    engine.register_fn("query_ships_in_sector", |sector_id: String| -> Array {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return Array::new(),
        };

        get_accessor(|accessor| {
            match accessor.get_ships_in_sector(id) {
                Ok(ships) => ships.into_iter().map(|s| s.to_dynamic()).collect(),
                Err(e) => {
                    tracing::warn!(script = true, "query_ships_in_sector error: {}", e);
                    Array::new()
                }
            }
        }).unwrap_or_default()
    });

    // query_ships_in_range(sector_id: String, x: f64, y: f64, z: f64, range: f64) -> Array
    engine.register_fn("query_ships_in_range", |sector_id: String, x: f64, y: f64, z: f64, range: f64| -> Array {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return Array::new(),
        };

        let position = Position::new(x, y, z);

        get_accessor(|accessor| {
            match accessor.get_ships_in_range(id, position, range) {
                Ok(ships) => ships.into_iter().map(|s| s.to_dynamic()).collect(),
                Err(e) => {
                    tracing::warn!(script = true, "query_ships_in_range error: {}", e);
                    Array::new()
                }
            }
        }).unwrap_or_default()
    });

    // query_ships_near(x: f64, y: f64, z: f64, range: f64) -> Array
    // Uses context sector
    engine.register_fn("query_ships_near", |x: f64, y: f64, z: f64, range: f64| -> Array {
        let position = Position::new(x, y, z);

        get_accessor(|accessor| {
            match accessor.get_context_sector() {
                Ok(Some(sector)) => {
                    let sector_id = sector.id;
                    match accessor.get_ships_in_range(sector_id, position, range) {
                        Ok(ships) => ships.into_iter().map(|s| s.to_dynamic()).collect(),
                        Err(e) => {
                            tracing::warn!(script = true, "query_ships_near error: {}", e);
                            Array::new()
                        }
                    }
                }
                _ => Array::new(),
            }
        }).unwrap_or_default()
    });

    // === Player queries ===

    // query_player(player_id: String) -> Map
    // Returns player data or throws error if not found
    engine.register_fn("query_player", |player_id: String| -> Result<Dynamic, Box<EvalAltResult>> {
        let id = Uuid::parse_str(&player_id)
            .map_err(|_| rhai_error(format!("Invalid UUID: {}", player_id)))?;

        get_accessor(|accessor| {
            match accessor.get_player(id) {
                Ok(Some(player)) => Ok(player.to_dynamic()),
                Ok(None) => Err(rhai_error(format!("Player not found: {}", player_id))),
                Err(e) => Err(rhai_error(format!("Access error: {}", e))),
            }
        }).unwrap_or_else(|| Err(rhai_error("No state accessor available")))
    });

    // === Sector queries ===

    // query_sector(sector_id: String) -> Map
    engine.register_fn("query_sector", |sector_id: String| -> Dynamic {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return Dynamic::UNIT,
        };

        get_accessor(|accessor| {
            match accessor.get_sector(id) {
                Ok(Some(sector)) => sector.to_dynamic(),
                Ok(None) => Dynamic::UNIT,
                Err(e) => {
                    tracing::warn!(script = true, "query_sector error: {}", e);
                    Dynamic::UNIT
                }
            }
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_context_sector() -> Map
    // Returns the sector the current entity is in
    engine.register_fn("get_context_sector", || -> Dynamic {
        get_accessor(|accessor| {
            match accessor.get_context_sector() {
                Ok(Some(sector)) => sector.to_dynamic(),
                _ => Dynamic::UNIT,
            }
        }).unwrap_or(Dynamic::UNIT)
    });

    // === Ship modifications ===

    // modify_ship(ship_id: String, changes: Map) -> ()
    // Throws error on failure
    engine.register_fn("modify_ship", |ship_id: String, changes: Map| -> Result<(), Box<EvalAltResult>> {
        let id = Uuid::parse_str(&ship_id)
            .map_err(|_| rhai_error(format!("Invalid UUID: {}", ship_id)))?;

        let changes_dyn = Dynamic::from(changes);
        let ship_changes = ShipChanges::from_dynamic(&changes_dyn)
            .ok_or_else(|| rhai_error("Invalid changes format"))?;

        get_accessor(|accessor| {
            accessor.modify_ship(id, ship_changes)
                .map_err(|e| rhai_error(format!("Modification error: {}", e)))
        }).unwrap_or_else(|| Err(rhai_error("No state accessor available")))
    });

    // damage_ship(ship_id: String, amount: f64) -> bool
    // Convenience function to damage a ship
    engine.register_fn("damage_ship", |ship_id: String, amount: f64| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        // Get current hull, then set new hull
        get_accessor(|accessor| {
            let current_hull = accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|s| s.hull)
                .unwrap_or(100.0);

            let new_hull = (current_hull - amount as f32).max(0.0);
            let changes = ShipChanges {
                hull: Some(new_hull),
                ..Default::default()
            };

            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // move_ship_to(ship_id: String, x: f64, y: f64, z: f64) -> bool
    // Set ship to move to a position
    engine.register_fn("move_ship_to", |ship_id: String, x: f64, y: f64, z: f64| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            status: Some(ShipStatusChange::InTransit {
                destination: Position::new(x, y, z),
                target_id: None,
            }),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // === Player modifications ===

    // modify_player(player_id: String, changes: Map) -> bool
    engine.register_fn("modify_player", |player_id: String, changes: Map| -> bool {
        let id = match Uuid::parse_str(&player_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes_dyn = Dynamic::from(changes);
        let player_changes = match PlayerChanges::from_dynamic(&changes_dyn) {
            Some(c) => c,
            None => return false,
        };

        get_accessor(|accessor| {
            match accessor.modify_player(id, player_changes) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(script = true, "modify_player error: {}", e);
                    false
                }
            }
        }).unwrap_or(false)
    });

    // === Entity spawning ===

    // spawn_npc(config: Map) -> String
    // Returns the spawned entity ID or empty string on failure
    engine.register_fn("spawn_npc", |config: Map| -> String {
        get_accessor(|accessor| {
            let sector_id = accessor.get_context_sector()
                .ok()
                .flatten()
                .map(|s| s.id)
                .unwrap_or_else(Uuid::nil);

            let spawn_config = match ShipSpawnConfig::from_dynamic(Dynamic::from(config), sector_id) {
                Some(c) => c,
                None => return String::new(),
            };

            match accessor.spawn_ship(spawn_config) {
                Ok(()) => "pending".to_string(), // ID assigned during apply
                Err(e) => {
                    tracing::warn!(script = true, "spawn_npc error: {}", e);
                    String::new()
                }
            }
        }).unwrap_or_default()
    });

    // === Entity destruction ===

    // destroy_ship(ship_id: String) -> bool
    engine.register_fn("destroy_ship", |ship_id: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        get_accessor(|accessor| {
            accessor.destroy_entity(id, EntityType::Ship).is_ok()
        }).unwrap_or(false)
    });

    // === Event emission ===

    // emit_event(event_type: String, data: Map) -> bool
    engine.register_fn("emit_event", |event_type: String, data: Map| -> bool {
        get_accessor(|accessor| {
            let actor_id = accessor.permissions().owner_entity_id;
            accessor.emit_event(event_type, data, actor_id, None).is_ok()
        }).unwrap_or(false)
    });

    // emit_event_with_target(event_type: String, target_id: String, data: Map) -> bool
    engine.register_fn("emit_event_with_target", |event_type: String, target_id: String, data: Map| -> bool {
        let target = Uuid::parse_str(&target_id).ok();

        get_accessor(|accessor| {
            let actor_id = accessor.permissions().owner_entity_id;
            accessor.emit_event(event_type, data, actor_id, target).is_ok()
        }).unwrap_or(false)
    });

    // === Utility functions ===

    // distance(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> f64
    engine.register_fn("distance", |x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64| -> f64 {
        let dx = x1 - x2;
        let dy = y1 - y2;
        let dz = z1 - z2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    });

    // distance_2d(x1: f64, y1: f64, x2: f64, y2: f64) -> f64
    engine.register_fn("distance_2d", |x1: f64, y1: f64, x2: f64, y2: f64| -> f64 {
        let dx = x1 - x2;
        let dy = y1 - y2;
        (dx * dx + dy * dy).sqrt()
    });

    // distance_to_ship(ship_id: String, x: f64, y: f64, z: f64) -> f64
    engine.register_fn("distance_to_ship", |ship_id: String, x: f64, y: f64, z: f64| -> f64 {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return f64::MAX,
        };

        get_accessor(|accessor| {
            match accessor.get_ship(id) {
                Ok(Some(ship)) => {
                    let dx = ship.position.x - x;
                    let dy = ship.position.y - y;
                    let dz = ship.position.z - z;
                    (dx * dx + dy * dy + dz * dz).sqrt()
                }
                _ => f64::MAX,
            }
        }).unwrap_or(f64::MAX)
    });

    // is_hostile_ship_class(ship_class: String) -> bool
    engine.register_fn("is_hostile_ship_class", |ship_class: String| -> bool {
        matches!(
            ship_class.to_lowercase().as_str(),
            "pirateraider" | "pirate_raider" |
            "piratefrigate" | "pirate_frigate" |
            "terroristbomber" | "terrorist_bomber" |
            "seraswarm" | "sera_swarm" |
            "serahunter" | "sera_hunter" |
            "droneharvester" | "drone_harvester" |
            "droneswarm" | "drone_swarm"
        )
    });

    // === Economy convenience functions ===

    // add_credits(player_id: String, amount: i64) -> bool
    engine.register_fn("add_credits", |player_id: String, amount: i64| -> bool {
        let id = match Uuid::parse_str(&player_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = PlayerChanges {
            credits_delta: Some(amount),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_player(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // spend_credits(player_id: String, amount: i64) -> bool
    // Returns false if not enough credits
    engine.register_fn("spend_credits", |player_id: String, amount: i64| -> bool {
        let id = match Uuid::parse_str(&player_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        get_accessor(|accessor| {
            // First check if player has enough credits
            if let Ok(Some(player)) = accessor.get_player(id) {
                if player.credits >= amount {
                    let changes = PlayerChanges {
                        credits_delta: Some(-amount),
                        ..Default::default()
                    };
                    return accessor.modify_player(id, changes).is_ok();
                }
            }
            false
        }).unwrap_or(false)
    });

    // get_credits(player_id: String) -> i64
    engine.register_fn("get_credits", |player_id: String| -> i64 {
        let id = match Uuid::parse_str(&player_id) {
            Ok(id) => id,
            Err(_) => return 0,
        };

        get_accessor(|accessor| {
            accessor.get_player(id)
                .ok()
                .flatten()
                .map(|p| p.credits)
                .unwrap_or(0)
        }).unwrap_or(0)
    });

    // === Cargo convenience functions ===

    // add_cargo(ship_id: String, cargo_type: String, quantity: i64, price: i64) -> bool
    engine.register_fn("add_cargo", |ship_id: String, cargo_type: String, quantity: i64, price: i64| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            add_cargo: Some(CargoChange {
                cargo_type,
                quantity: quantity as u32,
                purchase_price: price,
            }),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // remove_cargo(ship_id: String, cargo_type: String, quantity: i64) -> bool
    engine.register_fn("remove_cargo", |ship_id: String, cargo_type: String, quantity: i64| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            remove_cargo: Some(CargoChange {
                cargo_type,
                quantity: quantity as u32,
                purchase_price: 0,
            }),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // get_cargo(ship_id: String) -> Array
    engine.register_fn("get_cargo", |ship_id: String| -> Array {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return Array::new(),
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|ship| {
                    ship.cargo.iter().map(|c| {
                        let mut map = Map::new();
                        map.insert("type".into(), Dynamic::from(c.cargo_type.clone()));
                        map.insert("quantity".into(), Dynamic::from(c.quantity as i64));
                        map.insert("price".into(), Dynamic::from(c.purchase_price));
                        Dynamic::from(map)
                    }).collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    });

    // get_cargo_capacity(ship_id: String) -> i64
    engine.register_fn("get_cargo_capacity", |ship_id: String| -> i64 {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return 0,
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|ship| ship.cargo_capacity as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    });

    // get_cargo_used(ship_id: String) -> i64
    engine.register_fn("get_cargo_used", |ship_id: String| -> i64 {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return 0,
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|ship| ship.cargo_used as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    });

    // === Combat stance convenience functions ===

    // set_combat_stance(ship_id: String, stance: String) -> bool
    engine.register_fn("set_combat_stance", |ship_id: String, stance: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            combat_stance: Some(stance),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // get_combat_stance(ship_id: String) -> String
    engine.register_fn("get_combat_stance", |ship_id: String| -> String {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return "balanced".to_string(),
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|ship| format!("{:?}", ship.combat_stance).to_lowercase())
                .unwrap_or_else(|| "balanced".to_string())
        }).unwrap_or_else(|| "balanced".to_string())
    });

    // === Target lock convenience functions ===

    // lock_target(ship_id: String, target_id: String) -> bool
    engine.register_fn("lock_target", |ship_id: String, target_id: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };
        let target = match Uuid::parse_str(&target_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            locked_target: Some(Some(target)),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // clear_target(ship_id: String) -> bool
    engine.register_fn("clear_target", |ship_id: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            locked_target: Some(None),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // get_locked_target(ship_id: String) -> String
    engine.register_fn("get_locked_target", |ship_id: String| -> String {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return String::new(),
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .and_then(|ship| ship.locked_target)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    });

    // === Upgrade management functions ===

    // install_upgrade(ship_id: String, upgrade_id: String, slot: String) -> bool
    engine.register_fn("install_upgrade", |ship_id: String, upgrade_id: String, slot: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            install_upgrade: Some(UpgradeInstall {
                upgrade_id,
                slot,
            }),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // remove_upgrade(ship_id: String, slot: String) -> bool
    engine.register_fn("remove_upgrade", |ship_id: String, slot: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let changes = ShipChanges {
            remove_upgrade_slot: Some(slot),
            ..Default::default()
        };

        get_accessor(|accessor| {
            accessor.modify_ship(id, changes).is_ok()
        }).unwrap_or(false)
    });

    // get_upgrades(ship_id: String) -> Array
    // Returns array of maps with { upgrade_id: String, slot: String }
    engine.register_fn("get_upgrades", |ship_id: String| -> Array {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return Array::new(),
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|ship| {
                    ship.upgrades.iter().map(|u| {
                        let mut map = Map::new();
                        map.insert("upgrade_id".into(), Dynamic::from(u.upgrade_id.clone()));
                        map.insert("slot".into(), Dynamic::from(u.slot.clone()));
                        Dynamic::from(map)
                    }).collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    });

    // has_upgrade(ship_id: String, upgrade_id: String) -> bool
    engine.register_fn("has_upgrade", |ship_id: String, upgrade_id: String| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        get_accessor(|accessor| {
            accessor.get_ship(id)
                .ok()
                .flatten()
                .map(|ship| ship.upgrades.iter().any(|u| u.upgrade_id == upgrade_id))
                .unwrap_or(false)
        }).unwrap_or(false)
    });

    // === Property Watch API ===

    // watch_property(entity_id: String, property: String, condition: String, threshold: f64, callback: String) -> String
    // Returns watch ID or empty string on failure
    engine.register_fn("watch_property", |entity_id: String, property: String, condition: String, threshold: f64, callback: String| -> String {
        let Some(id) = parse_uuid_with_error(&entity_id, "watch_property") else {
            return String::new();
        };

        let Some(registry) = get_watch_registry() else {
            push_error(ScriptError::new("watch_property", "Watch registry not available"));
            tracing::warn!("watch_property called but no registry is set");
            return String::new();
        };

        let cond = match WatchCondition::parse(&condition, Some(threshold)) {
            Some(c) => c,
            None => {
                push_error(ScriptError::new("watch_property", format!("Invalid condition: {}", condition)));
                tracing::warn!("Invalid watch condition: {}", condition);
                return String::new();
            }
        };

        let watch = PropertyWatch::new(id, property, cond, callback, get_script_path());
        let watch_id = registry.register(watch);
        watch_id.to_string()
    });

    // watch_property_changed(entity_id: String, property: String, callback: String) -> String
    // Convenience function for watching any change
    engine.register_fn("watch_property_changed", |entity_id: String, property: String, callback: String| -> String {
        let id = match Uuid::parse_str(&entity_id) {
            Ok(id) => id,
            Err(_) => return String::new(),
        };

        let Some(registry) = get_watch_registry() else {
            tracing::warn!("watch_property_changed called but no registry is set");
            return String::new();
        };

        let watch = PropertyWatch::new(id, property, WatchCondition::Changed, callback, get_script_path());
        let watch_id = registry.register(watch);
        watch_id.to_string()
    });

    // watch_once(entity_id: String, property: String, condition: String, threshold: f64, callback: String) -> String
    // One-shot watch that auto-removes after triggering
    engine.register_fn("watch_once", |entity_id: String, property: String, condition: String, threshold: f64, callback: String| -> String {
        let id = match Uuid::parse_str(&entity_id) {
            Ok(id) => id,
            Err(_) => return String::new(),
        };

        let Some(registry) = get_watch_registry() else {
            tracing::warn!("watch_once called but no registry is set");
            return String::new();
        };

        let cond = match WatchCondition::parse(&condition, Some(threshold)) {
            Some(c) => c,
            None => {
                tracing::warn!("Invalid watch condition: {}", condition);
                return String::new();
            }
        };

        let watch = PropertyWatch::new(id, property, cond, callback, get_script_path()).one_shot();
        let watch_id = registry.register(watch);
        watch_id.to_string()
    });

    // unwatch(watch_id: String) -> bool
    engine.register_fn("unwatch", |watch_id: String| -> bool {
        let id = match Uuid::parse_str(&watch_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let Some(registry) = get_watch_registry() else {
            return false;
        };

        registry.unregister(id)
    });

    // unwatch_all_for_entity(entity_id: String)
    engine.register_fn("unwatch_all_for_entity", |entity_id: String| {
        let id = match Uuid::parse_str(&entity_id) {
            Ok(id) => id,
            Err(_) => return,
        };

        if let Some(registry) = get_watch_registry() {
            registry.unregister_for_entity(id);
        }
    });

    // pause_watch(watch_id: String) -> bool
    engine.register_fn("pause_watch", |watch_id: String| -> bool {
        let id = match Uuid::parse_str(&watch_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        get_watch_registry()
            .map(|r| r.pause(id))
            .unwrap_or(false)
    });

    // resume_watch(watch_id: String) -> bool
    engine.register_fn("resume_watch", |watch_id: String| -> bool {
        let id = match Uuid::parse_str(&watch_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        get_watch_registry()
            .map(|r| r.resume(id))
            .unwrap_or(false)
    });

    // list_watches_for_entity(entity_id: String) -> Array
    engine.register_fn("list_watches_for_entity", |entity_id: String| -> Array {
        let id = match Uuid::parse_str(&entity_id) {
            Ok(id) => id,
            Err(_) => return Array::new(),
        };

        get_watch_registry()
            .map(|r| {
                r.list_for_entity(id)
                    .into_iter()
                    .map(|w| {
                        let mut map = Map::new();
                        map.insert("id".into(), Dynamic::from(w.id.to_string()));
                        map.insert("property".into(), Dynamic::from(w.property));
                        map.insert("callback".into(), Dynamic::from(w.callback));
                        map.insert("active".into(), Dynamic::from(w.active));
                        map.insert("trigger_count".into(), Dynamic::from(w.trigger_count as i64));
                        map.insert("one_shot".into(), Dynamic::from(w.one_shot));
                        Dynamic::from(map)
                    })
                    .collect()
            })
            .unwrap_or_default()
    });

    // get_watch_count() -> i64
    engine.register_fn("get_watch_count", || -> i64 {
        get_watch_registry()
            .map(|r| r.count() as i64)
            .unwrap_or(0)
    });
}
