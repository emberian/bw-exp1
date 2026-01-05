//! Integration tests for the GM Playtest system

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use uuid::Uuid;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use bw_core::models::{Player, Position, Sector, Ship, ShipClass, DangerLevel};
use bw_scripting::{BehaviorManager, CoroutineScheduler, EventRegistry, EventDispatcher};
use bw_server::playtest::{
    ForkConfig, PlaytestBuilder, PlaytestError, PlaytestInstance, PlaytestManager,
};
use bw_server::GameState;
use bw_server::scripting::ScriptLogBuffer;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a test player.
fn create_test_player(username: &str, sector_id: Uuid, ship_id: Uuid) -> Player {
    let mut player = Player::new(
        username.to_string(),
        Uuid::new_v4(),
        ship_id,
        sector_id,
    );
    player.resources.reputation = 100;
    player.credits = 10000;
    player
}

/// Create a test ship.
fn create_test_ship(name: &str, owner_id: Uuid, sector_id: Uuid, faction_id: Uuid, is_player_ship: bool) -> Ship {
    if is_player_ship {
        Ship::new_player_ship(
            name.to_string(),
            owner_id,
            ShipClass::PatrolCorvette, // Default class for player ships
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
    Sector::new(
        name.to_string(),
        DangerLevel::Safe,
    )
}

/// Set up a minimal game state for testing playtests.
async fn setup_test_state() -> GameState {
    let db = bw_server::Database::new_in_memory().await.unwrap();
    let persist = db.spawn_persistence();
    let db = Arc::new(db);
    let scripts = Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let (broadcaster, _) = broadcast::channel(1000);

    let event_registry = Arc::new(EventRegistry::new());
    let behavior_manager = BehaviorManager::new(scripts.clone());
    let coroutine_scheduler = CoroutineScheduler::new(scripts.clone());
    let event_dispatcher = EventDispatcher::new(event_registry.clone(), scripts.clone());

    GameState {
        db,
        persist,
        scripts,
        sectors: DashMap::new(),
        player_data: bw_server::persistence::TrackedDashMap::new(),
        players: DashMap::new(),
        ships: bw_server::persistence::TrackedDashMap::new(),
        factions: DashMap::new(),
        faction_tags: DashMap::new(),
        squadrons: bw_server::persistence::TrackedDashMap::new(),
        pending_squadron_invites: DashMap::new(),
        pending_alliances: DashMap::new(),
        contested_sectors: DashMap::new(),
        broadcaster,
        tick: AtomicU64::new(100),
        behavior_manager: RwLock::new(behavior_manager),
        coroutine_scheduler: RwLock::new(coroutine_scheduler),
        event_registry,
        event_dispatcher: RwLock::new(event_dispatcher),
        state_accessor: RwLock::new(None),
        script_logs: RwLock::new(ScriptLogBuffer::new(100)),
        metrics: bw_server::simulation::metrics::MetricsStore::new(),
        playtest_manager: PlaytestManager::new(10),
    }
}

/// Set up a state with sectors, ships, and players.
async fn setup_populated_state() -> (GameState, Uuid, Uuid, Uuid, Uuid) {
    let state = setup_test_state().await;

    // Create a faction for the player
    let faction_id = Uuid::new_v4();

    // Create a sector
    let sector = create_test_sector("Test Sector Alpha");
    let sector_id = sector.id;
    let sector_instance = bw_server::state::SectorInstance::new(sector);
    state.sectors.insert(sector_id, sector_instance);

    // Create a player first to get their ID
    let mut player = Player::new(
        "TestGM".to_string(),
        Uuid::new_v4(),
        Uuid::nil(), // Will update after ship creation
        sector_id,
    );
    let player_id = player.id;
    player.resources.reputation = 100;
    player.credits = 10000;

    // Create a player ship
    let player_ship = create_test_ship("Player Ship", player_id, sector_id, faction_id, true);
    let player_ship_id = player_ship.id;
    state.ships.insert(player_ship_id, player_ship);

    // Update player with active_ship_id
    player.active_ship_id = player_ship_id;
    state.player_data.insert(player_id, player);

    // Add player session
    let (tx, _rx) = tokio::sync::mpsc::channel(100);
    state.players.insert(player_id, bw_server::state::PlayerSession {
        player_id,
        ship_id: player_ship_id,
        sector_id,
        connection: Some(tx),
        playtest_id: None,
    });

    // Add ship to sector index
    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.ship_ids.insert(player_ship_id, ());
    }

    // Create an NPC ship
    let npc_ship = create_test_ship("Pirate Scum", Uuid::nil(), sector_id, Uuid::nil(), false);
    let npc_ship_id = npc_ship.id;
    state.ships.insert(npc_ship_id, npc_ship);

    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.ship_ids.insert(npc_ship_id, ());
    }

    (state, sector_id, player_id, player_ship_id, npc_ship_id)
}

// =============================================================================
// PlaytestManager Tests
// =============================================================================

#[tokio::test]
async fn test_playtest_manager_creation() {
    let manager = PlaytestManager::new(10);
    assert_eq!(manager.instance_count(), 0);
}

#[tokio::test]
async fn test_playtest_manager_limit() {
    let manager = PlaytestManager::new(2);

    // Create factions Arc for building instances
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    // Create first playtest
    let builder1 = PlaytestBuilder::new(Uuid::new_v4(), "Test 1".into(), 100, ForkConfig::default());
    let instance1 = builder1.build(factions.clone(), faction_tags.clone());
    assert!(manager.register(instance1).is_ok());

    // Create second playtest
    let builder2 = PlaytestBuilder::new(Uuid::new_v4(), "Test 2".into(), 100, ForkConfig::default());
    let instance2 = builder2.build(factions.clone(), faction_tags.clone());
    assert!(manager.register(instance2).is_ok());

    // Third should fail (limit reached)
    let builder3 = PlaytestBuilder::new(Uuid::new_v4(), "Test 3".into(), 100, ForkConfig::default());
    let instance3 = builder3.build(factions.clone(), faction_tags.clone());
    let result = manager.register(instance3);
    assert!(matches!(result, Err(PlaytestError::LimitReached)));
}

#[tokio::test]
async fn test_playtest_player_tracking() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let owner_id = Uuid::new_v4();
    let player2_id = Uuid::new_v4();

    let builder = PlaytestBuilder::new(owner_id, "Test".into(), 100, ForkConfig::default());
    let playtest_id = builder.id();
    let instance = builder.build(factions.clone(), faction_tags.clone());

    // Add owner as participant
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    let instance = manager.register(instance).unwrap();

    // Owner should be tracked
    assert!(manager.is_player_in_playtest(owner_id));
    assert_eq!(manager.get_player_playtest_id(owner_id), Some(playtest_id));

    // Player2 is not in playtest yet
    assert!(!manager.is_player_in_playtest(player2_id));

    // Join player2
    manager.join_playtest(playtest_id, player2_id, Uuid::new_v4(), Uuid::new_v4()).unwrap();
    assert!(manager.is_player_in_playtest(player2_id));

    // Leave
    manager.leave_playtest(player2_id).unwrap();
    assert!(!manager.is_player_in_playtest(player2_id));
}

#[tokio::test]
async fn test_playtest_cannot_join_twice() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let owner_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    let builder = PlaytestBuilder::new(owner_id, "Test".into(), 100, ForkConfig::default());
    let playtest_id = builder.id();
    let instance = builder.build(factions.clone(), faction_tags.clone());
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    manager.register(instance).unwrap();

    // Join once
    manager.join_playtest(playtest_id, player_id, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    // Try to join again - should fail
    let result = manager.join_playtest(playtest_id, player_id, Uuid::new_v4(), Uuid::new_v4());
    assert!(matches!(result, Err(PlaytestError::AlreadyInPlaytest)));
}

#[tokio::test]
async fn test_playtest_leave_not_in_playtest() {
    let manager = PlaytestManager::new(10);

    let player_id = Uuid::new_v4();
    let result = manager.leave_playtest(player_id);
    assert!(matches!(result, Err(PlaytestError::NotInPlaytest)));
}

// =============================================================================
// PlaytestInstance Tests
// =============================================================================

#[tokio::test]
async fn test_playtest_instance_tick_management() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    assert_eq!(instance.get_tick(), 100);
    assert_eq!(instance.increment_tick(), 101);
    assert_eq!(instance.get_tick(), 101);
}

#[tokio::test]
async fn test_playtest_instance_pause() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    // Playtests start paused by design (so GM can set up)
    assert!(instance.is_paused());
    instance.set_paused(false);
    assert!(!instance.is_paused());
    instance.set_paused(true);
    assert!(instance.is_paused());
}

#[tokio::test]
async fn test_playtest_instance_time_scale() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    // Default is 1.0x
    assert!((instance.get_time_scale() - 1.0).abs() < 0.01);

    instance.set_time_scale(2.0);
    assert!((instance.get_time_scale() - 2.0).abs() < 0.01);

    // Test clamping - max is 10x
    instance.set_time_scale(100.0);
    assert!((instance.get_time_scale() - 10.0).abs() < 0.01);

    // Test clamping - min is 0.1x
    instance.set_time_scale(0.001);
    assert!((instance.get_time_scale() - 0.1).abs() < 0.01);
}

#[tokio::test]
async fn test_playtest_participant_limit() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    // Add max participants
    for i in 0..PlaytestInstance::MAX_PARTICIPANTS {
        let result = instance.add_participant(
            Uuid::new_v4(),
            false,
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        assert!(result.is_ok(), "Failed to add participant {}: {:?}", i, result);
    }

    // One more should fail
    let result = instance.add_participant(
        Uuid::new_v4(),
        false,
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    assert!(matches!(result, Err(PlaytestError::ParticipantLimitReached)));
}

// =============================================================================
// State Forking Tests
// =============================================================================

#[tokio::test]
async fn test_fork_to_playtest_ships() {
    let (state, sector_id, player_id, player_ship_id, npc_ship_id) = setup_populated_state().await;

    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let config = ForkConfig::single_sector(sector_id);
    let builder = PlaytestBuilder::new(player_id, "Test Fork".into(), state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags);

    // Fork state
    state.fork_to_playtest(&instance, &config);

    // Verify ships were forked
    assert!(instance.ships.contains_key(&player_ship_id), "Player ship should be forked");
    assert!(instance.ships.contains_key(&npc_ship_id), "NPC ship should be forked");

    // Verify it's a copy, not a reference
    if let Some(mut playtest_ship) = instance.ships.get_mut(&npc_ship_id) {
        playtest_ship.hull_integrity = 50.0;
    }

    // Live ship should be unchanged
    let live_ship = state.ships.get(&npc_ship_id).unwrap();
    assert!((live_ship.hull_integrity - 100.0).abs() < 0.01, "Live ship should be unchanged");
}

#[tokio::test]
async fn test_fork_to_playtest_sectors() {
    let (state, sector_id, player_id, _player_ship_id, _npc_ship_id) = setup_populated_state().await;

    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let config = ForkConfig::single_sector(sector_id);
    let builder = PlaytestBuilder::new(player_id, "Test Fork".into(), state.get_tick(), config.clone());
    let instance = builder.build(factions, faction_tags);

    state.fork_to_playtest(&instance, &config);

    // Verify sector was forked
    assert!(instance.sectors.contains_key(&sector_id));

    let playtest_sector = instance.sectors.get(&sector_id).unwrap();
    assert_eq!(playtest_sector.sector.name, "Test Sector Alpha");
}

#[tokio::test]
async fn test_fork_excludes_other_sectors() {
    let state = setup_test_state().await;

    // Create two sectors
    let sector1 = create_test_sector("Sector 1");
    let sector1_id = sector1.id;
    state.sectors.insert(sector1_id, bw_server::state::SectorInstance::new(sector1));

    let sector2 = create_test_sector("Sector 2");
    let sector2_id = sector2.id;
    state.sectors.insert(sector2_id, bw_server::state::SectorInstance::new(sector2));

    // Create ships in each sector
    let ship1 = create_test_ship("Ship in Sector 1", Uuid::nil(), sector1_id, Uuid::nil(), false);
    let ship1_id = ship1.id;
    state.ships.insert(ship1_id, ship1);

    let ship2 = create_test_ship("Ship in Sector 2", Uuid::nil(), sector2_id, Uuid::nil(), false);
    let ship2_id = ship2.id;
    state.ships.insert(ship2_id, ship2);

    // Fork only sector 1
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let config = ForkConfig::single_sector(sector1_id);
    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, config.clone());
    let instance = builder.build(factions, faction_tags);

    state.fork_to_playtest(&instance, &config);

    // Only sector1 and ship1 should be forked
    assert!(instance.sectors.contains_key(&sector1_id));
    assert!(!instance.sectors.contains_key(&sector2_id));
    assert!(instance.ships.contains_key(&ship1_id));
    assert!(!instance.ships.contains_key(&ship2_id));
}

// =============================================================================
// Authorization Tests
// =============================================================================

#[tokio::test]
async fn test_playtest_authorization() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let owner_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();
    let random_id = Uuid::new_v4();

    let builder = PlaytestBuilder::new(owner_id, "Test".into(), 100, ForkConfig::default());
    let playtest_id = builder.id();
    let instance = builder.build(factions.clone(), faction_tags.clone());
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    manager.register(instance).unwrap();

    // Owner is authorized
    assert!(manager.is_authorized(playtest_id, owner_id));

    // Non-participant is not authorized
    assert!(!manager.is_authorized(random_id, random_id));

    // Join a player
    manager.join_playtest(playtest_id, player_id, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    // Regular participant is not authorized (only owner)
    assert!(!manager.is_authorized(playtest_id, player_id));
}

// =============================================================================
// Destroy Tests
// =============================================================================

#[tokio::test]
async fn test_playtest_destroy() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let owner_id = Uuid::new_v4();
    let player_id = Uuid::new_v4();

    let builder = PlaytestBuilder::new(owner_id, "Test".into(), 100, ForkConfig::default());
    let playtest_id = builder.id();
    let instance = builder.build(factions.clone(), faction_tags.clone());
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    manager.register(instance).unwrap();
    manager.join_playtest(playtest_id, player_id, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    // Both players should be in playtest
    assert!(manager.is_player_in_playtest(owner_id));
    assert!(manager.is_player_in_playtest(player_id));

    // Destroy
    manager.destroy(playtest_id).unwrap();

    // Neither should be in playtest anymore
    assert!(!manager.is_player_in_playtest(owner_id));
    assert!(!manager.is_player_in_playtest(player_id));

    // Playtest should be gone
    assert!(manager.get(playtest_id).is_none());
}

#[tokio::test]
async fn test_destroy_nonexistent_playtest() {
    let manager = PlaytestManager::new(10);

    let result = manager.destroy(Uuid::new_v4());
    assert!(matches!(result, Err(PlaytestError::NotFound)));
}

// =============================================================================
// ForkConfig Tests
// =============================================================================

#[test]
fn test_fork_config_single_sector() {
    let sector_id = Uuid::new_v4();
    let config = ForkConfig::single_sector(sector_id);

    assert_eq!(config.sectors.len(), 1);
    assert!(config.sectors.contains(&sector_id));
    assert!(config.include_ships);
    assert!(config.include_npcs);
    assert!(!config.include_other_players);
}

#[test]
fn test_fork_config_default() {
    let config = ForkConfig::default();

    assert!(config.sectors.is_empty());
    assert!(config.include_ships);
    assert!(config.include_npcs);
    assert!(!config.include_other_players);
}

// =============================================================================
// Entity Tracking Tests
// =============================================================================

#[tokio::test]
async fn test_created_ship_tracking() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    // No created ships initially
    assert_eq!(instance.created_ship_ids.len(), 0);

    // Track a new ship
    let new_ship_id = Uuid::new_v4();
    instance.created_ship_ids.insert(new_ship_id, ());

    assert_eq!(instance.created_ship_ids.len(), 1);
    assert!(instance.created_ship_ids.contains_key(&new_ship_id));
}

#[tokio::test]
async fn test_deleted_ship_tracking() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Test".into(), 100, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    // No deleted ships initially
    assert_eq!(instance.deleted_ship_ids.len(), 0);

    // Track a deleted ship
    let deleted_ship_id = Uuid::new_v4();
    instance.deleted_ship_ids.insert(deleted_ship_id, ());

    assert_eq!(instance.deleted_ship_ids.len(), 1);
    assert!(instance.deleted_ship_ids.contains_key(&deleted_ship_id));
}

// =============================================================================
// Message Routing Tests
// =============================================================================

#[tokio::test]
async fn test_message_destination_live() {
    let manager = PlaytestManager::new(10);

    let player_id = Uuid::new_v4();
    let destination = manager.get_destination(player_id);

    assert!(matches!(destination, bw_server::playtest::MessageDestination::Live));
}

#[tokio::test]
async fn test_message_destination_playtest() {
    let manager = PlaytestManager::new(10);
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let owner_id = Uuid::new_v4();

    let builder = PlaytestBuilder::new(owner_id, "Test".into(), 100, ForkConfig::default());
    let playtest_id = builder.id();
    let instance = builder.build(factions.clone(), faction_tags.clone());
    instance.add_participant(owner_id, true, Uuid::new_v4(), Uuid::new_v4()).unwrap();

    manager.register(instance).unwrap();

    let destination = manager.get_destination(owner_id);
    match destination {
        bw_server::playtest::MessageDestination::Playtest(instance) => {
            assert_eq!(instance.id, playtest_id);
        }
        _ => panic!("Expected Playtest destination"),
    }
}

// =============================================================================
// End-to-End Integration Tests (Full Server Flow)
// =============================================================================

/// Test the complete playtest lifecycle through GameState.
/// This simulates what happens when admin handlers create, fork, and manage playtests.
#[tokio::test]
async fn test_end_to_end_playtest_lifecycle() {
    let (state, sector_id, player_id, player_ship_id, npc_ship_id) = setup_populated_state().await;

    // === Step 1: Create a playtest ===
    let fork_config = ForkConfig::single_sector(sector_id);
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(
        player_id,
        "End-to-End Test".into(),
        state.get_tick(),
        fork_config.clone(),
    );
    let playtest_id = builder.id();
    let instance = builder.build(factions, faction_tags);

    // Fork live state into playtest
    state.fork_to_playtest(&instance, &fork_config);

    // Verify forked data
    assert!(instance.ships.contains_key(&player_ship_id));
    assert!(instance.ships.contains_key(&npc_ship_id));
    assert!(instance.sectors.contains_key(&sector_id));

    // Add owner as participant
    instance.add_participant(player_id, true, player_ship_id, sector_id).unwrap();

    // Update session to indicate player is in playtest (what handler does)
    if let Some(mut session) = state.players.get_mut(&player_id) {
        session.playtest_id = Some(playtest_id);
    }

    // Register with manager
    let instance = state.playtest_manager.register(instance).unwrap();

    // === Step 2: Verify routing ===
    assert!(state.playtest_manager.is_player_in_playtest(player_id));
    let destination = state.playtest_manager.get_destination(player_id);
    assert!(matches!(destination, bw_server::playtest::MessageDestination::Playtest(_)));

    // === Step 3: Modify state in playtest ===
    // Damage an NPC ship in the playtest
    if let Some(mut playtest_ship) = instance.ships.get_mut(&npc_ship_id) {
        playtest_ship.hull_integrity = 25.0;
    }

    // Verify live state is unchanged
    let live_ship = state.ships.get(&npc_ship_id).unwrap();
    assert!((live_ship.hull_integrity - 100.0).abs() < 0.01);

    // === Step 4: Add another participant ===
    // Create second player
    let player2 = create_test_player("Player2", sector_id, Uuid::nil());
    let player2_id = player2.id;
    state.player_data.insert(player2_id, player2);

    let player2_ship = create_test_ship("Player2 Ship", player2_id, sector_id, Uuid::nil(), true);
    let player2_ship_id = player2_ship.id;
    state.ships.insert(player2_ship_id, player2_ship);

    let (tx2, _rx2) = tokio::sync::mpsc::channel(100);
    state.players.insert(player2_id, bw_server::state::PlayerSession {
        player_id: player2_id,
        ship_id: player2_ship_id,
        sector_id,
        connection: Some(tx2),
        playtest_id: None,
    });

    // Join player2 to playtest
    state.playtest_manager.join_playtest(playtest_id, player2_id, player2_ship_id, sector_id).unwrap();

    // Update session
    if let Some(mut session) = state.players.get_mut(&player2_id) {
        session.playtest_id = Some(playtest_id);
    }

    assert_eq!(instance.participant_count(), 2);
    assert!(state.playtest_manager.is_player_in_playtest(player2_id));

    // === Step 5: Player leaves ===
    state.playtest_manager.leave_playtest(player2_id).unwrap();
    if let Some(mut session) = state.players.get_mut(&player2_id) {
        session.playtest_id = None;
    }

    assert_eq!(instance.participant_count(), 1);
    assert!(!state.playtest_manager.is_player_in_playtest(player2_id));

    // === Step 6: Destroy playtest ===
    state.playtest_manager.destroy(playtest_id).unwrap();
    if let Some(mut session) = state.players.get_mut(&player_id) {
        session.playtest_id = None;
    }

    assert!(!state.playtest_manager.is_player_in_playtest(player_id));
    assert!(state.playtest_manager.get(playtest_id).is_none());
}

/// Test playtest state isolation - changes in playtest don't affect live.
#[tokio::test]
async fn test_playtest_state_isolation() {
    let (state, sector_id, player_id, _player_ship_id, npc_ship_id) = setup_populated_state().await;

    // Record original values
    let original_hull = state.ships.get(&npc_ship_id).unwrap().hull_integrity;

    // Create playtest
    let fork_config = ForkConfig::single_sector(sector_id);
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(player_id, "Isolation Test".into(), state.get_tick(), fork_config.clone());
    let instance = builder.build(factions, faction_tags);
    state.fork_to_playtest(&instance, &fork_config);

    // Modify ship in playtest
    if let Some(mut ship) = instance.ships.get_mut(&npc_ship_id) {
        ship.hull_integrity = 10.0;
        ship.shield_strength = 5.0;
    }

    // Verify live state is unchanged
    let live_ship = state.ships.get(&npc_ship_id).unwrap();
    assert!((live_ship.hull_integrity - original_hull).abs() < 0.01);

    // Verify playtest state was changed
    let playtest_ship = instance.ships.get(&npc_ship_id).unwrap();
    assert!((playtest_ship.hull_integrity - 10.0).abs() < 0.01);
}

/// Test that playtest can modify its own sectors independently.
#[tokio::test]
async fn test_playtest_sector_isolation() {
    let (state, sector_id, player_id, _player_ship_id, _npc_ship_id) = setup_populated_state().await;

    // Create playtest
    let fork_config = ForkConfig::single_sector(sector_id);
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let builder = PlaytestBuilder::new(player_id, "Sector Isolation Test".into(), state.get_tick(), fork_config.clone());
    let instance = builder.build(factions, faction_tags);
    state.fork_to_playtest(&instance, &fork_config);

    // Spawn a new NPC in playtest
    let new_npc = create_test_ship("Test Enemy", Uuid::nil(), sector_id, Uuid::nil(), false);
    let new_npc_id = new_npc.id;
    instance.ships.insert(new_npc_id, new_npc);
    instance.created_ship_ids.insert(new_npc_id, ());

    if let Some(playtest_sector) = instance.sectors.get(&sector_id) {
        playtest_sector.ship_ids.insert(new_npc_id, ());
    }

    // Verify the ship exists in playtest but not in live
    assert!(instance.ships.contains_key(&new_npc_id));
    assert!(!state.ships.contains_key(&new_npc_id));

    // Verify tracking
    assert!(instance.created_ship_ids.contains_key(&new_npc_id));
}

/// Test concurrent playtest sessions don't interfere.
#[tokio::test]
async fn test_concurrent_playtests() {
    let state = setup_test_state().await;

    // Create sector
    let sector = create_test_sector("Shared Sector");
    let sector_id = sector.id;
    state.sectors.insert(sector_id, bw_server::state::SectorInstance::new(sector));

    // Create shared NPC
    let npc = create_test_ship("Shared NPC", Uuid::nil(), sector_id, Uuid::nil(), false);
    let npc_id = npc.id;
    state.ships.insert(npc_id, npc);

    // Create two playtests
    let gm1_id = Uuid::new_v4();
    let gm2_id = Uuid::new_v4();

    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();

    let config = ForkConfig::single_sector(sector_id);

    let builder1 = PlaytestBuilder::new(gm1_id, "Playtest 1".into(), 100, config.clone());
    let playtest1_id = builder1.id();
    let instance1 = builder1.build(factions.clone(), faction_tags.clone());
    state.fork_to_playtest(&instance1, &config);
    instance1.add_participant(gm1_id, true, Uuid::new_v4(), sector_id).unwrap();

    let builder2 = PlaytestBuilder::new(gm2_id, "Playtest 2".into(), 100, config.clone());
    let playtest2_id = builder2.id();
    let instance2 = builder2.build(factions.clone(), faction_tags.clone());
    state.fork_to_playtest(&instance2, &config);
    instance2.add_participant(gm2_id, true, Uuid::new_v4(), sector_id).unwrap();

    state.playtest_manager.register(instance1).unwrap();
    state.playtest_manager.register(instance2).unwrap();

    // Modify NPC in playtest 1
    if let Some(instance) = state.playtest_manager.get(playtest1_id) {
        if let Some(mut ship) = instance.ships.get_mut(&npc_id) {
            ship.hull_integrity = 50.0;
        }
    }

    // Modify NPC differently in playtest 2
    if let Some(instance) = state.playtest_manager.get(playtest2_id) {
        if let Some(mut ship) = instance.ships.get_mut(&npc_id) {
            ship.hull_integrity = 75.0;
        }
    }

    // Verify each playtest has its own state
    let playtest1 = state.playtest_manager.get(playtest1_id).unwrap();
    let playtest2 = state.playtest_manager.get(playtest2_id).unwrap();

    assert!((playtest1.ships.get(&npc_id).unwrap().hull_integrity - 50.0).abs() < 0.01);
    assert!((playtest2.ships.get(&npc_id).unwrap().hull_integrity - 75.0).abs() < 0.01);

    // Live state unchanged
    assert!((state.ships.get(&npc_id).unwrap().hull_integrity - 100.0).abs() < 0.01);
}

/// Test playtest tick management.
#[tokio::test]
async fn test_playtest_tick_independence() {
    let factions = Arc::new(DashMap::new());
    let faction_tags = Arc::new(DashMap::new());

    let builder = PlaytestBuilder::new(Uuid::new_v4(), "Tick Test".into(), 1000, ForkConfig::default());
    let instance = builder.build(factions, faction_tags);

    // Starts at fork tick
    assert_eq!(instance.get_tick(), 1000);

    // Can increment independently
    for _ in 0..100 {
        instance.increment_tick();
    }

    assert_eq!(instance.get_tick(), 1100);

    // Playtests start paused
    assert!(instance.is_paused());

    // Unpause and verify tick is unchanged
    instance.set_paused(false);
    assert!(!instance.is_paused());
    assert_eq!(instance.get_tick(), 1100);
}

/// Test session update consistency.
#[tokio::test]
async fn test_session_playtest_id_consistency() {
    let (state, sector_id, player_id, player_ship_id, _npc_ship_id) = setup_populated_state().await;

    // Verify session has no playtest initially
    {
        let session = state.players.get(&player_id).unwrap();
        assert!(session.playtest_id.is_none());
    }

    // Create playtest
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();
    let config = ForkConfig::single_sector(sector_id);

    let builder = PlaytestBuilder::new(player_id, "Session Test".into(), state.get_tick(), config.clone());
    let playtest_id = builder.id();
    let instance = builder.build(factions, faction_tags);
    state.fork_to_playtest(&instance, &config);
    instance.add_participant(player_id, true, player_ship_id, sector_id).unwrap();

    // Simulate handler: update session BEFORE registration (race condition fix)
    {
        let mut session = state.players.get_mut(&player_id).unwrap();
        session.playtest_id = Some(playtest_id);
    }

    // Register
    state.playtest_manager.register(instance).unwrap();

    // Verify session is updated
    {
        let session = state.players.get(&player_id).unwrap();
        assert_eq!(session.playtest_id, Some(playtest_id));
    }

    // Verify routing works
    assert!(state.playtest_manager.is_player_in_playtest(player_id));

    // Now destroy and verify cleanup
    state.playtest_manager.destroy(playtest_id).unwrap();

    // Simulate handler: clear session
    {
        let mut session = state.players.get_mut(&player_id).unwrap();
        session.playtest_id = None;
    }

    // Verify cleanup
    {
        let session = state.players.get(&player_id).unwrap();
        assert!(session.playtest_id.is_none());
    }
    assert!(!state.playtest_manager.is_player_in_playtest(player_id));
}
