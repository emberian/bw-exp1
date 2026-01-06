//! Integration tests that simulate gameplay scenarios
//!
//! These tests verify the server works correctly by simulating
//! client-server interactions through the WebSocket message protocol.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::collections::VecDeque;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use bw_core::models::{
    DangerLevel, Faction, FactionType, Mission, MissionStatus, MissionType, Player, Position,
    Sector, Ship, ShipClass, ShipStatus,
};
use bw_scripting::{
    ActionDispatcher, ActionRegistry, BehaviorManager, CoroutineScheduler, EventDispatcher,
    EventRegistry, debug::DebugController,
};
use bw_shared::{ChatChannel, ClientMessage, ServerMessage, deserialize_message, serialize_message};

use bw_runtime::scripting::ScriptLogBuffer;
use bw_runtime::state::{PlayerSession, SectorInstance};
use bw_runtime::{Database, GameState};

// =============================================================================
// Test Client - Simulates a WebSocket client
// =============================================================================

/// A simulated client that can send messages and receive responses.
pub struct TestClient {
    pub player_id: Uuid,
    pub ship_id: Uuid,
    pub sector_id: Uuid,
    /// Outgoing messages to send to server
    pub outbox: VecDeque<ClientMessage>,
    /// Incoming messages received from server
    pub inbox: VecDeque<ServerMessage>,
    /// Sender for the player's connection
    tx: mpsc::Sender<ServerMessage>,
    /// Receiver for incoming messages
    rx: mpsc::Receiver<ServerMessage>,
}

impl TestClient {
    pub fn new(player_id: Uuid, ship_id: Uuid, sector_id: Uuid) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            player_id,
            ship_id,
            sector_id,
            outbox: VecDeque::new(),
            inbox: VecDeque::new(),
            tx,
            rx,
        }
    }

    /// Get the sender channel (for registering with GameState)
    pub fn sender(&self) -> mpsc::Sender<ServerMessage> {
        self.tx.clone()
    }

    /// Queue a message to send
    pub fn send(&mut self, msg: ClientMessage) {
        self.outbox.push_back(msg);
    }

    /// Drain all pending incoming messages into inbox
    pub fn receive_all(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.inbox.push_back(msg);
        }
    }

    /// Get the next message of a specific type
    pub fn find_message<F>(&mut self, pred: F) -> Option<ServerMessage>
    where
        F: Fn(&ServerMessage) -> bool,
    {
        self.receive_all();
        self.inbox.iter().find(|m| pred(m)).cloned()
    }

    /// Get all received messages
    pub fn messages(&mut self) -> Vec<ServerMessage> {
        self.receive_all();
        self.inbox.iter().cloned().collect()
    }

    /// Clear inbox
    pub fn clear_inbox(&mut self) {
        self.receive_all();
        self.inbox.clear();
    }

    /// Check if we received a specific message type
    pub fn has_message<F>(&mut self, pred: F) -> bool
    where
        F: Fn(&ServerMessage) -> bool,
    {
        self.find_message(pred).is_some()
    }
}

// =============================================================================
// Test Harness - Sets up game state and manages clients
// =============================================================================

pub struct TestHarness {
    pub state: Arc<GameState>,
    pub clients: Vec<TestClient>,
}

impl TestHarness {
    /// Create a new test harness with in-memory database
    pub async fn new() -> Self {
        let state = setup_test_state().await;
        Self {
            state: Arc::new(state),
            clients: Vec::new(),
        }
    }

    /// Initialize scripting systems (needed for script actions)
    pub fn init_scripting(&self) {
        self.state.initialize_scripting();
        self.state.initialize_action_scripts();
    }

    /// Create a new sector
    pub fn create_sector(&self, name: &str, danger_level: DangerLevel) -> Uuid {
        let sector = Sector::new(name.to_string(), danger_level);
        let id = sector.id;
        self.state.sectors.insert(id, SectorInstance::new(sector));
        id
    }

    /// Create a faction
    pub fn create_faction(&self, name: &str, tag: &str) -> Uuid {
        let faction = Faction {
            id: Uuid::new_v4(),
            name: name.to_string(),
            tag: tag.to_string(),
            faction_type: FactionType::ContinuityCompact,
            description: String::new(),
            philosophy: String::new(),
            aesthetic: String::new(),
            default_standings: vec![],
            is_playable: true,
            is_hostile: false,
        };
        let id = faction.id;
        self.state.faction_tags.insert(tag.to_string(), id);
        self.state.factions.insert(id, faction);
        id
    }

    /// Create a player with ship and register a test client
    pub fn create_player(&mut self, username: &str, sector_id: Uuid) -> &TestClient {
        let faction_id = self.state.factions.iter().next().map(|f| *f.key()).unwrap_or(Uuid::nil());

        // Create player
        let mut player = Player::new(username.to_string(), Uuid::new_v4(), faction_id, sector_id);
        player.resources.reputation = 100;
        player.credits = 10000;
        let player_id = player.id;

        // Create ship
        let ship = Ship::new_player_ship(
            format!("{}'s Ship", username),
            player_id,
            ShipClass::PatrolCorvette,
            sector_id,
            faction_id,
        );
        let ship_id = ship.id;
        player.active_ship_id = ship_id;

        // Insert into state
        self.state.ships.insert(ship_id, ship);
        self.state.player_data.insert(player_id, player);

        // Add ship to sector
        if let Some(sector) = self.state.sectors.get(&sector_id) {
            sector.ship_ids.insert(ship_id, ());
        }

        // Create test client
        let client = TestClient::new(player_id, ship_id, sector_id);

        // Register session with connection
        self.state.players.insert(
            player_id,
            PlayerSession {
                player_id,
                ship_id,
                sector_id,
                connection: Some(client.sender()),
                connection_id: Some(Uuid::new_v4()),
                playtest_id: None,
            },
        );

        // Register connection with sector
        if let Some(sector) = self.state.sectors.get(&sector_id) {
            sector.connections.insert(player_id, client.sender());
        }

        self.clients.push(client);
        self.clients.last().unwrap()
    }

    /// Create an NPC ship in a sector
    pub fn spawn_npc(&self, name: &str, sector_id: Uuid, position: Position) -> Uuid {
        let ship = Ship::new_npc_ship(name.to_string(), ShipClass::PirateRaider, sector_id, position, None);
        let id = ship.id;
        self.state.ships.insert(id, ship);

        if let Some(sector) = self.state.sectors.get(&sector_id) {
            sector.ship_ids.insert(id, ());
        }
        id
    }

    /// Create a mission in a sector
    pub fn create_mission(
        &self,
        sector_id: Uuid,
        title: &str,
        mission_type: MissionType,
    ) -> Uuid {
        let mission = Mission::new(
            mission_type,
            title.to_string(),
            sector_id,
            mission_type.default_script_path().to_string(),
        );
        let id = mission.id;

        if let Some(sector) = self.state.sectors.get(&sector_id) {
            sector.missions.insert(id, mission);
        }
        id
    }

    /// Get a mutable reference to a client by player ID
    pub fn client_mut(&mut self, player_id: Uuid) -> Option<&mut TestClient> {
        self.clients.iter_mut().find(|c| c.player_id == player_id)
    }

    /// Advance the game tick
    pub fn tick(&self) -> u64 {
        self.state.increment_tick()
    }

    /// Run N ticks of the game loop (simplified - just increments tick)
    pub fn run_ticks(&self, n: u64) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Send a message directly to a player's channel
    pub async fn send_to_player(&self, player_id: Uuid, msg: ServerMessage) {
        if let Some(session) = self.state.players.get(&player_id) {
            if let Some(ref conn) = session.connection {
                let _ = conn.send(msg).await;
            }
        }
    }

    /// Broadcast to all players in a sector
    pub async fn broadcast_to_sector(&self, sector_id: Uuid, msg: ServerMessage) {
        if let Some(sector) = self.state.sectors.get(&sector_id) {
            for conn in sector.connections.iter() {
                let _ = conn.value().send(msg.clone()).await;
            }
        }
    }

    /// Link adjacent sectors
    pub fn link_sectors(&self, sector_a: Uuid, sector_b: Uuid) {
        if let Some(mut sector) = self.state.sectors.get_mut(&sector_a) {
            sector.sector.adjacent_sectors.push(sector_b);
        }
        if let Some(mut sector) = self.state.sectors.get_mut(&sector_b) {
            sector.sector.adjacent_sectors.push(sector_a);
        }
    }
}

// =============================================================================
// State Setup Helpers
// =============================================================================

async fn setup_test_state() -> GameState {
    let db = Database::new_in_memory().await.unwrap();
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

    let mut playtest_manager = bw_runtime::playtest::PlaytestManager::new(10);
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
        tick: AtomicU64::new(0),
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

// =============================================================================
// Tests: Basic State Setup
// =============================================================================

#[tokio::test]
async fn test_harness_creates_valid_state() {
    let harness = TestHarness::new().await;

    // State should be empty initially
    assert_eq!(harness.state.sectors.len(), 0);
    assert_eq!(harness.state.players.len(), 0);
    assert_eq!(harness.state.ships.len(), 0);
}

#[tokio::test]
async fn test_create_sector() {
    let harness = TestHarness::new().await;

    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);

    assert!(harness.state.sectors.contains_key(&sector_id));
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    assert_eq!(sector.sector.name, "Test Sector");
    assert_eq!(sector.sector.danger_level, DangerLevel::Safe);
}

#[tokio::test]
async fn test_create_player() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Starting Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let player_id = client.player_id;
    let ship_id = client.ship_id;

    // Player should exist
    assert!(harness.state.player_data.contains_key(&player_id));
    let player = harness.state.player_data.get(&player_id).unwrap();
    assert_eq!(player.username, "TestPlayer");

    // Ship should exist
    assert!(harness.state.ships.contains_key(&ship_id));
    let ship = harness.state.ships.get(&ship_id).unwrap();
    assert_eq!(ship.sector_id, sector_id);

    // Session should be registered
    assert!(harness.state.players.contains_key(&player_id));
    let session = harness.state.players.get(&player_id).unwrap();
    assert_eq!(session.sector_id, sector_id);

    // Ship should be in sector
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    assert!(sector.ship_ids.contains_key(&ship_id));
}

// =============================================================================
// Tests: Message Sending and Receiving
// =============================================================================

#[tokio::test]
async fn test_send_message_to_player() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");
    let client = harness.create_player("TestPlayer", sector_id);
    let player_id = client.player_id;

    // Send a message to the player
    let test_msg = ServerMessage::Notification {
        message: "Hello, player!".to_string(),
        notification_type: "test".to_string(),
    };
    harness.send_to_player(player_id, test_msg.clone()).await;

    // Client should receive it
    let client = harness.client_mut(player_id).unwrap();
    assert!(client.has_message(|m| matches!(m, ServerMessage::Notification { .. })));
}

#[tokio::test]
async fn test_broadcast_to_sector() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Create multiple players in same sector
    harness.create_player("Player1", sector_id);
    harness.create_player("Player2", sector_id);
    harness.create_player("Player3", sector_id);

    // Broadcast to sector
    let msg = ServerMessage::Notification {
        message: "Sector-wide alert!".to_string(),
        notification_type: "alert".to_string(),
    };
    harness.broadcast_to_sector(sector_id, msg).await;

    // All clients should receive it
    for client in &mut harness.clients {
        assert!(client.has_message(|m| matches!(
            m,
            ServerMessage::Notification { message, .. } if message == "Sector-wide alert!"
        )));
    }
}

// =============================================================================
// Tests: Ship and Player State
// =============================================================================

#[tokio::test]
async fn test_spawn_npc() {
    let harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Pirate Haven", DangerLevel::Dangerous);

    let npc_id = harness.spawn_npc("Pirate Raider", sector_id, Position::new(100.0, 0.0, 50.0));

    // NPC should exist
    assert!(harness.state.ships.contains_key(&npc_id));
    let npc = harness.state.ships.get(&npc_id).unwrap();
    assert_eq!(npc.name, "Pirate Raider");
    assert!(!npc.is_player_ship);
    assert_eq!(npc.sector_id, sector_id);

    // NPC should be in sector
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    assert!(sector.ship_ids.contains_key(&npc_id));
}

#[tokio::test]
async fn test_ship_in_multiple_sectors() {
    let mut harness = TestHarness::new().await;
    let sector_a = harness.create_sector("Sector Alpha", DangerLevel::Safe);
    let sector_b = harness.create_sector("Sector Beta", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Player starts in sector A
    let client = harness.create_player("Traveler", sector_a);
    let player_id = client.player_id;
    let ship_id = client.ship_id;

    // Verify ship is in sector A
    {
        let sector = harness.state.sectors.get(&sector_a).unwrap();
        assert!(sector.ship_ids.contains_key(&ship_id));
    }

    // Manually move ship to sector B (simulating travel completion)
    {
        // Remove from sector A
        if let Some(sector) = harness.state.sectors.get(&sector_a) {
            sector.ship_ids.remove(&ship_id);
        }

        // Update ship's sector
        if let Some(mut ship) = harness.state.ships.get_mut(&ship_id) {
            ship.sector_id = sector_b;
        }

        // Add to sector B
        if let Some(sector) = harness.state.sectors.get(&sector_b) {
            sector.ship_ids.insert(ship_id, ());
        }

        // Update session
        if let Some(mut session) = harness.state.players.get_mut(&player_id) {
            session.sector_id = sector_b;
        }
    }

    // Verify ship is now in sector B, not A
    {
        let sector_a = harness.state.sectors.get(&sector_a).unwrap();
        assert!(!sector_a.ship_ids.contains_key(&ship_id));

        let sector_b = harness.state.sectors.get(&sector_b).unwrap();
        assert!(sector_b.ship_ids.contains_key(&ship_id));
    }
}

// =============================================================================
// Tests: Missions
// =============================================================================

#[tokio::test]
async fn test_create_mission() {
    let harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Mission Hub", DangerLevel::Safe);

    let mission_id = harness.create_mission(sector_id, "Pirate Attack", MissionType::PirateIntercept);

    // Mission should exist in sector
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    assert!(sector.missions.contains_key(&mission_id));

    let mission = sector.missions.get(&mission_id).unwrap();
    assert_eq!(mission.title, "Pirate Attack");
    assert!(matches!(mission.status, MissionStatus::Available));
}

#[tokio::test]
async fn test_mission_assignment() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Mission Hub", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("MissionRunner", sector_id);
    let player_id = client.player_id;

    let mission_id = harness.create_mission(sector_id, "Test Mission", MissionType::SectorDefense);

    // Assign mission to player using the accept method
    if let Some(sector) = harness.state.sectors.get(&sector_id) {
        if let Some(mut mission) = sector.missions.get_mut(&mission_id) {
            mission.accept(player_id);
        }
    }

    // Verify assignment
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    let mission = sector.missions.get(&mission_id).unwrap();
    assert_eq!(mission.assigned_to, Some(player_id));
    assert!(matches!(mission.status, MissionStatus::InProgress));
}

// =============================================================================
// Tests: Ship Status Changes
// =============================================================================

#[tokio::test]
async fn test_ship_docking() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Station Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Docker", sector_id);
    let ship_id = client.ship_id;
    let station_id = Uuid::new_v4();

    // Ship starts as Idle
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::Idle));
    }

    // Change to Docked
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.status = ShipStatus::Docked { station_id };
    }

    // Verify
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::Docked { .. }));
    }
}

#[tokio::test]
async fn test_ship_damage() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Combat Zone", DangerLevel::Dangerous);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Fighter", sector_id);
    let ship_id = client.ship_id;

    // Ship starts at full health
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!((ship.hull_integrity - 100.0).abs() < 0.01);
    }

    // Apply damage
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.hull_integrity = 50.0;
        ship.shield_strength = 25.0;
    }

    // Verify damage
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!((ship.hull_integrity - 50.0).abs() < 0.01);
        assert!((ship.shield_strength - 25.0).abs() < 0.01);
    }
}

// =============================================================================
// Tests: Ship Movement Within Sector
// =============================================================================

#[tokio::test]
async fn test_ship_position_update() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Open Space", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Navigator", sector_id);
    let ship_id = client.ship_id;

    // Set initial position
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(0.0, 0.0, 0.0);
    }

    // Move ship to new position
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(100.0, 50.0, 0.0);
    }

    // Verify position updated
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!((ship.position.x - 100.0).abs() < 0.01);
        assert!((ship.position.y - 50.0).abs() < 0.01);
        assert!((ship.position.z - 0.0).abs() < 0.01);
    }
}

#[tokio::test]
async fn test_ship_in_transit_status() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Travel Zone", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Traveler", sector_id);
    let ship_id = client.ship_id;

    // Ship starts Idle
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::Idle));
    }

    // Set ship to InTransit toward a destination
    let destination = Position::new(500.0, 300.0, 0.0);
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.status = ShipStatus::InTransit {
            destination,
            target_id: None,
        };
    }

    // Verify InTransit status with correct destination
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        match &ship.status {
            ShipStatus::InTransit { destination: dest, target_id } => {
                assert!((dest.x - 500.0).abs() < 0.01);
                assert!((dest.y - 300.0).abs() < 0.01);
                assert!(target_id.is_none());
            }
            _ => panic!("Expected InTransit status"),
        }
    }
}

#[tokio::test]
async fn test_ship_in_transit_to_station() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Station Space", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Docker", sector_id);
    let ship_id = client.ship_id;

    let station_id = Uuid::new_v4();
    let station_position = Position::new(1000.0, 500.0, 0.0);

    // Set ship to InTransit toward a station
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.status = ShipStatus::InTransit {
            destination: station_position,
            target_id: Some(station_id),
        };
    }

    // Verify InTransit status with station target
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        match &ship.status {
            ShipStatus::InTransit { destination: _, target_id } => {
                assert_eq!(*target_id, Some(station_id));
            }
            _ => panic!("Expected InTransit status"),
        }
    }
}

#[tokio::test]
async fn test_ship_arrival_at_destination() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Travel Zone", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Traveler", sector_id);
    let ship_id = client.ship_id;

    let destination = Position::new(500.0, 300.0, 0.0);

    // Start at origin, set transit to destination
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(0.0, 0.0, 0.0);
        ship.status = ShipStatus::InTransit {
            destination,
            target_id: None,
        };
    }

    // Simulate arrival: update position to destination and set status back to Idle
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = destination;
        ship.status = ShipStatus::Idle;
    }

    // Verify ship arrived and is Idle
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::Idle));
        assert!((ship.position.x - 500.0).abs() < 0.01);
        assert!((ship.position.y - 300.0).abs() < 0.01);
    }
}

#[tokio::test]
async fn test_ship_movement_via_mutation() {
    use bw_game::state::{StateProvider, StateMutation, ShipChanges};

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Mover", sector_id);
    let ship_id = client.ship_id;

    // Set initial position
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(0.0, 0.0, 0.0);
    }

    // Move via mutation
    let new_position = Position::new(200.0, 150.0, 0.0);
    let mutation = StateMutation::ModifyShip {
        ship_id,
        changes: ShipChanges {
            position: Some(new_position),
            hull: None,
            shields: None,
            ammunition: None,
            fuel: None,
            morale: None,
            experience: None,
            status: None,
            combat_stance: None,
            locked_target: None,
            add_cargo: None,
            remove_cargo: None,
            install_upgrade: None,
            remove_upgrade_slot: None,
        },
    };

    let results = harness.state.apply_mutations(vec![mutation]);
    assert!(results[0].success);

    // Verify position updated
    let ship = harness.state.ships.get(&ship_id).unwrap();
    assert!((ship.position.x - 200.0).abs() < 0.01);
    assert!((ship.position.y - 150.0).abs() < 0.01);
}

#[tokio::test]
async fn test_ship_distance_calculation() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Player", sector_id);
    let ship_id = client.ship_id;

    // Place player ship at origin
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(0.0, 0.0, 0.0);
    }

    // Create NPC at known distance (3-4-5 triangle = distance 5)
    let npc_id = harness.spawn_npc("Target", sector_id, Position::new(3.0, 4.0, 0.0));

    // Calculate distance
    let player_pos = harness.state.ships.get(&ship_id).unwrap().position;
    let npc_pos = harness.state.ships.get(&npc_id).unwrap().position;
    let distance = player_pos.distance_to(&npc_pos);

    assert!((distance - 5.0).abs() < 0.01);
}

#[tokio::test]
async fn test_multiple_ships_moving_simultaneously() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Busy Space", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Create multiple ships
    let p1 = harness.create_player("Player1", sector_id);
    let ship1_id = p1.ship_id;

    let p2 = harness.create_player("Player2", sector_id);
    let ship2_id = p2.ship_id;

    let npc_id = harness.spawn_npc("NPC", sector_id, Position::new(0.0, 0.0, 0.0));

    // Set all ships to different transit destinations
    {
        let mut ship1 = harness.state.ships.get_mut(&ship1_id).unwrap();
        ship1.status = ShipStatus::InTransit {
            destination: Position::new(100.0, 0.0, 0.0),
            target_id: None,
        };
    }
    {
        let mut ship2 = harness.state.ships.get_mut(&ship2_id).unwrap();
        ship2.status = ShipStatus::InTransit {
            destination: Position::new(0.0, 100.0, 0.0),
            target_id: None,
        };
    }
    {
        let mut npc = harness.state.ships.get_mut(&npc_id).unwrap();
        npc.status = ShipStatus::InTransit {
            destination: Position::new(-100.0, -100.0, 0.0),
            target_id: None,
        };
    }

    // Verify all ships are in transit
    assert!(matches!(
        harness.state.ships.get(&ship1_id).unwrap().status,
        ShipStatus::InTransit { .. }
    ));
    assert!(matches!(
        harness.state.ships.get(&ship2_id).unwrap().status,
        ShipStatus::InTransit { .. }
    ));
    assert!(matches!(
        harness.state.ships.get(&npc_id).unwrap().status,
        ShipStatus::InTransit { .. }
    ));
}

#[tokio::test]
async fn test_ship_movement_cancellation() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Indecisive", sector_id);
    let ship_id = client.ship_id;

    let start_pos = Position::new(50.0, 50.0, 0.0);

    // Set initial position and start transit
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = start_pos;
        ship.status = ShipStatus::InTransit {
            destination: Position::new(500.0, 500.0, 0.0),
            target_id: None,
        };
    }

    // Simulate partial movement (halfway)
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(275.0, 275.0, 0.0);
    }

    // Cancel movement - return to Idle at current position
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.status = ShipStatus::Idle;
    }

    // Verify ship stopped mid-journey
    {
        let ship = harness.state.ships.get(&ship_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::Idle));
        assert!((ship.position.x - 275.0).abs() < 0.01);
        assert!((ship.position.y - 275.0).abs() < 0.01);
    }
}

// =============================================================================
// Tests: Player Resources
// =============================================================================

#[tokio::test]
async fn test_player_reputation_changes() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("RepPlayer", sector_id);
    let player_id = client.player_id;

    // Initial reputation
    {
        let player = harness.state.player_data.get(&player_id).unwrap();
        assert_eq!(player.resources.reputation, 100);
    }

    // Gain reputation
    {
        let mut player = harness.state.player_data.get_mut(&player_id).unwrap();
        player.resources.reputation += 50;
    }

    // Verify
    {
        let player = harness.state.player_data.get(&player_id).unwrap();
        assert_eq!(player.resources.reputation, 150);
    }
}

#[tokio::test]
async fn test_player_credits() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("RichPlayer", sector_id);
    let player_id = client.player_id;

    // Initial credits
    {
        let player = harness.state.player_data.get(&player_id).unwrap();
        assert_eq!(player.credits, 10000);
    }

    // Spend credits
    {
        let mut player = harness.state.player_data.get_mut(&player_id).unwrap();
        player.credits -= 2500;
    }

    // Verify
    {
        let player = harness.state.player_data.get(&player_id).unwrap();
        assert_eq!(player.credits, 7500);
    }
}

// =============================================================================
// Tests: Tick Management
// =============================================================================

#[tokio::test]
async fn test_tick_increment() {
    let harness = TestHarness::new().await;

    assert_eq!(harness.state.get_tick(), 0);

    harness.tick();
    assert_eq!(harness.state.get_tick(), 1);

    harness.run_ticks(10);
    assert_eq!(harness.state.get_tick(), 11);
}

// =============================================================================
// Tests: Adjacent Sectors
// =============================================================================

#[tokio::test]
async fn test_sector_links() {
    let harness = TestHarness::new().await;
    let sector_a = harness.create_sector("Alpha", DangerLevel::Safe);
    let sector_b = harness.create_sector("Beta", DangerLevel::Moderate);
    let sector_c = harness.create_sector("Gamma", DangerLevel::Dangerous);

    // Link A <-> B and B <-> C
    harness.link_sectors(sector_a, sector_b);
    harness.link_sectors(sector_b, sector_c);

    // Verify links
    {
        let sector = harness.state.sectors.get(&sector_a).unwrap();
        assert!(sector.sector.adjacent_sectors.contains(&sector_b));
        assert!(!sector.sector.adjacent_sectors.contains(&sector_c));
    }

    {
        let sector = harness.state.sectors.get(&sector_b).unwrap();
        assert!(sector.sector.adjacent_sectors.contains(&sector_a));
        assert!(sector.sector.adjacent_sectors.contains(&sector_c));
    }

    {
        let sector = harness.state.sectors.get(&sector_c).unwrap();
        assert!(sector.sector.adjacent_sectors.contains(&sector_b));
        assert!(!sector.sector.adjacent_sectors.contains(&sector_a));
    }
}

// =============================================================================
// Tests: Multiple Players Interaction
// =============================================================================

#[tokio::test]
async fn test_multiple_players_in_sector() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Busy Hub", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Create multiple players
    let p1 = harness.create_player("Player1", sector_id);
    let p1_id = p1.player_id;
    let p1_ship = p1.ship_id;

    let p2 = harness.create_player("Player2", sector_id);
    let p2_id = p2.player_id;
    let p2_ship = p2.ship_id;

    let p3 = harness.create_player("Player3", sector_id);
    let p3_id = p3.player_id;
    let p3_ship = p3.ship_id;

    // All players and ships should be in the sector
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    assert!(sector.ship_ids.contains_key(&p1_ship));
    assert!(sector.ship_ids.contains_key(&p2_ship));
    assert!(sector.ship_ids.contains_key(&p3_ship));
    assert!(sector.connections.contains_key(&p1_id));
    assert!(sector.connections.contains_key(&p2_id));
    assert!(sector.connections.contains_key(&p3_id));

    // Should have 3 ships total
    assert_eq!(sector.ship_ids.len(), 3);
    assert_eq!(sector.connections.len(), 3);
}

// =============================================================================
// Tests: Message Serialization (Verifying protocol works)
// =============================================================================

#[test]
fn test_client_message_serialization() {
    let msg = ClientMessage::Ping { timestamp: 12345 };
    let bytes = serialize_message(&msg).unwrap();
    let decoded: ClientMessage = deserialize_message(&bytes).unwrap();

    match decoded {
        ClientMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_server_message_serialization() {
    let msg = ServerMessage::Pong {
        timestamp: 12345,
        server_tick: 100,
    };
    let bytes = serialize_message(&msg).unwrap();
    let decoded: ServerMessage = deserialize_message(&bytes).unwrap();

    match decoded {
        ServerMessage::Pong { timestamp, server_tick } => {
            assert_eq!(timestamp, 12345);
            assert_eq!(server_tick, 100);
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_script_action_serialization() {
    let msg = ClientMessage::move_to_position(100.0, 200.0, 0.0);
    let bytes = serialize_message(&msg).unwrap();
    let decoded: ClientMessage = deserialize_message(&bytes).unwrap();

    match decoded {
        ClientMessage::ScriptAction { action, params } => {
            assert_eq!(action, "move_to_position");
            assert_eq!(params["x"], 100.0);
            assert_eq!(params["y"], 200.0);
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_chat_message_serialization() {
    let msg = ClientMessage::SendChat {
        message: "Hello, world!".to_string(),
        channel: ChatChannel::Sector,
    };
    let bytes = serialize_message(&msg).unwrap();
    let decoded: ClientMessage = deserialize_message(&bytes).unwrap();

    match decoded {
        ClientMessage::SendChat { message, channel } => {
            assert_eq!(message, "Hello, world!");
            assert_eq!(channel, ChatChannel::Sector);
        }
        _ => panic!("Wrong message type"),
    }
}

// =============================================================================
// Tests: StateProvider Interface
// =============================================================================

#[tokio::test]
async fn test_state_provider_get_ship() {
    use bw_game::state::StateProvider;

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let ship_id = client.ship_id;

    // Get ship via StateProvider
    let snapshot = harness.state.get_ship(ship_id);
    assert!(snapshot.is_some());

    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.id, ship_id);
    assert_eq!(snapshot.sector_id, sector_id);
    assert!(snapshot.is_player);
}

#[tokio::test]
async fn test_state_provider_get_ships_in_sector() {
    use bw_game::state::StateProvider;

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Create player and NPCs
    harness.create_player("TestPlayer", sector_id);
    harness.spawn_npc("NPC 1", sector_id, Position::new(100.0, 0.0, 0.0));
    harness.spawn_npc("NPC 2", sector_id, Position::new(200.0, 0.0, 0.0));

    // Get all ships in sector
    let ships = harness.state.get_ships_in_sector(sector_id);
    assert_eq!(ships.len(), 3);

    // Verify mix of player and NPC ships
    let player_ships = ships.iter().filter(|s| s.is_player).count();
    let npc_ships = ships.iter().filter(|s| !s.is_player).count();
    assert_eq!(player_ships, 1);
    assert_eq!(npc_ships, 2);
}

#[tokio::test]
async fn test_state_provider_get_ships_in_range() {
    use bw_game::state::StateProvider;

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Create player at origin
    let client = harness.create_player("TestPlayer", sector_id);
    let ship_id = client.ship_id;

    // Set player position to origin
    {
        let mut ship = harness.state.ships.get_mut(&ship_id).unwrap();
        ship.position = Position::new(0.0, 0.0, 0.0);
    }

    // Create NPCs at various distances
    harness.spawn_npc("Close NPC", sector_id, Position::new(50.0, 0.0, 0.0)); // 50 units
    harness.spawn_npc("Medium NPC", sector_id, Position::new(150.0, 0.0, 0.0)); // 150 units
    harness.spawn_npc("Far NPC", sector_id, Position::new(500.0, 0.0, 0.0)); // 500 units

    // Get ships within 100 units - should find player + close NPC
    let ships = harness.state.get_ships_in_range(sector_id, Position::new(0.0, 0.0, 0.0), 100.0);
    assert_eq!(ships.len(), 2);

    // Get ships within 200 units - should find player + close + medium
    let ships = harness.state.get_ships_in_range(sector_id, Position::new(0.0, 0.0, 0.0), 200.0);
    assert_eq!(ships.len(), 3);

    // Get ships within 1000 units - should find all
    let ships = harness.state.get_ships_in_range(sector_id, Position::new(0.0, 0.0, 0.0), 1000.0);
    assert_eq!(ships.len(), 4);
}

#[tokio::test]
async fn test_state_provider_get_player() {
    use bw_game::state::StateProvider;

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let player_id = client.player_id;

    // Get player via StateProvider
    let snapshot = harness.state.get_player(player_id);
    assert!(snapshot.is_some());

    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.id, player_id);
    assert_eq!(snapshot.username, "TestPlayer");
    assert_eq!(snapshot.reputation, 100);
    assert_eq!(snapshot.credits, 10000);
}

#[tokio::test]
async fn test_state_provider_get_sector() {
    use bw_game::state::StateProvider;

    let harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Moderate);

    // Get sector via StateProvider
    let snapshot = harness.state.get_sector(sector_id);
    assert!(snapshot.is_some());

    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.id, sector_id);
    assert_eq!(snapshot.name, "Test Sector");
}

// =============================================================================
// Tests: State Mutations
// =============================================================================

#[tokio::test]
async fn test_apply_ship_mutation() {
    use bw_game::state::{StateProvider, StateMutation, ShipChanges};

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let ship_id = client.ship_id;

    // Apply mutation to damage ship
    let mutation = StateMutation::ModifyShip {
        ship_id,
        changes: ShipChanges {
            hull: Some(50.0),
            shields: Some(25.0),
            ammunition: None,
            fuel: None,
            morale: None,
            experience: None,
            position: None,
            status: None,
            combat_stance: None,
            locked_target: None,
            add_cargo: None,
            remove_cargo: None,
            install_upgrade: None,
            remove_upgrade_slot: None,
        },
    };

    let results = harness.state.apply_mutations(vec![mutation]);
    assert_eq!(results.len(), 1);
    assert!(results[0].success);

    // Verify ship was modified
    let ship = harness.state.ships.get(&ship_id).unwrap();
    assert!((ship.hull_integrity - 50.0).abs() < 0.01);
    assert!((ship.shield_strength - 25.0).abs() < 0.01);
}

#[tokio::test]
async fn test_apply_player_mutation() {
    use bw_game::state::{StateProvider, StateMutation, PlayerChanges};

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let player_id = client.player_id;

    // Apply mutation to modify reputation and credits
    let mutation = StateMutation::ModifyPlayer {
        player_id,
        changes: PlayerChanges {
            reputation: Some(200),
            fame: Some(50),
            reputation_delta: None,
            fame_delta: None,
            credits: Some(20000),
            credits_delta: None,
        },
    };

    let results = harness.state.apply_mutations(vec![mutation]);
    assert_eq!(results.len(), 1);
    assert!(results[0].success);

    // Verify player was modified
    let player = harness.state.player_data.get(&player_id).unwrap();
    assert_eq!(player.resources.reputation, 200);
    assert_eq!(player.resources.fame, 50);
    assert_eq!(player.credits, 20000);
}

#[tokio::test]
async fn test_apply_spawn_ship_mutation() {
    use bw_game::state::{StateProvider, StateMutation, ShipSpawnConfig};

    let harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    // Get faction ID
    let faction_id = harness.state.factions.iter().next().map(|f| *f.key());

    // Apply mutation to spawn NPC ship
    let mutation = StateMutation::SpawnShip {
        config: ShipSpawnConfig {
            name: "Spawned Pirate".to_string(),
            ship_class: ShipClass::PirateRaider,
            sector_id,
            position: Position::new(100.0, 100.0, 0.0),
            faction_id,
            behavior_script: None,
        },
    };

    let results = harness.state.apply_mutations(vec![mutation]);
    assert_eq!(results.len(), 1);
    assert!(results[0].success);

    // Get spawned ship ID from result
    let ship_id = results[0].spawned_id.unwrap();

    // Verify ship was created
    assert!(harness.state.ships.contains_key(&ship_id));
    let ship = harness.state.ships.get(&ship_id).unwrap();
    assert_eq!(ship.name, "Spawned Pirate");
    assert_eq!(ship.sector_id, sector_id);

    // Verify ship is in sector
    let sector = harness.state.sectors.get(&sector_id).unwrap();
    assert!(sector.ship_ids.contains_key(&ship_id));
}

#[tokio::test]
async fn test_apply_multiple_mutations() {
    use bw_game::state::{StateProvider, StateMutation, ShipChanges, PlayerChanges};

    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let player_id = client.player_id;
    let ship_id = client.ship_id;

    // Apply multiple mutations in one call
    let mutations = vec![
        StateMutation::ModifyShip {
            ship_id,
            changes: ShipChanges {
                hull: Some(75.0),
                shields: None,
                ammunition: Some(50.0),
                fuel: Some(60.0),
                morale: None,
                experience: None,
                position: None,
                status: None,
                combat_stance: None,
                locked_target: None,
                add_cargo: None,
                remove_cargo: None,
                install_upgrade: None,
                remove_upgrade_slot: None,
            },
        },
        StateMutation::ModifyPlayer {
            player_id,
            changes: PlayerChanges {
                reputation: None,
                fame: None,
                reputation_delta: Some(10),
                fame_delta: Some(5),
                credits: None,
                credits_delta: Some(1000),
            },
        },
    ];

    let results = harness.state.apply_mutations(mutations);
    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert!(results[1].success);

    // Verify ship changes
    let ship = harness.state.ships.get(&ship_id).unwrap();
    assert!((ship.hull_integrity - 75.0).abs() < 0.01);
    assert!((ship.resources.ammunition - 50.0).abs() < 0.01);
    assert!((ship.resources.fuel - 60.0).abs() < 0.01);

    // Verify player changes (delta applied)
    let player = harness.state.player_data.get(&player_id).unwrap();
    assert_eq!(player.resources.reputation, 110); // 100 + 10
    assert_eq!(player.resources.fame, 5); // 0 + 5
    assert_eq!(player.credits, 11000); // 10000 + 1000
}

// =============================================================================
// Tests: Combat Engagement
// =============================================================================

#[tokio::test]
async fn test_combat_engagement_creation() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Combat Zone", DangerLevel::Dangerous);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("Fighter", sector_id);
    let player_ship_id = client.ship_id;

    let npc_id = harness.spawn_npc("Enemy Pirate", sector_id, Position::new(50.0, 0.0, 0.0));

    // Create a combat engagement
    let engagement_id = Uuid::new_v4();

    // Set ships to in-combat status
    {
        let mut ship = harness.state.ships.get_mut(&player_ship_id).unwrap();
        ship.status = ShipStatus::InCombat { engagement_id };
    }
    {
        let mut ship = harness.state.ships.get_mut(&npc_id).unwrap();
        ship.status = ShipStatus::InCombat { engagement_id };
    }

    // Verify both ships are in combat
    {
        let ship = harness.state.ships.get(&player_ship_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::InCombat { .. }));
    }
    {
        let ship = harness.state.ships.get(&npc_id).unwrap();
        assert!(matches!(ship.status, ShipStatus::InCombat { .. }));
    }
}

// =============================================================================
// Tests: Scripting System Initialization
// =============================================================================

#[tokio::test]
async fn test_scripting_initialization() {
    let harness = TestHarness::new().await;

    // Before initialization, state_accessor should be None
    {
        let accessor = harness.state.state_accessor.read();
        assert!(accessor.is_none());
    }

    // Initialize scripting
    harness.init_scripting();

    // After initialization, state_accessor should be Some
    {
        let accessor = harness.state.state_accessor.read();
        assert!(accessor.is_some());
    }

    // Action registry count should be accessible
    let _action_count = harness.state.action_registry.count();
}

// =============================================================================
// Tests: Player Disconnection Handling
// =============================================================================

#[tokio::test]
async fn test_player_disconnect_clears_connection() {
    let mut harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);
    harness.create_faction("Test Faction", "TEST");

    let client = harness.create_player("TestPlayer", sector_id);
    let player_id = client.player_id;

    // Verify player is connected
    {
        let session = harness.state.players.get(&player_id).unwrap();
        assert!(session.connection.is_some());
    }

    // Simulate disconnect by clearing connection
    {
        let mut session = harness.state.players.get_mut(&player_id).unwrap();
        session.connection = None;
        session.connection_id = None;
    }

    // Verify connection is cleared
    {
        let session = harness.state.players.get(&player_id).unwrap();
        assert!(session.connection.is_none());
    }

    // Player data should still exist (not deleted on disconnect)
    assert!(harness.state.player_data.contains_key(&player_id));
}

// =============================================================================
// Tests: Ship Destruction
// =============================================================================

#[tokio::test]
async fn test_ship_destruction() {
    use bw_game::state::{StateProvider, StateMutation, EntityType};

    let harness = TestHarness::new().await;
    let sector_id = harness.create_sector("Test Sector", DangerLevel::Safe);

    let npc_id = harness.spawn_npc("Doomed Ship", sector_id, Position::new(0.0, 0.0, 0.0));

    // Verify ship exists
    assert!(harness.state.ships.contains_key(&npc_id));
    {
        let sector = harness.state.sectors.get(&sector_id).unwrap();
        assert!(sector.ship_ids.contains_key(&npc_id));
    }

    // Destroy the ship
    let mutation = StateMutation::DestroyEntity {
        entity_id: npc_id,
        entity_type: EntityType::Ship,
    };

    let results = harness.state.apply_mutations(vec![mutation]);
    assert!(results[0].success);

    // Verify ship is gone
    assert!(!harness.state.ships.contains_key(&npc_id));
    {
        let sector = harness.state.sectors.get(&sector_id).unwrap();
        assert!(!sector.ship_ids.contains_key(&npc_id));
    }
}

// =============================================================================
// Tests: Faction Lookup
// =============================================================================

#[tokio::test]
async fn test_faction_tag_lookup() {
    let harness = TestHarness::new().await;

    let faction_id = harness.create_faction("Test Pirates", "PIRATE");

    // Test faction_tag lookup
    let tag = harness.state.faction_tag(Some(faction_id));
    assert_eq!(tag, Some("PIRATE".to_string()));

    // Test faction_tag_or with existing faction
    let tag = harness.state.faction_tag_or(faction_id, "UNKNOWN");
    assert_eq!(tag, "PIRATE");

    // Test faction_tag_or with non-existent faction
    let tag = harness.state.faction_tag_or(Uuid::nil(), "UNKNOWN");
    assert_eq!(tag, "UNKNOWN");
}
