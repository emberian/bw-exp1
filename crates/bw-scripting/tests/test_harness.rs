//! Shared test infrastructure for bw-scripting integration tests
//!
//! Provides a TestHarness that sets up:
//! - Script engine with loaded scripts
//! - Mock state provider for controlled state
//! - StateAccessor for script context
//! - Helpers for creating test entities

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

use bw_core::models::Position;
use bw_game::state::{
    AccessPermissions, SectorSnapshot, LocationSnapshot,
    StateAccessor, StateMutation, MutationResult, StateProvider, ShipChanges,
};
use bw_shared::dto::{Ship as ShipSnapshot, Player as PlayerSnapshot, PositionDto};
use bw_scripting::{
    ActionDispatcher, ActionRegistry, ScriptEngine, MissionRunner, MissionContext,
};

/// Mock state provider that allows controlled test state.
pub struct MockStateProvider {
    pub ships: RwLock<HashMap<Uuid, ShipSnapshot>>,
    pub players: RwLock<HashMap<Uuid, PlayerSnapshot>>,
    pub sectors: RwLock<HashMap<Uuid, SectorSnapshot>>,
    /// Mutations collected from apply_mutations calls
    pub applied_mutations: RwLock<Vec<StateMutation>>,
}

impl MockStateProvider {
    pub fn new() -> Self {
        Self {
            ships: RwLock::new(HashMap::new()),
            players: RwLock::new(HashMap::new()),
            sectors: RwLock::new(HashMap::new()),
            applied_mutations: RwLock::new(Vec::new()),
        }
    }

    /// Add a ship to the mock state
    pub fn add_ship(&self, ship: ShipSnapshot) {
        self.ships.write().insert(ship.id, ship);
    }

    /// Add a player to the mock state
    pub fn add_player(&self, player: PlayerSnapshot) {
        self.players.write().insert(player.id, player);
    }

    /// Add a sector to the mock state
    pub fn add_sector(&self, sector: SectorSnapshot) {
        self.sectors.write().insert(sector.id, sector);
    }

    /// Get all applied mutations (for test verification)
    pub fn take_applied_mutations(&self) -> Vec<StateMutation> {
        std::mem::take(&mut *self.applied_mutations.write())
    }

    /// Update a ship in the mock state (simulates mutation application)
    pub fn update_ship(&self, ship_id: Uuid, changes: &ShipChanges) {
        if let Some(ship) = self.ships.write().get_mut(&ship_id) {
            if let Some(pos) = &changes.position {
                ship.position = PositionDto { x: pos.x, y: pos.y, z: pos.z };
            }
            if let Some(hull) = changes.hull {
                ship.hull = hull;
            }
            if let Some(shields) = changes.shields {
                ship.shields = shields;
            }
            if let Some(status) = &changes.status {
                // Convert ShipStatusChange to the string status format
                use bw_game::state::ShipStatusChange;
                ship.status = match status {
                    ShipStatusChange::Idle => "idle".to_string(),
                    ShipStatusChange::InTransit { .. } => "in_transit".to_string(),
                    ShipStatusChange::Disabled => "disabled".to_string(),
                };
            }
        }
    }
}

impl StateProvider for MockStateProvider {
    fn get_ship(&self, ship_id: Uuid) -> Option<ShipSnapshot> {
        self.ships.read().get(&ship_id).cloned()
    }

    fn get_ships_in_sector(&self, sector_id: Uuid) -> Vec<ShipSnapshot> {
        self.ships.read()
            .values()
            .filter(|s| s.sector_id == sector_id)
            .cloned()
            .collect()
    }

    fn get_ships_in_range(&self, sector_id: Uuid, position: Position, range: f64) -> Vec<ShipSnapshot> {
        self.ships.read()
            .values()
            .filter(|s| {
                s.sector_id == sector_id && {
                    let dx = s.position.x - position.x;
                    let dy = s.position.y - position.y;
                    let dz = s.position.z - position.z;
                    (dx * dx + dy * dy + dz * dz).sqrt() <= range
                }
            })
            .cloned()
            .collect()
    }

    fn get_player(&self, player_id: Uuid) -> Option<PlayerSnapshot> {
        self.players.read().get(&player_id).cloned()
    }

    fn get_sector(&self, sector_id: Uuid) -> Option<SectorSnapshot> {
        self.sectors.read().get(&sector_id).cloned()
    }

    fn apply_mutations(&self, mutations: Vec<StateMutation>) -> Vec<MutationResult> {
        let mut results = Vec::with_capacity(mutations.len());

        for mutation in &mutations {
            // Apply the mutation to our mock state
            match mutation {
                StateMutation::ModifyShip { ship_id, changes } => {
                    self.update_ship(*ship_id, changes);
                    results.push(MutationResult::success(mutation.clone()));
                }
                StateMutation::ModifyPlayer { player_id, changes } => {
                    if let Some(player) = self.players.write().get_mut(player_id) {
                        if let Some(rep) = changes.reputation {
                            player.reputation = rep;
                        }
                        if let Some(delta) = changes.reputation_delta {
                            player.reputation += delta;
                        }
                        if let Some(fame) = changes.fame {
                            player.fame = fame;
                        }
                        if let Some(delta) = changes.fame_delta {
                            player.fame += delta;
                        }
                        if let Some(credits) = changes.credits {
                            player.credits = credits;
                        }
                        if let Some(delta) = changes.credits_delta {
                            player.credits += delta;
                        }
                    }
                    results.push(MutationResult::success(mutation.clone()));
                }
                _ => {
                    results.push(MutationResult::success(mutation.clone()));
                }
            }
        }

        // Store for test verification
        self.applied_mutations.write().extend(mutations);

        results
    }
}

/// Test harness for bw-scripting integration tests.
pub struct ScriptingTestHarness {
    pub engine: Arc<ScriptEngine>,
    pub state_provider: Arc<MockStateProvider>,
    pub state_accessor: Arc<StateAccessor>,
    pub action_registry: Arc<ActionRegistry>,
    pub action_dispatcher: ActionDispatcher,
}

impl ScriptingTestHarness {
    /// Create a new test harness with scripts loaded from the project's scripts directory.
    pub fn new() -> Self {
        let engine = Arc::new(ScriptEngine::new("../../scripts"));
        let state_provider = Arc::new(MockStateProvider::new());
        let state_accessor = Arc::new(StateAccessor::new(state_provider.clone()));
        let action_registry = Arc::new(ActionRegistry::new());

        let mut action_dispatcher = ActionDispatcher::new(action_registry.clone(), engine.clone());
        action_dispatcher.set_state_accessor(state_accessor.clone());

        Self {
            engine,
            state_provider,
            state_accessor,
            action_registry,
            action_dispatcher,
        }
    }

    /// Load a specific script.
    pub fn load_script(&self, path: &str) -> Result<(), bw_scripting::engine::ScriptError> {
        self.engine.load_script(path)
    }

    /// Load action scripts and initialize them.
    pub fn load_action_scripts(&self) -> Result<(), bw_scripting::engine::ScriptError> {
        // Load movement actions
        self.engine.load_script("actions/movement.rhai")?;

        // Initialize by calling the init function
        let _ = self.engine.call_function_dynamic(
            "actions/movement.rhai",
            "init",
            (),
        );

        Ok(())
    }

    /// Load mission scripts.
    pub fn load_mission_scripts(&self) -> Result<(), bw_scripting::engine::ScriptError> {
        self.engine.load_script("missions/random/pirate_attack.rhai")?;
        Ok(())
    }

    /// Create a test ship and add it to the mock state.
    pub fn create_ship(&self, name: &str, sector_id: Uuid, position: Position) -> ShipSnapshot {
        let ship = ShipSnapshot {
            id: Uuid::new_v4(),
            name: name.to_string(),
            owner_id: None,
            ship_class: "corvette".to_string(),
            position: PositionDto { x: position.x, y: position.y, z: position.z },
            hull: 100.0,
            shields: 100.0,
            status: "idle".to_string(),
            is_player: false,
            faction_tag: None,
            is_hostile: false,
            docked_at: None,
            sector_id,
            ammunition: 100.0,
            fuel: 100.0,
            morale: 75.0,
            experience: 100,
            faction_id: None,
            can_attack: true,
            can_move: true,
            attack: 10.0,
            defense: 10.0,
            speed: 100.0,
            sensor_range: 500.0,
            combat_stance: "balanced".to_string(),
            locked_target: None,
            cargo: vec![],
            cargo_capacity: 100,
            cargo_used: 0,
            upgrades: vec![],
        };
        self.state_provider.add_ship(ship.clone());
        ship
    }

    /// Create a player ship (owned by a player).
    pub fn create_player_ship(&self, player_id: Uuid, name: &str, sector_id: Uuid, position: Position) -> ShipSnapshot {
        let ship = ShipSnapshot {
            id: Uuid::new_v4(),
            name: name.to_string(),
            owner_id: Some(player_id),
            ship_class: "corvette".to_string(),
            position: PositionDto { x: position.x, y: position.y, z: position.z },
            hull: 100.0,
            shields: 100.0,
            status: "idle".to_string(),
            is_player: true,
            faction_tag: None,
            is_hostile: false,
            docked_at: None,
            sector_id,
            ammunition: 100.0,
            fuel: 100.0,
            morale: 75.0,
            experience: 100,
            faction_id: None,
            can_attack: true,
            can_move: true,
            attack: 10.0,
            defense: 10.0,
            speed: 100.0,
            sensor_range: 500.0,
            combat_stance: "balanced".to_string(),
            locked_target: None,
            cargo: vec![],
            cargo_capacity: 100,
            cargo_used: 0,
            upgrades: vec![],
        };
        self.state_provider.add_ship(ship.clone());
        ship
    }

    /// Create a test player and add it to the mock state.
    pub fn create_player(&self, username: &str, sector_id: Uuid) -> PlayerSnapshot {
        let player = PlayerSnapshot {
            id: Uuid::new_v4(),
            username: username.to_string(),
            reputation: 100,
            fame: 50,
            faction_tag: "TEST".to_string(),
            squadron_tag: None,
            is_online: true,
            is_admin: false,
            credits: 10000,
            game_mode: "standard".to_string(),
            owned_ships: vec![],
            active_ship_id: Uuid::nil(),
            sector_id,
            faction_id: Uuid::nil(),
            squadron_id: None,
            missions_completed: 0,
            missions_failed: 0,
            is_disgraced: false,
        };
        self.state_provider.add_player(player.clone());
        player
    }

    /// Create a test sector and add it to the mock state.
    pub fn create_sector(&self, name: &str) -> SectorSnapshot {
        let sector = SectorSnapshot {
            id: Uuid::new_v4(),
            name: name.to_string(),
            danger_level: "safe".to_string(),
            traffic_density: "light".to_string(),
            is_core_sector: false,
            controlling_faction: None,
            controlling_squadron: None,
            locations: vec![],
        };
        self.state_provider.add_sector(sector.clone());
        sector
    }

    /// Create a sector with a jumpgate location.
    pub fn create_sector_with_jumpgate(&self, name: &str, adjacent_to: Vec<Uuid>) -> SectorSnapshot {
        let jumpgate = LocationSnapshot {
            id: Uuid::new_v4(),
            name: "Jumpgate".to_string(),
            location_type: "Jumpgate".to_string(),
            position: PositionDto { x: 0.0, y: 0.0, z: 0.0 },
            faction_id: None,
            is_active: true,
            is_dockable: false,
        };

        let sector = SectorSnapshot {
            id: Uuid::new_v4(),
            name: name.to_string(),
            danger_level: "safe".to_string(),
            traffic_density: "light".to_string(),
            is_core_sector: false,
            controlling_faction: None,
            controlling_squadron: None,
            locations: vec![jumpgate],
        };

        // Note: adjacent_sectors would need to be added to SectorSnapshot if needed
        let _ = adjacent_to; // Currently not stored in SectorSnapshot

        self.state_provider.add_sector(sector.clone());
        sector
    }

    /// Create a MissionContext for testing mission scripts.
    pub fn create_mission_context(
        &self,
        script_path: &str,
        player: &PlayerSnapshot,
        ship: &ShipSnapshot,
    ) -> MissionContext {
        MissionContext {
            mission_id: Uuid::new_v4(),
            mission_type: "random".to_string(),
            script_path: script_path.to_string(),
            current_state: "start".to_string(),
            data: rhai::Map::new(),
            player_id: player.id,
            player_reputation: player.reputation,
            player_fame: player.fame,
            ship_id: ship.id,
            ship_hull: ship.hull,
            ship_ammo: ship.ammunition,
            ship_fuel: ship.fuel,
            crew_morale: ship.morale,
            crew_experience: ship.experience,
            sector_id: ship.sector_id,
        }
    }

    /// Get the MissionRunner for testing mission scripts.
    pub fn mission_runner(&self) -> MissionRunner<'_> {
        MissionRunner::new(&self.engine)
    }

    /// Take all mutations that were applied (for verification).
    pub fn take_applied_mutations(&self) -> Vec<StateMutation> {
        self.state_provider.take_applied_mutations()
    }

    /// Set permissions on the state accessor.
    pub fn set_permissions(&self, permissions: AccessPermissions) {
        self.state_accessor.set_permissions(permissions);
    }
}

impl Default for ScriptingTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to check if a mutation matches expected ship position change.
pub fn mutation_sets_ship_position(mutation: &StateMutation, ship_id: Uuid, expected_x: f64, expected_y: f64) -> bool {
    match mutation {
        StateMutation::ModifyShip { ship_id: id, changes } => {
            *id == ship_id && changes.position.as_ref().is_some_and(|pos| {
                (pos.x - expected_x).abs() < 0.01 && (pos.y - expected_y).abs() < 0.01
            })
        }
        _ => false,
    }
}

/// Helper to check if a mutation modifies ship status.
pub fn mutation_sets_ship_status(mutation: &StateMutation, ship_id: Uuid, expected_status: &str) -> bool {
    use bw_game::state::ShipStatusChange;
    match mutation {
        StateMutation::ModifyShip { ship_id: id, changes } => {
            *id == ship_id && changes.status.as_ref().is_some_and(|s| {
                let status_str = match s {
                    ShipStatusChange::Idle => "idle",
                    ShipStatusChange::InTransit { .. } => "in_transit",
                    ShipStatusChange::Disabled => "disabled",
                };
                status_str == expected_status
            })
        }
        _ => false,
    }
}

/// Helper to check if a mutation modifies player reputation.
pub fn mutation_changes_player_reputation(mutation: &StateMutation, player_id: Uuid) -> Option<i32> {
    match mutation {
        StateMutation::ModifyPlayer { player_id: id, changes } if *id == player_id => {
            changes.reputation_delta.or(changes.reputation)
        }
        _ => None,
    }
}
