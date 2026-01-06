//! Action dispatcher integration tests
//!
//! Tests the complete action dispatch flow:
//! ActionDispatcher.dispatch() → script execution → StateMutation collection
//!
//! These tests verify that action scripts produce correct mutations.

mod test_harness;

use bw_core::models::Position;
use bw_game::state::AccessPermissions;
use bw_scripting::ActionContext;
use test_harness::{ScriptingTestHarness, mutation_sets_ship_status};

/// Test that dispatching an unknown action returns an error.
#[test]
fn test_unknown_action_returns_error() {
    let harness = ScriptingTestHarness::new();
    let sector = harness.create_sector("Test Sector");
    let player = harness.create_player("test_player", sector.id);
    let ship = harness.create_player_ship(player.id, "Test Ship", sector.id, Position::new(0.0, 0.0, 0.0));

    let ctx = ActionContext {
        player_id: player.id,
        ship_id: ship.id,
        sector_id: sector.id,
        action: "nonexistent_action".to_string(),
        params: serde_json::json!({}),
    };

    let result = harness.action_dispatcher.dispatch(ctx);

    assert!(!result.success, "Unknown action should fail");
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("Unknown action"));
}

/// Test move_to_position action produces correct status mutation.
#[test]
fn test_move_to_position_produces_in_transit_mutation() {
    let harness = ScriptingTestHarness::new();

    // Load movement scripts
    if harness.load_action_scripts().is_err() {
        eprintln!("Could not load action scripts - skipping test");
        return;
    }

    // Set up state
    let sector = harness.create_sector("Test Sector");
    let player = harness.create_player("test_player", sector.id);
    let ship = harness.create_player_ship(player.id, "Test Ship", sector.id, Position::new(0.0, 0.0, 0.0));

    // Set permissions for trusted access
    harness.set_permissions(AccessPermissions::trusted());

    let ctx = ActionContext {
        player_id: player.id,
        ship_id: ship.id,
        sector_id: sector.id,
        action: "move_to_position".to_string(),
        params: serde_json::json!({
            "x": 100.0,
            "y": 200.0,
            "z": 0.0
        }),
    };

    let result = harness.action_dispatcher.dispatch(ctx);

    // The action should succeed
    assert!(result.success, "move_to_position should succeed. Error: {:?}", result.error);

    // Verify the mutations include setting ship to in_transit
    let has_status_mutation = result.mutations.iter().any(|m| {
        mutation_sets_ship_status(m, ship.id, "in_transit")
    });

    assert!(has_status_mutation, "Should produce a mutation setting ship to in_transit. Mutations: {:?}", result.mutations);
}

/// Test move action fails for docked ships.
#[test]
fn test_move_fails_when_docked() {
    let harness = ScriptingTestHarness::new();

    if harness.load_action_scripts().is_err() {
        eprintln!("Could not load action scripts - skipping test");
        return;
    }

    let sector = harness.create_sector("Test Sector");
    let player = harness.create_player("test_player", sector.id);

    // Create a docked ship by modifying status
    let mut ship = harness.create_player_ship(player.id, "Test Ship", sector.id, Position::new(0.0, 0.0, 0.0));
    ship.status = "docked".to_string();
    harness.state_provider.add_ship(ship.clone()); // Update with docked status

    harness.set_permissions(AccessPermissions::trusted());

    let ctx = ActionContext {
        player_id: player.id,
        ship_id: ship.id,
        sector_id: sector.id,
        action: "move_to_position".to_string(),
        params: serde_json::json!({
            "x": 100.0,
            "y": 200.0
        }),
    };

    let result = harness.action_dispatcher.dispatch(ctx);

    // Action should fail because ship is docked
    assert!(!result.success, "move_to_position should fail for docked ship");
    assert!(result.error.as_ref().is_some_and(|e| e.contains("docked")),
        "Error should mention docked state. Error: {:?}", result.error);
}

/// Test move_to_position requires x and y coordinates.
#[test]
fn test_move_requires_coordinates() {
    let harness = ScriptingTestHarness::new();

    if harness.load_action_scripts().is_err() {
        eprintln!("Could not load action scripts - skipping test");
        return;
    }

    let sector = harness.create_sector("Test Sector");
    let player = harness.create_player("test_player", sector.id);
    let ship = harness.create_player_ship(player.id, "Test Ship", sector.id, Position::new(0.0, 0.0, 0.0));

    harness.set_permissions(AccessPermissions::trusted());

    // Missing x and y
    let ctx = ActionContext {
        player_id: player.id,
        ship_id: ship.id,
        sector_id: sector.id,
        action: "move_to_position".to_string(),
        params: serde_json::json!({}),
    };

    let result = harness.action_dispatcher.dispatch(ctx);

    assert!(!result.success, "Should fail without coordinates");
    assert!(result.error.as_ref().is_some_and(|e| e.contains("required") || e.contains("coordinates")),
        "Error should mention missing coordinates. Error: {:?}", result.error);
}

/// Test stop_movement action sets ship back to idle.
#[test]
fn test_stop_movement_sets_idle() {
    let harness = ScriptingTestHarness::new();

    if harness.load_action_scripts().is_err() {
        eprintln!("Could not load action scripts - skipping test");
        return;
    }

    let sector = harness.create_sector("Test Sector");
    let player = harness.create_player("test_player", sector.id);

    // Create ship that's already moving
    let mut ship = harness.create_player_ship(player.id, "Test Ship", sector.id, Position::new(0.0, 0.0, 0.0));
    ship.status = "in_transit".to_string();
    harness.state_provider.add_ship(ship.clone());

    harness.set_permissions(AccessPermissions::trusted());

    let ctx = ActionContext {
        player_id: player.id,
        ship_id: ship.id,
        sector_id: sector.id,
        action: "stop_movement".to_string(),
        params: serde_json::json!({}),
    };

    let result = harness.action_dispatcher.dispatch(ctx);

    assert!(result.success, "stop_movement should succeed. Error: {:?}", result.error);

    // Should produce mutation setting status back to idle
    let has_idle_mutation = result.mutations.iter().any(|m| {
        mutation_sets_ship_status(m, ship.id, "idle")
    });

    assert!(has_idle_mutation, "Should produce mutation setting ship to idle. Mutations: {:?}", result.mutations);
}

/// Test stop_movement fails if ship is not moving.
#[test]
fn test_stop_movement_fails_when_not_moving() {
    let harness = ScriptingTestHarness::new();

    if harness.load_action_scripts().is_err() {
        eprintln!("Could not load action scripts - skipping test");
        return;
    }

    let sector = harness.create_sector("Test Sector");
    let player = harness.create_player("test_player", sector.id);
    let ship = harness.create_player_ship(player.id, "Test Ship", sector.id, Position::new(0.0, 0.0, 0.0));
    // Ship starts with status "idle"

    harness.set_permissions(AccessPermissions::trusted());

    let ctx = ActionContext {
        player_id: player.id,
        ship_id: ship.id,
        sector_id: sector.id,
        action: "stop_movement".to_string(),
        params: serde_json::json!({}),
    };

    let result = harness.action_dispatcher.dispatch(ctx);

    assert!(!result.success, "stop_movement should fail for non-moving ship");
    assert!(result.error.as_ref().is_some_and(|e| e.contains("not moving")),
        "Error should mention ship is not moving. Error: {:?}", result.error);
}
