//! Mission API bindings for Rhai
//!
//! Provides ergonomic helpers for writing mission scripts.

use rhai::{Engine, Dynamic, Map, Array};

/// Register mission API functions.
pub fn register(engine: &mut Engine) {
    // ==========================================================================
    // Context Accessors
    // ==========================================================================

    // Get mission data
    engine.register_fn("get_mission_state", |ctx: Map| -> String {
        ctx.get("current_state")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_else(|| "start".to_string())
    });

    engine.register_fn("get_mission_data", |ctx: Map| -> Map {
        ctx.get("data")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap_or_default()
    });

    // Get crew experience from context
    engine.register_fn("get_crew_experience", |ctx: Map| -> i64 {
        ctx.get("crew_experience")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    // Get player fame from context
    engine.register_fn("get_player_fame", |ctx: Map| -> i64 {
        ctx.get("player_fame")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    // Get player reputation from context
    engine.register_fn("get_player_reputation", |ctx: Map| -> i64 {
        ctx.get("player_reputation")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(0)
    });

    // Get ship hull from context
    engine.register_fn("get_ship_hull", |ctx: Map| -> f64 {
        ctx.get("ship_hull")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    // Get ship ammo from context
    engine.register_fn("get_ship_ammo", |ctx: Map| -> f64 {
        ctx.get("ship_ammo")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    // Get ship fuel from context
    engine.register_fn("get_ship_fuel", |ctx: Map| -> f64 {
        ctx.get("ship_fuel")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    // Get crew morale from context
    engine.register_fn("get_crew_morale", |ctx: Map| -> f64 {
        ctx.get("crew_morale")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(100.0)
    });

    // ==========================================================================
    // Success Chance Calculations
    // ==========================================================================

    // Calculate success chance based on experience
    // Each 10 experience adds 1% success chance, capped at 90%
    engine.register_fn("experience_success_chance", |base_chance: f64, experience: i64| -> f64 {
        let bonus = (experience as f64 / 10.0) * 0.01;
        let total = base_chance + bonus;
        total.min(0.9)
    });

    // Calculate success chance based on fame (reputation intimidation)
    engine.register_fn("fame_success_chance", |base_chance: f64, fame: i64| -> f64 {
        let bonus = fame as f64 / 200.0;
        let total = base_chance + bonus;
        total.clamp(0.2, 0.8)
    });

    // ==========================================================================
    // Result Builders (Shorthand)
    // ==========================================================================

    // Ergonomic success result: success(rep, fame, narrative)
    engine.register_fn("success", |reputation: i64, fame: i64, narrative: String| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), true.into());
        result.insert("new_state".into(), "complete".into());
        result.insert("reputation_change".into(), reputation.into());
        result.insert("fame_change".into(), fame.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), false.into());
        result
    });

    // Ergonomic failure result: failure(rep_loss, narrative)
    engine.register_fn("failure", |reputation_loss: i64, narrative: String| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), false.into());
        result.insert("new_state".into(), "complete".into());
        result.insert("reputation_change".into(), (-reputation_loss).into());
        result.insert("fame_change".into(), 0i64.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), false.into());
        result
    });

    // Ergonomic combat result: combat(narrative)
    engine.register_fn("combat", |narrative: String| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), false.into());
        result.insert("new_state".into(), "combat".into());
        result.insert("reputation_change".into(), 0i64.into());
        result.insert("fame_change".into(), 0i64.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), true.into());
        result
    });

    // Ergonomic choice result: choices(state, narrative, choices_array)
    engine.register_fn("choices", |state: String, narrative: String, choices: Array| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), false.into());
        result.insert("new_state".into(), state.into());
        result.insert("reputation_change".into(), 0i64.into());
        result.insert("fame_change".into(), 0i64.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), false.into());
        result.insert("choices".into(), Dynamic::from(choices));
        result
    });

    // Ergonomic choice builder: choice(id, text)
    engine.register_fn("choice", |id: String, text: String| -> Map {
        let mut choice = Map::new();
        choice.insert("id".into(), id.into());
        choice.insert("text".into(), text.into());
        choice.insert("requirements".into(), Dynamic::from(Array::new()));
        choice
    });

    // Ergonomic gated choice: gated_choice(id, text, req_type, value)
    engine.register_fn("gated_choice", |id: String, text: String, req_type: String, req_value: i64| -> Map {
        let mut choice = Map::new();
        choice.insert("id".into(), id.into());
        choice.insert("text".into(), text.into());

        let mut req = Map::new();
        req.insert("type".into(), req_type.clone().into());
        req.insert("value".into(), req_value.into());

        // Use the conventional field name
        let field_name = match req_type.as_str() {
            "MinExperience" => "min_experience",
            "MinFame" => "min_fame",
            "MinReputation" => "min_reputation",
            "MinHull" => "min_hull",
            "MinAmmo" => "min_ammo",
            "MinFuel" => "min_fuel",
            _ => "custom",
        };

        let mut req_map = Map::new();
        req_map.insert(field_name.into(), req_value.into());

        let mut reqs = Array::new();
        reqs.push(Dynamic::from(req_map));
        choice.insert("requirements".into(), Dynamic::from(reqs));

        choice
    });

    // ==========================================================================
    // Original Verbose Builders (kept for backwards compatibility)
    // ==========================================================================

    // Create result builders
    engine.register_fn("create_success_result", |reputation: i64, fame: i64, narrative: String| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), true.into());
        result.insert("new_state".into(), "complete".into());
        result.insert("reputation_change".into(), reputation.into());
        result.insert("fame_change".into(), fame.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), false.into());
        result
    });

    engine.register_fn("create_failure_result", |reputation_penalty: i64, narrative: String| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), false.into());
        result.insert("new_state".into(), "complete".into());
        result.insert("reputation_change".into(), (-reputation_penalty).into());
        result.insert("fame_change".into(), 0i64.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), false.into());
        result
    });

    engine.register_fn("create_combat_result", |narrative: String| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), false.into()); // Not determined yet
        result.insert("new_state".into(), "combat".into());
        result.insert("reputation_change".into(), 0i64.into());
        result.insert("fame_change".into(), 0i64.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), true.into());
        result
    });

    engine.register_fn("create_choice_result", |state: String, narrative: String, choices: Array| -> Map {
        let mut result = Map::new();
        result.insert("success".into(), false.into()); // Not determined yet
        result.insert("new_state".into(), state.into());
        result.insert("reputation_change".into(), 0i64.into());
        result.insert("fame_change".into(), 0i64.into());
        result.insert("narrative".into(), narrative.into());
        result.insert("spawn_combat".into(), false.into());
        result.insert("choices".into(), Dynamic::from(choices));
        result
    });

    // Create a choice option
    engine.register_fn("create_choice", |id: String, text: String| -> Map {
        let mut choice = Map::new();
        choice.insert("id".into(), id.into());
        choice.insert("text".into(), text.into());
        choice.insert("requirements".into(), Dynamic::from(Array::new()));
        choice
    });

    engine.register_fn("create_choice_with_req", |id: String, text: String, req_type: String, req_value: i64| -> Map {
        let mut choice = Map::new();
        choice.insert("id".into(), id.into());
        choice.insert("text".into(), text.into());

        let mut req = Map::new();
        req.insert("type".into(), req_type.into());
        req.insert("value".into(), req_value.into());

        let mut reqs = Array::new();
        reqs.push(Dynamic::from(req));
        choice.insert("requirements".into(), Dynamic::from(reqs));

        choice
    });

    // Check if player meets requirement
    engine.register_fn("meets_requirement", |ctx: Map, req_type: String, req_value: i64| -> bool {
        match req_type.as_str() {
            "MinExperience" => {
                let exp = ctx.get("crew_experience")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0);
                exp >= req_value
            }
            "MinFame" => {
                let fame = ctx.get("player_fame")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0);
                fame >= req_value
            }
            "MinReputation" => {
                let rep = ctx.get("player_reputation")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0);
                rep >= req_value
            }
            "MinHull" => {
                let hull = ctx.get("ship_hull")
                    .and_then(|v| v.as_float().ok())
                    .unwrap_or(100.0);
                hull >= req_value as f64
            }
            "MinAmmo" => {
                let ammo = ctx.get("ship_ammo")
                    .and_then(|v| v.as_float().ok())
                    .unwrap_or(100.0);
                ammo >= req_value as f64
            }
            _ => true
        }
    });
}
