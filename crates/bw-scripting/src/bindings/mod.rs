//! Rhai bindings for game APIs
//!
//! These bindings expose game functionality to Rhai scripts.

mod ship_api;
mod mission_api;
mod combat_api;
mod world_api;
mod message_api;
pub mod state_api;
pub mod coroutine_api;
pub mod event_api;
pub mod action_api;
pub mod data_api;

use rhai::Engine;


/// Register all API bindings with the engine.
pub fn register_all(engine: &mut Engine) {
    ship_api::register(engine);
    mission_api::register(engine);
    combat_api::register(engine);
    world_api::register(engine);
    state_api::register(engine);
    coroutine_api::register(engine);
    event_api::register(engine);
    action_api::register(engine);
    data_api::register(engine);
    message_api::register(engine);

    // Register facade views for fluent script API
    crate::views::register(engine);

    // Register transaction support for atomic operations
    crate::transaction::register(engine);

    // Register AI system (behavior trees + utility AI)
    crate::ai::register_ai_bindings(engine);

    // Register persistence system for saving/loading script state
    crate::persistence::register_persistence_bindings(engine);

    // Register common utility functions
    register_utils(engine);
}

fn register_utils(engine: &mut Engine) {
    // Random number generation
    engine.register_fn("rand", || -> f64 {
        use rand::Rng;
        rand::thread_rng().r#gen()
    });

    engine.register_fn("rand_int", |min: i64, max: i64| -> i64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    });

    engine.register_fn("rand_float", |min: f64, max: f64| -> f64 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    });

    // Math utilities
    engine.register_fn("clamp", |value: f64, min: f64, max: f64| -> f64 {
        value.clamp(min, max)
    });

    engine.register_fn("clamp_int", |value: i64, min: i64, max: i64| -> i64 {
        value.clamp(min, max)
    });

    engine.register_fn("lerp", |a: f64, b: f64, t: f64| -> f64 {
        a + (b - a) * t.clamp(0.0, 1.0)
    });

    // String utilities
    engine.register_fn("uuid", || -> String {
        uuid::Uuid::new_v4().to_string()
    });

    // Logging
    engine.register_fn("log_info", |msg: &str| {
        tracing::info!(script = true, "{}", msg);
    });

    engine.register_fn("log_warn", |msg: &str| {
        tracing::warn!(script = true, "{}", msg);
    });

    engine.register_fn("log_error", |msg: &str| {
        tracing::error!(script = true, "{}", msg);
    });

    // Error inspection for scripts
    // Scripts can check for errors after API calls that might fail
    engine.register_fn("last_error", || -> String {
        crate::errors::last_error_message()
    });

    engine.register_fn("has_error", || -> bool {
        crate::errors::has_errors()
    });

    engine.register_fn("clear_errors", || {
        crate::errors::clear_errors();
    });

    engine.register_fn("error_count", || -> i64 {
        crate::errors::error_count() as i64
    });
}
