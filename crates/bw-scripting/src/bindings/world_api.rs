//! World API bindings for Rhai
//!
//! These are stubs that will be connected to actual game state in the server.

use rhai::{Engine, Map};

/// Register world API functions.
pub fn register(engine: &mut Engine) {
    // Spawn NPC ship (returns ship ID, actual spawning done by server)
    engine.register_fn("spawn_npc", |ship_class: String, position_x: f64, position_y: f64| -> Map {
        let mut ship = Map::new();
        ship.insert("id".into(), uuid::Uuid::new_v4().to_string().into());
        ship.insert("class".into(), ship_class.into());
        ship.insert("x".into(), position_x.into());
        ship.insert("y".into(), position_y.into());
        ship.insert("spawned".into(), true.into());
        ship
    });

    // Mark NPC for despawn
    engine.register_fn("despawn_npc", |ship_id: String| -> Map {
        let mut result = Map::new();
        result.insert("id".into(), ship_id.into());
        result.insert("despawned".into(), true.into());
        result
    });

    // Get sector danger level (stub)
    engine.register_fn("get_sector_danger", |_sector_id: String| -> String {
        "Moderate".to_string()
    });

    // Check if position is near location
    engine.register_fn("is_near_location", |
        x: f64,
        y: f64,
        loc_x: f64,
        loc_y: f64,
        radius: f64,
    | -> bool {
        let dx = x - loc_x;
        let dy = y - loc_y;
        (dx * dx + dy * dy).sqrt() <= radius
    });

    // Calculate distance
    engine.register_fn("distance", |x1: f64, y1: f64, x2: f64, y2: f64| -> f64 {
        let dx = x1 - x2;
        let dy = y1 - y2;
        (dx * dx + dy * dy).sqrt()
    });

    // Generate random position in bounds
    engine.register_fn("random_position", |min_x: f64, max_x: f64, min_y: f64, max_y: f64| -> Map {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut pos = Map::new();
        pos.insert("x".into(), rng.gen_range(min_x..max_x).into());
        pos.insert("y".into(), rng.gen_range(min_y..max_y).into());
        pos
    });

    // Time utilities
    engine.register_fn("current_tick", || -> i64 {
        // Placeholder - actual tick provided by server
        0
    });

    // Create timer (returns when to trigger, actual tracking by server)
    engine.register_fn("create_timer", |delay_ticks: i64| -> Map {
        let mut timer = Map::new();
        timer.insert("trigger_at".into(), delay_ticks.into());
        timer.insert("id".into(), uuid::Uuid::new_v4().to_string().into());
        timer
    });
}
