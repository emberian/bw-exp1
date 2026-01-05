//! Ship API bindings for Rhai

use rhai::{Engine, Map};

/// Register ship API functions.
pub fn register(engine: &mut Engine) {
    // Ship stat getters (from context)
    engine.register_fn("get_ship_hull", |ctx: Map| -> f64 {
        ctx.get("ship_hull")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    engine.register_fn("get_ship_ammo", |ctx: Map| -> f64 {
        ctx.get("ship_ammo")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    engine.register_fn("get_ship_fuel", |ctx: Map| -> f64 {
        ctx.get("ship_fuel")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    engine.register_fn("get_crew_morale", |ctx: Map| -> f64 {
        ctx.get("crew_morale")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(50.0)
    });

    engine.register_fn("get_crew_experience", |ctx: Map| -> i64 {
        ctx.get("crew_experience")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    // Ship status checks
    engine.register_fn("is_ship_damaged", |ctx: Map| -> bool {
        let hull = ctx.get("ship_hull")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0);
        hull < 100.0
    });

    engine.register_fn("is_ship_critical", |ctx: Map| -> bool {
        let hull = ctx.get("ship_hull")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0);
        hull < 25.0
    });

    engine.register_fn("is_ammo_low", |ctx: Map| -> bool {
        let ammo = ctx.get("ship_ammo")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0);
        ammo < 15.0
    });

    engine.register_fn("is_fuel_low", |ctx: Map| -> bool {
        let fuel = ctx.get("ship_fuel")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0);
        fuel < 10.0
    });

    engine.register_fn("is_morale_low", |ctx: Map| -> bool {
        let morale = ctx.get("crew_morale")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(50.0);
        morale < 30.0
    });

    // Experience-based success chance calculation
    engine.register_fn("experience_success_chance", |base_chance: f64, experience: i64| -> f64 {
        let exp_bonus = (experience as f64).sqrt() / 100.0;
        (base_chance + exp_bonus).clamp(0.05, 0.95)
    });

    // Morale modifier calculation
    engine.register_fn("morale_modifier", |morale: f64| -> f64 {
        (morale - 50.0) / 200.0 // -0.25 to +0.25
    });
}
