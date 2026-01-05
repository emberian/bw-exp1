//! Playtest manager
//!
//! Manages all active playtest instances.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::config::{ForkConfig, PlaytestError};
use super::instance::PlaytestInstance;

/// Manages all active playtest instances.
pub struct PlaytestManager {
    /// Active playtests (playtest_id -> PlaytestInstance)
    instances: DashMap<Uuid, Arc<PlaytestInstance>>,

    /// Index: player_id -> playtest_id (for quick lookup)
    player_playtest: DashMap<Uuid, Uuid>,

    /// Simulation task handles (playtest_id -> JoinHandle)
    simulation_tasks: DashMap<Uuid, tokio::task::JoinHandle<()>>,

    /// Maximum concurrent playtests allowed
    max_instances: usize,
}

impl PlaytestManager {
    /// Create a new PlaytestManager.
    pub fn new(max_instances: usize) -> Self {
        Self {
            instances: DashMap::new(),
            player_playtest: DashMap::new(),
            simulation_tasks: DashMap::new(),
            max_instances,
        }
    }

    /// Register a simulation task handle for a playtest.
    pub fn register_simulation_task(&self, playtest_id: Uuid, handle: tokio::task::JoinHandle<()>) {
        self.simulation_tasks.insert(playtest_id, handle);
    }

    /// Get the number of active playtests.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Get a playtest by ID.
    pub fn get(&self, id: Uuid) -> Option<Arc<PlaytestInstance>> {
        self.instances.get(&id).map(|r| r.clone())
    }

    /// Get the playtest a player is currently in, if any.
    pub fn get_for_player(&self, player_id: Uuid) -> Option<Arc<PlaytestInstance>> {
        self.player_playtest
            .get(&player_id)
            .and_then(|playtest_id| self.instances.get(&*playtest_id))
            .map(|r| r.clone())
    }

    /// Check if a player is in any playtest.
    pub fn is_player_in_playtest(&self, player_id: Uuid) -> bool {
        self.player_playtest.contains_key(&player_id)
    }

    /// Get the playtest ID a player is in, if any.
    pub fn get_player_playtest_id(&self, player_id: Uuid) -> Option<Uuid> {
        self.player_playtest.get(&player_id).map(|r| *r)
    }

    /// List all active playtests.
    pub fn list_all(&self) -> Vec<Arc<PlaytestInstance>> {
        self.instances.iter().map(|r| r.clone()).collect()
    }

    /// Register a new playtest instance.
    ///
    /// This is called after fork_from_live() creates the instance.
    pub fn register(&self, instance: PlaytestInstance) -> Result<Arc<PlaytestInstance>, PlaytestError> {
        if self.instances.len() >= self.max_instances {
            return Err(PlaytestError::LimitReached);
        }

        let id = instance.id;
        let owner_id = instance.owner_id;
        let instance = Arc::new(instance);

        self.instances.insert(id, instance.clone());

        // Register owner as participant
        self.player_playtest.insert(owner_id, id);

        tracing::info!(
            playtest_id = %id,
            owner_id = %owner_id,
            name = %instance.name,
            "Playtest registered"
        );

        Ok(instance)
    }

    /// Join a player to an existing playtest.
    pub fn join_playtest(
        &self,
        playtest_id: Uuid,
        player_id: Uuid,
        ship_id: Uuid,
        sector_id: Uuid,
    ) -> Result<Arc<PlaytestInstance>, PlaytestError> {
        // Check if player is already in a playtest
        if self.player_playtest.contains_key(&player_id) {
            return Err(PlaytestError::AlreadyInPlaytest);
        }

        // Get the playtest
        let instance = self.instances.get(&playtest_id)
            .ok_or(PlaytestError::NotFound)?;

        // Add participant
        instance.add_participant(player_id, false, ship_id, sector_id)?;

        // Update index
        self.player_playtest.insert(player_id, playtest_id);

        tracing::info!(
            playtest_id = %playtest_id,
            player_id = %player_id,
            "Player joined playtest"
        );

        Ok(instance.clone())
    }

    /// Remove a player from their current playtest.
    pub fn leave_playtest(&self, player_id: Uuid) -> Result<(), PlaytestError> {
        // Get the playtest ID
        let playtest_id = self.player_playtest.remove(&player_id)
            .map(|(_, id)| id)
            .ok_or(PlaytestError::NotInPlaytest)?;

        // Remove from instance
        if let Some(instance) = self.instances.get(&playtest_id) {
            instance.remove_participant(player_id);

            // Remove from sector connections
            for sector in instance.sectors.iter() {
                sector.connections.remove(&player_id);
            }
        }

        tracing::info!(
            playtest_id = %playtest_id,
            player_id = %player_id,
            "Player left playtest"
        );

        Ok(())
    }

    /// Destroy a playtest and clean up all resources.
    pub fn destroy(&self, playtest_id: Uuid) -> Result<(), PlaytestError> {
        // Get the instance
        let instance = self.instances.remove(&playtest_id)
            .map(|(_, i)| i)
            .ok_or(PlaytestError::NotFound)?;

        // Abort the simulation task if running
        if let Some((_, handle)) = self.simulation_tasks.remove(&playtest_id) {
            handle.abort();
        }

        // Remove all participants from the index
        for participant in instance.participants.iter() {
            self.player_playtest.remove(&participant.player_id);
        }

        tracing::info!(
            playtest_id = %playtest_id,
            name = %instance.name,
            participant_count = instance.participants.len(),
            "Playtest destroyed"
        );

        // Arc will deallocate when all references are dropped
        Ok(())
    }

    /// Check if a player is authorized to manage a playtest.
    pub fn is_authorized(&self, playtest_id: Uuid, player_id: Uuid) -> bool {
        self.instances.get(&playtest_id)
            .map(|i| i.is_owner(player_id))
            .unwrap_or(false)
    }
}

/// Builder for creating PlaytestInstance from live GameState.
///
/// This is separate from PlaytestInstance to avoid circular dependencies
/// with GameState.
pub struct PlaytestBuilder {
    id: Uuid,
    name: String,
    owner_id: Uuid,
    created_at_tick: u64,
    config: ForkConfig,
}

impl PlaytestBuilder {
    /// Create a new builder.
    pub fn new(owner_id: Uuid, name: String, created_at_tick: u64, config: ForkConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            owner_id,
            created_at_tick,
            config,
        }
    }

    /// Get the playtest ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the fork configuration.
    pub fn config(&self) -> &ForkConfig {
        &self.config
    }

    /// Build the PlaytestInstance.
    ///
    /// This creates an empty instance that must be populated by the caller
    /// with forked state from GameState.
    pub fn build(
        self,
        factions: Arc<DashMap<Uuid, bw_core::models::Faction>>,
        faction_tags: Arc<DashMap<String, Uuid>>,
    ) -> PlaytestInstance {
        let (broadcaster, _) = broadcast::channel(1000);

        PlaytestInstance {
            id: self.id,
            name: self.name,
            owner_id: self.owner_id,
            created_at_tick: self.created_at_tick,
            participants: DashMap::new(),
            ships: DashMap::new(),
            player_data: DashMap::new(),
            sectors: DashMap::new(),
            squadrons: DashMap::new(),
            factions,
            faction_tags,
            tick: AtomicU64::new(self.created_at_tick), // Start at fork tick
            paused: AtomicBool::new(true), // Start paused so GM can set up
            time_scale: AtomicU32::new(1000), // 1.0x
            broadcaster,
            state_accessor: RwLock::new(None),
            created_ship_ids: DashMap::new(),
            deleted_ship_ids: DashMap::new(),
        }
    }
}
