// Mission script helpers
//
// Import with: import "mission" as mission;
//
// Provides ergonomic builders for mission outcomes.

/// Create a success outcome.
fn success(reputation, fame, narrative) {
    #{
        success: true,
        new_state: "complete",
        reputation_change: reputation,
        fame_change: fame,
        narrative: narrative,
        spawn_combat: false
    }
}

/// Create a failure outcome.
fn failure(reputation_loss, narrative) {
    #{
        success: false,
        new_state: "complete",
        reputation_change: -reputation_loss,
        fame_change: 0,
        narrative: narrative,
        spawn_combat: false
    }
}

/// Create a choice outcome (presents choices to player).
fn choice_outcome(new_state, narrative, choices) {
    #{
        success: true,
        new_state: new_state,
        narrative: narrative,
        choices: choices,
        spawn_combat: false
    }
}

/// Create a combat outcome (spawns combat).
fn combat_outcome(narrative) {
    #{
        success: true,
        new_state: "combat",
        narrative: narrative,
        spawn_combat: true
    }
}

/// Create a simple choice.
fn choice(id, text) {
    #{
        id: id,
        text: text,
        requirements: []
    }
}

/// Create a choice with a requirement.
fn choice_with_req(id, text, req_type, req_value) {
    let req = #{};
    req[req_type] = req_value;
    #{
        id: id,
        text: text,
        requirements: [req]
    }
}

/// Check if context meets a requirement.
fn meets_req(ctx, req_type, value) {
    if req_type == "MinExperience" {
        ctx["crew_experience"] >= value
    } else if req_type == "MinFame" {
        ctx["player_fame"] >= value
    } else if req_type == "MinReputation" {
        ctx["player_reputation"] >= value
    } else if req_type == "MinHull" {
        ctx["ship_hull"] >= value
    } else if req_type == "MinAmmo" {
        ctx["ship_ammo"] >= value
    } else if req_type == "MinFuel" {
        ctx["ship_fuel"] >= value
    } else {
        true
    }
}

/// Calculate success chance based on experience.
fn exp_success_chance(base, experience) {
    // Each 10 experience adds 1% success chance, capped at 90%
    let bonus = (experience / 10) * 0.01;
    let total = base + bonus;
    if total > 0.9 { 0.9 } else { total }
}

/// Get mission data from context.
fn get_data(ctx) {
    ctx["data"]
}

/// Get crew experience from context.
fn get_experience(ctx) {
    ctx["crew_experience"]
}

/// Get player fame from context.
fn get_fame(ctx) {
    ctx["player_fame"]
}

/// Get player reputation from context.
fn get_reputation(ctx) {
    ctx["player_reputation"]
}
