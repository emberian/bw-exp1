//! Mission API bindings for Rhai

use rhai::{Engine, Dynamic, Map, Array};

/// Register mission API functions.
pub fn register(engine: &mut Engine) {
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
