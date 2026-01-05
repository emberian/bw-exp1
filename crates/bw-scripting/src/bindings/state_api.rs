//! State API bindings for Rhai
//!
//! Exposes state query and mutation functions to scripts.
//! Uses thread-local accessor for state access during script execution.
//!
//! # Thread Safety
//!
//! The thread-local pattern is safe because:
//! 1. Rhai script execution is synchronous (no await points during execution)
//! 2. Callers must hold appropriate locks (e.g., BehaviorManager's RwLock)
//! 3. A guard flag prevents re-entrant script execution on the same thread
//!
//! Callers MUST NOT call script execution concurrently from multiple async tasks
//! that might run on the same thread without synchronization.

use std::cell::RefCell;
use std::sync::Arc;
use rhai::{Engine, Dynamic, Map, Array};
use uuid::Uuid;

use bw_core::models::Position;
use crate::state::{StateAccessor, ShipChanges, PlayerChanges, ShipSpawnConfig, EntityType};

thread_local! {
    /// Thread-local state accessor for the currently executing script.
    static CURRENT_ACCESSOR: RefCell<Option<Arc<StateAccessor>>> = const { RefCell::new(None) };
    /// Guard to detect re-entrant script execution
    static SCRIPT_EXECUTING: RefCell<bool> = const { RefCell::new(false) };
}

/// Set the state accessor for the current thread during script execution.
///
/// # Panics
///
/// Panics if called while another script is already executing on this thread,
/// which would indicate incorrect usage (potential data race).
pub fn set_current_accessor(accessor: Arc<StateAccessor>) {
    SCRIPT_EXECUTING.with(|guard| {
        let mut executing = guard.borrow_mut();
        if *executing {
            panic!("Re-entrant script execution detected! This indicates a bug - scripts should not be executed concurrently on the same thread.");
        }
        *executing = true;
    });

    CURRENT_ACCESSOR.with(|cell| {
        *cell.borrow_mut() = Some(accessor);
    });
}

/// Clear the state accessor after script execution.
pub fn clear_current_accessor() {
    CURRENT_ACCESSOR.with(|cell| {
        *cell.borrow_mut() = None;
    });

    SCRIPT_EXECUTING.with(|guard| {
        *guard.borrow_mut() = false;
    });
}

/// Get the current accessor (panics if not set).
fn with_accessor<T, F: FnOnce(&StateAccessor) -> T>(f: F) -> Option<T> {
    CURRENT_ACCESSOR.with(|cell| {
        cell.borrow().as_ref().map(|accessor| f(accessor))
    })
}

/// Register state API functions with the engine.
pub fn register(engine: &mut Engine) {
    // === Ship queries ===

    // query_ship(ship_id: String) -> Map
    // Returns ship data or empty map if not found
    engine.register_fn("query_ship", |ship_id: String| -> Dynamic {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return Dynamic::UNIT,
        };

        with_accessor(|accessor| {
            match accessor.get_ship(id) {
                Ok(Some(ship)) => ship.to_dynamic(),
                Ok(None) => Dynamic::UNIT,
                Err(e) => {
                    tracing::warn!(script = true, "query_ship error: {}", e);
                    Dynamic::UNIT
                }
            }
        }).unwrap_or(Dynamic::UNIT)
    });

    // query_ships_in_sector(sector_id: String) -> Array
    // Returns array of ship data
    engine.register_fn("query_ships_in_sector", |sector_id: String| -> Array {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return Array::new(),
        };

        with_accessor(|accessor| {
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

        with_accessor(|accessor| {
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

        with_accessor(|accessor| {
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
    engine.register_fn("query_player", |player_id: String| -> Dynamic {
        let id = match Uuid::parse_str(&player_id) {
            Ok(id) => id,
            Err(_) => return Dynamic::UNIT,
        };

        with_accessor(|accessor| {
            match accessor.get_player(id) {
                Ok(Some(player)) => player.to_dynamic(),
                Ok(None) => Dynamic::UNIT,
                Err(e) => {
                    tracing::warn!(script = true, "query_player error: {}", e);
                    Dynamic::UNIT
                }
            }
        }).unwrap_or(Dynamic::UNIT)
    });

    // === Sector queries ===

    // query_sector(sector_id: String) -> Map
    engine.register_fn("query_sector", |sector_id: String| -> Dynamic {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return Dynamic::UNIT,
        };

        with_accessor(|accessor| {
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
        with_accessor(|accessor| {
            match accessor.get_context_sector() {
                Ok(Some(sector)) => sector.to_dynamic(),
                _ => Dynamic::UNIT,
            }
        }).unwrap_or(Dynamic::UNIT)
    });

    // === Ship modifications ===

    // modify_ship(ship_id: String, changes: Map) -> bool
    engine.register_fn("modify_ship", |ship_id: String, changes: Map| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        let ship_changes = match ShipChanges::from_dynamic(Dynamic::from(changes)) {
            Some(c) => c,
            None => return false,
        };

        with_accessor(|accessor| {
            match accessor.modify_ship(id, ship_changes) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(script = true, "modify_ship error: {}", e);
                    false
                }
            }
        }).unwrap_or(false)
    });

    // damage_ship(ship_id: String, amount: f64) -> bool
    // Convenience function to damage a ship
    engine.register_fn("damage_ship", |ship_id: String, amount: f64| -> bool {
        let id = match Uuid::parse_str(&ship_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        // Get current hull, then set new hull
        with_accessor(|accessor| {
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
            status: Some(crate::state::ShipStatusChange::InTransit {
                destination: Position::new(x, y, z),
                target_id: None,
            }),
            ..Default::default()
        };

        with_accessor(|accessor| {
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

        let player_changes = match PlayerChanges::from_dynamic(Dynamic::from(changes)) {
            Some(c) => c,
            None => return false,
        };

        with_accessor(|accessor| {
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
        with_accessor(|accessor| {
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

        with_accessor(|accessor| {
            accessor.destroy_entity(id, EntityType::Ship).is_ok()
        }).unwrap_or(false)
    });

    // === Event emission ===

    // emit_event(event_type: String, data: Map) -> bool
    engine.register_fn("emit_event", |event_type: String, data: Map| -> bool {
        with_accessor(|accessor| {
            let actor_id = accessor.permissions().owner_entity_id;
            accessor.emit_event(event_type, data, actor_id, None).is_ok()
        }).unwrap_or(false)
    });

    // emit_event_with_target(event_type: String, target_id: String, data: Map) -> bool
    engine.register_fn("emit_event_with_target", |event_type: String, target_id: String, data: Map| -> bool {
        let target = Uuid::parse_str(&target_id).ok();

        with_accessor(|accessor| {
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

        with_accessor(|accessor| {
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
}
