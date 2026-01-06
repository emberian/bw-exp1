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
pub mod effect_api;
pub mod data_api;

use rhai::Engine;
use tracing::{debug, info, instrument, trace};


/// Register all API bindings with the engine.
#[instrument(level = "debug", skip_all)]
pub fn register_all(engine: &mut Engine) {
    debug!("Registering all script API bindings");

    trace!("Registering ship_api bindings");
    ship_api::register(engine);

    trace!("Registering mission_api bindings");
    mission_api::register(engine);

    trace!("Registering combat_api bindings");
    combat_api::register(engine);

    trace!("Registering world_api bindings");
    world_api::register(engine);

    trace!("Registering state_api bindings");
    state_api::register(engine);

    trace!("Registering coroutine_api bindings");
    coroutine_api::register(engine);

    trace!("Registering event_api bindings");
    event_api::register(engine);

    trace!("Registering action_api bindings");
    action_api::register(engine);

    trace!("Registering effect_api bindings");
    effect_api::register(engine);

    trace!("Registering data_api bindings");
    data_api::register(engine);

    trace!("Registering message_api bindings");
    message_api::register(engine);

    // Register facade views for fluent script API
    trace!("Registering view facade bindings");
    crate::views::register(engine);

    // Register transaction support for atomic operations
    trace!("Registering transaction bindings");
    crate::transaction::register(engine);

    // Register AI system (behavior trees + utility AI)
    trace!("Registering AI bindings");
    bw_ai::register_ai_bindings(engine);

    // Register persistence system for saving/loading script state
    trace!("Registering persistence bindings");
    crate::persistence::register_persistence_bindings(engine);

    // Register common utility functions
    trace!("Registering utility functions");
    register_utils(engine);

    info!("All script API bindings registered");
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
