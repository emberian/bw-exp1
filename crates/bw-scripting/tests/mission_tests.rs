//! Integration tests for mission scripts

use uuid::Uuid;
use bw_scripting::{ScriptEngine, MissionContext, MissionRunner, MissionState};

/// Create a test mission context with default values.
fn create_test_context(script_path: &str) -> MissionContext {
    MissionContext {
        mission_id: Uuid::new_v4(),
        mission_type: "random".to_string(),
        script_path: script_path.to_string(),
        current_state: "start".to_string(),
        data: rhai::Map::new(),
        player_id: Uuid::new_v4(),
        player_reputation: 100,
        player_fame: 50,
        ship_id: Uuid::new_v4(),
        ship_hull: 100.0,
        ship_ammo: 100.0,
        ship_fuel: 100.0,
        crew_morale: 75.0,
        crew_experience: 100,
        sector_id: Uuid::new_v4(),
    }
}

/// Get engine with scripts loaded.
fn get_engine_with_scripts() -> ScriptEngine {
    let engine = ScriptEngine::new("../../scripts");
    // Try to load each script category and report which fails
    let scripts_to_load = [
        "missions/random/pirate_attack.rhai",
        "missions/random/distress_signal.rhai",
        "missions/random/smugglers.rhai",
        "missions/random/asteroid_threat.rhai",
        "missions/random/terrorist_plot.rhai",
        "combat/formulas.rhai",
        "ai/npc_behaviors.rhai",
    ];

    for script in scripts_to_load {
        if let Err(e) = engine.load_script(script) {
            panic!("Failed to load script '{}': {:?}", script, e);
        }
    }
    engine
}

// ============== Script Loading Tests ==============

#[test]
fn test_load_all_mission_scripts() {
    let engine = get_engine_with_scripts();
    let scripts = engine.loaded_scripts();

    // Check that we have the expected mission scripts
    assert!(scripts.iter().any(|s| s.contains("pirate_attack")), "pirate_attack script not found");
    assert!(scripts.iter().any(|s| s.contains("distress_signal")), "distress_signal script not found");
    assert!(scripts.iter().any(|s| s.contains("smugglers")), "smugglers script not found");
    assert!(scripts.iter().any(|s| s.contains("asteroid_threat")), "asteroid_threat script not found");
    assert!(scripts.iter().any(|s| s.contains("terrorist_plot")), "terrorist_plot script not found");
}

#[test]
fn test_load_combat_scripts() {
    let engine = get_engine_with_scripts();
    let scripts = engine.loaded_scripts();

    assert!(scripts.iter().any(|s| s.contains("formulas")), "combat/formulas script not found");
}

#[test]
fn test_load_ai_scripts() {
    let engine = get_engine_with_scripts();
    let scripts = engine.loaded_scripts();

    assert!(scripts.iter().any(|s| s.contains("npc_behaviors")), "ai/npc_behaviors script not found");
}

// ============== Pirate Attack Mission Tests ==============

#[test]
fn test_pirate_attack_start() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/pirate_attack.rhai");
    let result = runner.start_mission(&ctx);

    assert!(result.is_ok(), "Failed to start pirate_attack: {:?}", result);

    let outcome = result.unwrap();

    // Should have narrative text
    assert!(!outcome.narrative.is_empty(), "Narrative should not be empty");
    assert!(outcome.narrative.contains("distress") || outcome.narrative.contains("pirate"),
        "Narrative should mention distress or pirates: {}", outcome.narrative);

    // Should have choices
    assert!(!outcome.choices.is_empty(), "Should have choices");

    // Check that we have expected choices
    let choice_ids: Vec<&str> = outcome.choices.iter().map(|c| c.id.as_str()).collect();
    assert!(choice_ids.contains(&"attack"), "Should have attack choice");
    assert!(choice_ids.contains(&"abandon"), "Should have abandon choice");

    // Either deceive or negotiate might be available depending on requirements
    assert!(choice_ids.contains(&"deceive") || outcome.choices.iter().any(|c| c.id == "deceive"),
        "Should have deceive choice");
}

#[test]
fn test_pirate_attack_choices_with_low_stats() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let mut ctx = create_test_context("missions/random/pirate_attack.rhai");
    ctx.crew_experience = 10; // Low experience
    ctx.player_fame = 5; // Low fame

    let result = runner.start_mission(&ctx);
    assert!(result.is_ok());

    let outcome = result.unwrap();

    // With low stats, some choices should have requirements shown
    // The script adds locked versions of choices when requirements aren't met
    assert!(outcome.choices.len() >= 2, "Should have at least 2 choices");
}

#[test]
fn test_pirate_attack_attack_choice() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/pirate_attack.rhai");

    // Start mission first
    let start_result = runner.start_mission(&ctx);
    assert!(start_result.is_ok());

    // Make attack choice
    let result = runner.process_choice(&ctx, "attack");
    assert!(result.is_ok(), "Attack choice failed: {:?}", result);

    let outcome = result.unwrap();

    // Attack should spawn combat
    assert!(outcome.spawn_combat.is_some(), "Attack should spawn combat");
    assert!(!outcome.narrative.is_empty(), "Should have narrative for attack");
}

#[test]
fn test_pirate_attack_abandon_choice() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/pirate_attack.rhai");

    // Start mission
    runner.start_mission(&ctx).unwrap();

    // Make abandon choice
    let result = runner.process_choice(&ctx, "abandon");
    assert!(result.is_ok(), "Abandon choice failed: {:?}", result);

    let outcome = result.unwrap();

    // Abandon should complete with partial success
    assert!(outcome.is_complete, "Abandon should complete mission");
    assert!(!outcome.narrative.is_empty(), "Should have narrative");
    assert!(outcome.narrative.contains("cargo") || outcome.narrative.contains("saved"),
        "Narrative should mention cargo or saving: {}", outcome.narrative);

    // Should have some reputation gain (saved lives)
    assert!(outcome.resource_changes.reputation >= 0, "Should have non-negative rep");
}

// ============== Distress Signal Mission Tests ==============

#[test]
fn test_distress_signal_start() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/distress_signal.rhai");
    let result = runner.start_mission(&ctx);

    assert!(result.is_ok(), "Failed to start distress_signal: {:?}", result);

    let outcome = result.unwrap();
    assert!(!outcome.narrative.is_empty());
    assert!(!outcome.choices.is_empty(), "Should have choices");
}

// ============== Smugglers Mission Tests ==============

#[test]
fn test_smugglers_start() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/smugglers.rhai");
    let result = runner.start_mission(&ctx);

    assert!(result.is_ok(), "Failed to start smugglers: {:?}", result);

    let outcome = result.unwrap();
    assert!(!outcome.narrative.is_empty());
    assert!(!outcome.choices.is_empty(), "Should have choices");
}

// ============== Asteroid Threat Mission Tests ==============

#[test]
fn test_asteroid_threat_start() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/asteroid_threat.rhai");
    let result = runner.start_mission(&ctx);

    assert!(result.is_ok(), "Failed to start asteroid_threat: {:?}", result);

    let outcome = result.unwrap();
    assert!(!outcome.narrative.is_empty());
    assert!(!outcome.choices.is_empty(), "Should have choices");
}

// ============== Terrorist Plot Mission Tests ==============

#[test]
fn test_terrorist_plot_start() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/terrorist_plot.rhai");
    let result = runner.start_mission(&ctx);

    assert!(result.is_ok(), "Failed to start terrorist_plot: {:?}", result);

    let outcome = result.unwrap();
    assert!(!outcome.narrative.is_empty());
    assert!(!outcome.choices.is_empty(), "Should have choices");
}

// ============== Combat Resolution Tests ==============

#[test]
fn test_pirate_attack_combat_won() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/pirate_attack.rhai");

    // Start and attack
    runner.start_mission(&ctx).unwrap();
    runner.process_choice(&ctx, "attack").unwrap();

    // Resolve combat - player won
    let result = runner.process_combat_result(&ctx, true, false);
    assert!(result.is_ok(), "Combat resolution failed: {:?}", result);

    let outcome = result.unwrap();
    assert!(outcome.is_complete, "Should complete after combat");
    assert!(outcome.success, "Should succeed when winning");
    assert!(outcome.resource_changes.reputation > 0, "Should gain reputation");
}

#[test]
fn test_pirate_attack_combat_lost() {
    let engine = get_engine_with_scripts();
    let runner = MissionRunner::new(&engine);

    let ctx = create_test_context("missions/random/pirate_attack.rhai");

    // Start and attack
    runner.start_mission(&ctx).unwrap();
    runner.process_choice(&ctx, "attack").unwrap();

    // Resolve combat - player lost
    let result = runner.process_combat_result(&ctx, false, false);
    assert!(result.is_ok(), "Combat resolution failed: {:?}", result);

    let outcome = result.unwrap();
    assert!(outcome.is_complete, "Should complete after combat");
    assert!(!outcome.success, "Should fail when losing");
    assert!(outcome.resource_changes.reputation < 0, "Should lose reputation");
}

// ============== Choice Requirement Tests ==============

#[test]
fn test_choice_requirements_parsing() {
    use bw_scripting::mission_runner::ChoiceRequirement;

    let ctx = create_test_context("test");

    // MinExperience
    let req = ChoiceRequirement::MinExperience(50);
    assert!(req.is_met(&ctx), "With 100 exp, should meet 50 req");

    let mut low_exp_ctx = ctx.clone();
    low_exp_ctx.crew_experience = 30;
    assert!(!req.is_met(&low_exp_ctx), "With 30 exp, should not meet 50 req");

    // MinFame
    let req = ChoiceRequirement::MinFame(30);
    assert!(req.is_met(&ctx), "With 50 fame, should meet 30 req");

    // MinReputation
    let req = ChoiceRequirement::MinReputation(80);
    assert!(req.is_met(&ctx), "With 100 rep, should meet 80 req");
}

// ============== Mission State Tests ==============

#[test]
fn test_mission_state_parsing() {
    assert_eq!(MissionState::from_string("started"), MissionState::Started);
    assert_eq!(MissionState::from_string("choice"), MissionState::AwaitingChoice);
    assert_eq!(MissionState::from_string("awaiting_choice"), MissionState::AwaitingChoice);
    assert_eq!(MissionState::from_string("combat"), MissionState::InCombat);
    assert_eq!(MissionState::from_string("in_combat"), MissionState::InCombat);
    assert_eq!(MissionState::from_string("success"), MissionState::CompletedSuccess);
    assert_eq!(MissionState::from_string("failure"), MissionState::CompletedFailure);
    assert_eq!(MissionState::from_string("custom_state"), MissionState::Custom("custom_state".to_string()));
}

// ============== Resource Changes Tests ==============

#[test]
fn test_resource_changes_critical_check() {
    use bw_scripting::mission_runner::ResourceChanges;

    let ctx = create_test_context("test");

    // Normal changes - no critical
    let changes = ResourceChanges {
        reputation: -10,
        ..Default::default()
    };
    assert!(changes.would_cause_critical(&ctx).is_none());

    // Critical - rep would drop to 0
    let changes = ResourceChanges {
        reputation: -100,
        ..Default::default()
    };
    assert!(changes.would_cause_critical(&ctx).is_some());

    // Critical - ammo would deplete
    let changes = ResourceChanges {
        ammunition: -100.0,
        ..Default::default()
    };
    assert!(changes.would_cause_critical(&ctx).is_some());
}
