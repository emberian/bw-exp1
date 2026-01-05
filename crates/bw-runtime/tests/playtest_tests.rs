//! Integration tests for the GM Playtest system
//!
//! Focused test suite covering core functionality:
//! - PlaytestManager lifecycle
//! - State forking and isolation
//! - Player participation
//! - Promotion workflow
//! - End-to-end scenarios

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use uuid::Uuid;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use bw_core::models::{Player, Position, Sector, Ship, ShipClass, DangerLevel};
use bw_scripting::{
    BehaviorManager, CoroutineScheduler, EventRegistry, EventDispatcher,
    ActionRegistry, ActionDispatcher,
    debug::DebugController,
};
use bw_runtime::playtest::{
    ForkConfig, PlaytestBuilder, PlaytestError, PlaytestInstance, PlaytestManager,
};
use bw_runtime::GameState;
use bw_runtime::scripting::ScriptLogBuffer;

// =============================================================================
// Test Setup
// =============================================================================

/// Populated test state with all IDs accessible.
struct TestState {
    state: GameState,
    sector_id: Uuid,
    player_id: Uuid,
    player_ship_id: Uuid,
    npc_ship_id: Uuid,
}

/// Create a test ship.
fn create_test_ship(name: &str, owner_id: Uuid, sector_id: Uuid, faction_id: Uuid, is_player_ship: bool) -> Ship {
    if is_player_ship {
        Ship::new_player_ship(
            name.to_string(),
            owner_id,
            ShipClass::PatrolCorvette,
            sector_id,
            faction_id,
        )
    } else {
        Ship::new_npc_ship(
            name.to_string(),
            ShipClass::PirateRaider,
            sector_id,
            Position::new(100.0, 0.0, 0.0),
            None,
        )
    }
}

/// Create a test sector.
fn create_test_sector(name: &str) -> Sector {
    Sector::new(name.to_string(), DangerLevel::Safe)
}

/// Set up a minimal game state.
async fn setup_test_state() -> GameState {
    let db = bw_runtime::Database::new_in_memory().await.unwrap();
    let persist = db.spawn_persistence();
    let db = Arc::new(db);
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let (broadcaster, _) = broadcast::channel(1000);

    let event_registry = Arc::new(EventRegistry::new());
    let action_registry = Arc::new(ActionRegistry::new());
    let debug_controller = Arc::new(DebugController::new());
    let mut behavior_manager = BehaviorManager::new(scripts.clone());
    behavior_manager.set_debug_controller(debug_controller.clone());
    let coroutine_scheduler = CoroutineScheduler::new(scripts.clone());
    let event_dispatcher = EventDispatcher::new(event_registry.clone(), scripts.clone());
    let action_dispatcher = ActionDispatcher::new(action_registry.clone(), scripts.clone());

    let mut playtest_manager = PlaytestManager::new(10);
    playtest_manager.set_debug_controller(debug_controller.clone());

    GameState {
        db,
        persist,
        scripts,
        sectors: DashMap::new(),
        player_data: bw_runtime::persistence::TrackedDashMap::new(),
        players: DashMap::new(),
        ships: bw_runtime::persistence::TrackedDashMap::new(),
        factions: DashMap::new(),
        faction_tags: DashMap::new(),
        squadrons: bw_runtime::persistence::TrackedDashMap::new(),
        pending_squadron_invites: DashMap::new(),
        pending_alliances: DashMap::new(),
        contested_sectors: DashMap::new(),
        broadcaster,
        tick: AtomicU64::new(100),
        behavior_manager: RwLock::new(behavior_manager),
        coroutine_scheduler: RwLock::new(coroutine_scheduler),
        event_registry,
        event_dispatcher: RwLock::new(event_dispatcher),
        action_registry,
        action_dispatcher: RwLock::new(action_dispatcher),
        state_accessor: RwLock::new(None),
        script_logs: RwLock::new(ScriptLogBuffer::new(100)),
        metrics: bw_runtime::simulation::metrics::MetricsStore::new(),
        playtest_manager,
        debug_controller,
    }
}

/// Set up a populated state with sectors, ships, and players.
async fn setup_populated_state() -> TestState {
    let state = setup_test_state().await;
    let faction_id = Uuid::new_v4();

    // Create sector
    let sector = create_test_sector("Test Sector Alpha");
    let sector_id = sector.id;
    state.sectors.insert(sector_id, bw_runtime::state::SectorInstance::new(sector));

    // Create player
    let mut player = Player::new(
        "TestGM".to_string(),
        Uuid::new_v4(),
        Uuid::nil(),
        sector_id,
    );
    let player_id = player.id;
    player.resources.reputation = 100;
    player.credits = 10000;

    // Create player ship
    let player_ship = create_test_ship("Player Ship", player_id, sector_id, faction_id, true);
    let player_ship_id = player_ship.id;
    state.ships.insert(player_ship_id, player_ship);

    // Update player with active_ship_id
    player.active_ship_id = player_ship_id;
    state.player_data.insert(player_id, player);

    // Add player session
    let (tx, _rx) = tokio::sync::mpsc::channel(100);
    state.players.insert(player_id, bw_runtime::state::PlayerSession {
        player_id,
        ship_id: player_ship_id,
        sector_id,
        connection: Some(tx),
        connection_id: None,
        playtest_id: None,
    });

    // Add ship to sector
    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.ship_ids.insert(player_ship_id, ());
    }

    // Create NPC ship
    let npc_ship = create_test_ship("Pirate Scum", Uuid::nil(), sector_id, Uuid::nil(), false);
    let npc_ship_id = npc_ship.id;
    state.ships.insert(npc_ship_id, npc_ship);

    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.ship_ids.insert(npc_ship_id, ());
    }

    TestState {
        state,
        sector_id,
        player_id,
        player_ship_id,
        npc_ship_id,
    }
}

/// Helper to create and register a playtest.
fn create_playtest(
    state: &GameState,
    owner_id: Uuid,
    name: &str,
    config: ForkConfig,
) -> (Uuid, Arc<PlaytestInstance>) {
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(owner_id, name.to_string(), state.get_tick(), config.clone());
    let playtest_id = builder.id();
    let instance = builder.build(factions, faction_tags, state.scripts.clone(), state.debug_controller.clone());

    state.fork_to_playtest(&instance, &config);
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    let instance = state.playtest_manager.register(instance).unwrap();
    (playtest_id, instance)
}

// =============================================================================
// Manager Tests
// =============================================================================

#[tokio::test]
async fn test_manager_instance_limit() {
    let manager = PlaytestManager::new(2);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let debug_controller = Arc::new(DebugController::new());

    // Create two playtests - should succeed
    for i in 0..2 {
        let builder = PlaytestBuilder::new(Uuid::new_v4(), format!("Test {}", i), 100, ForkConfig::default());
        let instance = builder.build(factions.clone(), faction_tags.clone(), scripts.clone(), debug_controller.clone());
        assert!(manager.register(instance).is_ok());
    }

    // Third should fail
    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test 3".into(), 100, ForkConfig::default());
    let instance = builder.build(factions.clone(), faction_tags.clone(), scripts.clone(), debug_controller.clone());
    assert!(matches!(manager.register(instance), Err(PlaytestError::LimitReached)));
}

#[tokio::test]
async fn test_player_cannot_join_multiple_playtests() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let debug_controller = Arc::new(DebugController::new());

    let player_id = Uuid::new_v4();

    // Create and register first playtest
    let builder1 = PlaytestBuilder::new(Uuid::new_v4(), "Test 1".into(), 100, ForkConfig::default());
    let playtest1_id = builder1.id();
    let instance1 = builder1.build(factions.clone(), faction_tags.clone(), scripts.clone(), debug_controller.clone());
    instance1.add_participant(Uuid::new_v4(), true, Uuid::new_v4(), Uuid::new_v4()).unwrap();
    manager.register(instance1).unwrap();

    // Create second playtest
    let builder2 = PlaytestBuilder::new(Uuid::new_v4(), "Test 2".into(), 100, ForkConfig::default());
    let playtest2_id = builder2.id();
    let instance2 = builder2.build(factions.clone(), faction_tags.clone(), scripts.clone(), debug_controller.clone());
    instance2.add_participant(Uuid::new_v4(), true, Uuid::new_v4(), Uuid::new_v4()).unwrap();
    manager.register(instance2).unwrap();

    // Join first playtest
    manager.join_playtest(playtest1_id, player_id, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    // Try to join second - should fail
    let result = manager.join_playtest(playtest2_id, player_id, Uuid::new_v4(), Uuid::new_v4());
    assert!(matches!(result, Err(PlaytestError::AlreadyInPlaytest)));
}

#[tokio::test]
async fn test_destroy_cleans_up_all_participants() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let debug_controller = Arc::new(DebugController::new());

    let owner_id = Uuid::new_v4();
    let player_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

    let builder = PlaytestBuilder::new(owner_id, "Test".into(), 100, ForkConfig::default());
    let playtest_id = builder.id();
    let instance = builder.build(factions.clone(), faction_tags.clone(), scripts.clone(), debug_controller.clone());
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();
    manager.register(instance).unwrap();

    // Add multiple players
    for pid in &player_ids {
        manager.join_playtest(playtest_id, *pid, Uuid::new_v4(), Uuid::new_v4()).unwrap();
    }

    // Verify all are tracked
    assert!(manager.is_player_in_playtest(owner_id));
    for pid in &player_ids {
        assert!(manager.is_player_in_playtest(*pid));
    }

    // Destroy
    manager.destroy(playtest_id).unwrap();

    // Verify all cleaned up
    assert!(!manager.is_player_in_playtest(owner_id));
    for pid in &player_ids {
        assert!(!manager.is_player_in_playtest(*pid));
    }
    assert!(manager.get(playtest_id).is_none());
}

// =============================================================================
// State Forking Tests
// =============================================================================

#[tokio::test]
async fn test_fork_copies_ships_and_sectors() {
    let ts = setup_populated_state().await;

    let config = ForkConfig::single_sector(ts.sector_id);
    let (_, instance) = create_playtest(&ts.state, ts.player_id, "Test Fork", config);

    // Ships forked
    assert!(instance.ships.contains_key(&ts.player_ship_id));
    assert!(instance.ships.contains_key(&ts.npc_ship_id));

    // Sector forked
    assert!(instance.sectors.contains_key(&ts.sector_id));
    let playtest_sector = instance.sectors.get(&ts.sector_id).unwrap();
    assert_eq!(playtest_sector.sector.name, "Test Sector Alpha");
}

#[tokio::test]
async fn test_fork_only_includes_specified_sectors() {
    let state = setup_test_state().await;

    // Create two sectors with ships
    let sector1 = create_test_sector("Sector 1");
    let sector1_id = sector1.id;
    state.sectors.insert(sector1_id, bw_runtime::state::SectorInstance::new(sector1));

    let sector2 = create_test_sector("Sector 2");
    let sector2_id = sector2.id;
    state.sectors.insert(sector2_id, bw_runtime::state::SectorInstance::new(sector2));

    let ship1 = create_test_ship("Ship 1", Uuid::nil(), sector1_id, Uuid::nil(), false);
    let ship1_id = ship1.id;
    state.ships.insert(ship1_id, ship1);

    let ship2 = create_test_ship("Ship 2", Uuid::nil(), sector2_id, Uuid::nil(), false);
    let ship2_id = ship2.id;
    state.ships.insert(ship2_id, ship2);

    // Fork only sector 1
    let config = ForkConfig::single_sector(sector1_id);
    let (_, instance) = create_playtest(&state, Uuid::new_v4(), "Test", config);

    // Only sector1 and ship1 should be present
    assert!(instance.sectors.contains_key(&sector1_id));
    assert!(!instance.sectors.contains_key(&sector2_id));
    assert!(instance.ships.contains_key(&ship1_id));
    assert!(!instance.ships.contains_key(&ship2_id));
}

#[tokio::test]
async fn test_playtest_changes_do_not_affect_live() {
    let ts = setup_populated_state().await;

    let original_hull = ts.state.ships.get(&ts.npc_ship_id).unwrap().hull_integrity;

    let config = ForkConfig::single_sector(ts.sector_id);
    let (_, instance) = create_playtest(&ts.state, ts.player_id, "Test", config);

    // Modify ship in playtest
    if let Some(mut ship) = instance.ships.get_mut(&ts.npc_ship_id) {
        ship.hull_integrity = 10.0;
        ship.shield_strength = 5.0;
    }

    // Live unchanged
    let live_ship = ts.state.ships.get(&ts.npc_ship_id).unwrap();
    assert!((live_ship.hull_integrity - original_hull).abs() < 0.01);

    // Playtest changed
    let playtest_ship = instance.ships.get(&ts.npc_ship_id).unwrap();
    assert!((playtest_ship.hull_integrity - 10.0).abs() < 0.01);
}

#[tokio::test]
async fn test_concurrent_playtests_are_isolated() {
    let state = setup_test_state().await;

    // Create shared sector and ship
    let sector = create_test_sector("Shared");
    let sector_id = sector.id;
    state.sectors.insert(sector_id, bw_runtime::state::SectorInstance::new(sector));

    let npc = create_test_ship("NPC", Uuid::nil(), sector_id, Uuid::nil(), false);
    let npc_id = npc.id;
    state.ships.insert(npc_id, npc);

    // Create two playtests
    let config = ForkConfig::single_sector(sector_id);
    let (_, instance1) = create_playtest(&state, Uuid::new_v4(), "Playtest 1", config.clone());
    let (_, instance2) = create_playtest(&state, Uuid::new_v4(), "Playtest 2", config);

    // Modify NPC differently in each
    instance1.ships.get_mut(&npc_id).unwrap().hull_integrity = 50.0;
    instance2.ships.get_mut(&npc_id).unwrap().hull_integrity = 75.0;

    // Each has independent state
    assert!((instance1.ships.get(&npc_id).unwrap().hull_integrity - 50.0).abs() < 0.01);
    assert!((instance2.ships.get(&npc_id).unwrap().hull_integrity - 75.0).abs() < 0.01);
    assert!((state.ships.get(&npc_id).unwrap().hull_integrity - 100.0).abs() < 0.01);
}

// =============================================================================
// Instance Controls Tests
// =============================================================================

#[test]
fn test_playtest_tick_and_time_controls() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let debug_controller = Arc::new(DebugController::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 1000, ForkConfig::default());
    let instance = builder.build(factions, faction_tags, scripts, debug_controller);

    // Starts at fork tick
    assert_eq!(instance.get_tick(), 1000);

    // Can increment
    assert_eq!(instance.increment_tick(), 1001);
    assert_eq!(instance.get_tick(), 1001);

    // Starts paused
    assert!(instance.is_paused());
    instance.set_paused(false);
    assert!(!instance.is_paused());

    // Time scale defaults to 1.0x
    assert!((instance.get_time_scale() - 1.0).abs() < 0.01);

    // Time scale clamped to 0.1x - 10x
    instance.set_time_scale(100.0);
    assert!((instance.get_time_scale() - 10.0).abs() < 0.01);
    instance.set_time_scale(0.001);
    assert!((instance.get_time_scale() - 0.1).abs() < 0.01);
}

#[test]
fn test_participant_limit() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let debug_controller = Arc::new(DebugController::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags, scripts, debug_controller);

    // Fill to max
    for _ in 0..PlaytestInstance::MAX_PARTICIPANTS {
        instance.add_participant(Uuid::new_v4(), false, Uuid::new_v4(), Uuid::new_v4()).unwrap();
    }

    // One more fails
    let result = instance.add_participant(Uuid::new_v4(), false, Uuid::new_v4(), Uuid::new_v4());
    assert!(matches!(result, Err(PlaytestError::ParticipantLimitReached)));
}

// =============================================================================
// Promotion Tests
// =============================================================================

#[tokio::test]
async fn test_promotion_copies_ship_stats_to_live() {
    let ts = setup_populated_state().await;

    let config = ForkConfig::single_sector(ts.sector_id);
    let factions = ts.state.get_factions_arc();
    let faction_tags = ts.state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(ts.player_id, "Promo Test".into(), ts.state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags, ts.state.scripts.clone(), ts.state.debug_controller.clone());
    ts.state.fork_to_playtest(&instance, &config);

    // Modify ship in playtest
    if let Some(mut ship) = instance.ships.get_mut(&ts.npc_ship_id) {
        ship.hull_integrity = 50.0;
        ship.shield_strength = 25.0;
    }

    // Manually promote (simulating what handler does)
    if let Some(playtest_ship) = instance.ships.get(&ts.npc_ship_id)
        && let Some(mut live_ship) = ts.state.ships.get_mut(&ts.npc_ship_id) {
            live_ship.hull_integrity = playtest_ship.hull_integrity;
            live_ship.shield_strength = playtest_ship.shield_strength;
        }

    // Live state updated
    let live_ship = ts.state.ships.get(&ts.npc_ship_id).unwrap();
    assert!((live_ship.hull_integrity - 50.0).abs() < 0.01);
    assert!((live_ship.shield_strength - 25.0).abs() < 0.01);
}

#[tokio::test]
async fn test_promotion_spawns_created_entities() {
    let ts = setup_populated_state().await;

    let config = ForkConfig::single_sector(ts.sector_id);
    let factions = ts.state.get_factions_arc();
    let faction_tags = ts.state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(ts.player_id, "Spawn Test".into(), ts.state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags, ts.state.scripts.clone(), ts.state.debug_controller.clone());
    ts.state.fork_to_playtest(&instance, &config);

    // Create new ship in playtest
    let new_ship = create_test_ship("New Enemy", Uuid::nil(), ts.sector_id, Uuid::nil(), false);
    let new_ship_id = new_ship.id;
    instance.ships.insert(new_ship_id, new_ship.clone());
    instance.created_ship_ids.insert(new_ship_id, ());

    // Verify doesn't exist in live yet
    assert!(!ts.state.ships.contains_key(&new_ship_id));

    // Promote created entities
    for entry in instance.created_ship_ids.iter() {
        let ship_id = *entry.key();
        if let Some(playtest_ship) = instance.ships.get(&ship_id) {
            // Verify sector exists (safety check from fix)
            if ts.state.sectors.contains_key(&playtest_ship.sector_id) {
                let ship_clone = (*playtest_ship).clone();
                ts.state.ships.insert(ship_id, ship_clone.clone());
                if let Some(sector) = ts.state.sectors.get(&ship_clone.sector_id) {
                    sector.ship_ids.insert(ship_id, ());
                }
            }
        }
    }

    // Now exists in live
    assert!(ts.state.ships.contains_key(&new_ship_id));
}

#[tokio::test]
async fn test_promotion_applies_deletions() {
    let ts = setup_populated_state().await;

    let config = ForkConfig::single_sector(ts.sector_id);
    let factions = ts.state.get_factions_arc();
    let faction_tags = ts.state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(ts.player_id, "Delete Test".into(), ts.state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags, ts.state.scripts.clone(), ts.state.debug_controller.clone());
    ts.state.fork_to_playtest(&instance, &config);

    // Delete NPC in playtest
    instance.ships.remove(&ts.npc_ship_id);
    instance.deleted_ship_ids.insert(ts.npc_ship_id, ());

    // Verify still exists in live
    assert!(ts.state.ships.contains_key(&ts.npc_ship_id));

    // Promote deletions
    for entry in instance.deleted_ship_ids.iter() {
        let ship_id = *entry.key();
        if let Some((_, ship)) = ts.state.ships.remove(&ship_id)
            && let Some(sector) = ts.state.sectors.get(&ship.sector_id) {
                sector.ship_ids.remove(&ship_id);
            }
    }

    // Now deleted from live
    assert!(!ts.state.ships.contains_key(&ts.npc_ship_id));
}

#[tokio::test]
async fn test_promotion_skips_ships_that_changed_sector() {
    let ts = setup_populated_state().await;

    // Create second sector
    let sector2 = create_test_sector("Sector 2");
    let sector2_id = sector2.id;
    ts.state.sectors.insert(sector2_id, bw_runtime::state::SectorInstance::new(sector2));

    // Fork original sector
    let config = ForkConfig::single_sector(ts.sector_id);
    let factions = ts.state.get_factions_arc();
    let faction_tags = ts.state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(ts.player_id, "Sector Change Test".into(), ts.state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags, ts.state.scripts.clone(), ts.state.debug_controller.clone());
    ts.state.fork_to_playtest(&instance, &config);

    // Record original position
    let original_position = ts.state.ships.get(&ts.npc_ship_id).unwrap().position;

    // Move ship to different sector in playtest
    if let Some(mut ship) = instance.ships.get_mut(&ts.npc_ship_id) {
        ship.sector_id = sector2_id;
        ship.position = Position::new(999.0, 999.0, 999.0);
        ship.hull_integrity = 50.0; // This should still promote
    }

    // Simulate promotion logic with sector change check
    if let Some(playtest_ship) = instance.ships.get(&ts.npc_ship_id)
        && let Some(mut live_ship) = ts.state.ships.get_mut(&ts.npc_ship_id) {
            let sector_changed = playtest_ship.sector_id != live_ship.sector_id;

            // Always promote stats
            live_ship.hull_integrity = playtest_ship.hull_integrity;

            // Only promote position if sector unchanged
            if !sector_changed {
                live_ship.position = playtest_ship.position;
            }
        }

    let live_ship = ts.state.ships.get(&ts.npc_ship_id).unwrap();
    // Hull promoted
    assert!((live_ship.hull_integrity - 50.0).abs() < 0.01);
    // Position NOT promoted (sector changed)
    assert!((live_ship.position.x - original_position.x).abs() < 0.01);
}

#[tokio::test]
async fn test_promotion_skips_nonexistent_sector() {
    let ts = setup_populated_state().await;

    let config = ForkConfig::single_sector(ts.sector_id);
    let factions = ts.state.get_factions_arc();
    let faction_tags = ts.state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(ts.player_id, "Missing Sector Test".into(), ts.state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags, ts.state.scripts.clone(), ts.state.debug_controller.clone());
    ts.state.fork_to_playtest(&instance, &config);

    // Create ship in playtest targeting nonexistent sector
    let fake_sector_id = Uuid::new_v4();
    let new_ship = Ship::new_npc_ship(
        "Orphan".to_string(),
        ShipClass::PirateRaider,
        fake_sector_id, // This sector doesn't exist in live
        Position::new(0.0, 0.0, 0.0),
        None,
    );
    let new_ship_id = new_ship.id;
    instance.ships.insert(new_ship_id, new_ship);
    instance.created_ship_ids.insert(new_ship_id, ());

    // Attempt to promote - should skip due to missing sector
    let mut spawned = false;
    for entry in instance.created_ship_ids.iter() {
        let ship_id = *entry.key();
        if let Some(playtest_ship) = instance.ships.get(&ship_id)
            && ts.state.sectors.contains_key(&playtest_ship.sector_id) {
                ts.state.ships.insert(ship_id, (*playtest_ship).clone());
                spawned = true;
            }
    }

    // Ship should NOT have been spawned
    assert!(!spawned);
    assert!(!ts.state.ships.contains_key(&new_ship_id));
}

// =============================================================================
// End-to-End Test
// =============================================================================

#[tokio::test]
async fn test_full_playtest_lifecycle() {
    let ts = setup_populated_state().await;

    // === 1. Create playtest ===
    let config = ForkConfig::single_sector(ts.sector_id);
    let (playtest_id, instance) = create_playtest(&ts.state, ts.player_id, "E2E Test", config);

    // Verify forked
    assert!(instance.ships.contains_key(&ts.player_ship_id));
    assert!(instance.sectors.contains_key(&ts.sector_id));

    // Update session (what handler does)
    if let Some(mut session) = ts.state.players.get_mut(&ts.player_id) {
        session.playtest_id = Some(playtest_id);
    }

    // === 2. Verify routing ===
    assert!(ts.state.playtest_manager.is_player_in_playtest(ts.player_id));

    // === 3. Modify playtest state ===
    instance.ships.get_mut(&ts.npc_ship_id).unwrap().hull_integrity = 25.0;

    // Live unchanged
    assert!((ts.state.ships.get(&ts.npc_ship_id).unwrap().hull_integrity - 100.0).abs() < 0.01);

    // === 4. Add another player ===
    let player2_id = Uuid::new_v4();
    let mut player2 = Player::new("Player2".to_string(), Uuid::new_v4(), Uuid::nil(), ts.sector_id);
    player2.active_ship_id = Uuid::new_v4();
    ts.state.player_data.insert(player2_id, player2);

    let (tx2, _rx2) = tokio::sync::mpsc::channel(100);
    ts.state.players.insert(player2_id, bw_runtime::state::PlayerSession {
        player_id: player2_id,
        ship_id: Uuid::new_v4(),
        sector_id: ts.sector_id,
        connection: Some(tx2),
        connection_id: None,
        playtest_id: None,
    });

    ts.state.playtest_manager.join_playtest(playtest_id, player2_id, Uuid::new_v4(), ts.sector_id).unwrap();
    assert_eq!(instance.participant_count(), 2);

    // === 5. Player leaves ===
    ts.state.playtest_manager.leave_playtest(player2_id).unwrap();
    assert_eq!(instance.participant_count(), 1);
    assert!(!ts.state.playtest_manager.is_player_in_playtest(player2_id));

    // === 6. Destroy ===
    ts.state.playtest_manager.destroy(playtest_id).unwrap();
    assert!(!ts.state.playtest_manager.is_player_in_playtest(ts.player_id));
    assert!(ts.state.playtest_manager.get(playtest_id).is_none());
}
