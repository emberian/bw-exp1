//! Playtest instance
//!
//! A forked game state for GM playtesting.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use bw_core::models::{Faction, Mission, Player, Sector, Ship, Squadron};
use bw_game::state::StateAccessor;
use bw_game::systems::CombatEngagement;
use bw_scripting::{BehaviorManager, EntityBehavior};
use bw_shared::ServerMessage;

use crate::config::PlaytestError;

/// A forked game state for GM playtesting.
pub struct PlaytestInstance {
    /// Unique identifier for this playtest
    pub id: Uuid,

    /// Human-readable name (e.g., "Boss Fight Test #3")
    pub name: String,

    /// GM who created this playtest (owns it)
    pub owner_id: Uuid,

    /// Tick when created (from live state)
    pub created_at_tick: u64,

    /// Players participating in this playtest (player_id -> PlaytestParticipant)
    pub participants: DashMap<Uuid, PlaytestParticipant>,

    // === Forked Mutable State ===
    // These are COPIES, not references - modifications here don't affect live

    /// Forked ships (ship_id -> Ship)
    pub ships: DashMap<Uuid, Ship>,

    /// Forked player data (player_id -> Player)
    pub player_data: DashMap<Uuid, Player>,

    /// Forked sectors with runtime state (sector_id -> PlaytestSectorInstance)
    pub sectors: DashMap<Uuid, PlaytestSectorInstance>,

    /// Forked squadrons (squadron_id -> Squadron)
    pub squadrons: DashMap<Uuid, Squadron>,

    // === Shared Static Data (references to live state) ===

    /// Reference to live factions (read-only, shared)
    pub factions: Arc<DashMap<Uuid, Faction>>,

    /// Reference to live faction tag index
    pub faction_tags: Arc<DashMap<String, Uuid>>,

    // === Playtest-Specific State ===

    /// Current playtest tick (independent of live)
    pub tick: AtomicU64,

    /// Whether playtest simulation is paused
    pub paused: AtomicBool,

    /// Time scale stored as fixed-point (1000 = 1.0x speed)
    pub time_scale: AtomicU32,

    /// Broadcast channel for playtest-wide messages
    pub broadcaster: broadcast::Sender<ServerMessage>,

    /// State accessor configured for this playtest
    pub state_accessor: RwLock<Option<Arc<StateAccessor>>>,

    /// Behavior manager for script execution (separate from live server)
    pub behavior_manager: RwLock<BehaviorManager>,

    /// IDs of entities created in playtest (for promote tracking)
    pub created_ship_ids: DashMap<Uuid, ()>,

    /// IDs of entities deleted in playtest (for promote tracking)
    pub deleted_ship_ids: DashMap<Uuid, ()>,
}

impl PlaytestInstance {
    /// Initialize scripting systems after the instance is wrapped in Arc.
    ///
    /// This must be called after the instance is registered with the PlaytestManager
    /// because the StateAccessor needs a reference to the instance as a StateProvider.
    pub fn initialize_scripting(self: &Arc<Self>) {
        use bw_game::state::StateAccessor;

        // Create state accessor with self as provider
        let accessor = Arc::new(StateAccessor::new(self.clone()));

        // Wire up behavior manager
        {
            let mut bm = self.behavior_manager.write();
            bm.set_state_accessor(accessor.clone());
        }

        // Store accessor for direct access
        *self.state_accessor.write() = Some(accessor);

        tracing::debug!(
            playtest_id = %self.id,
            "Playtest scripting systems initialized"
        );
    }

    /// Attach behaviors for NPC ships that had behaviors in the live server.
    ///
    /// This should be called after `initialize_scripting()` to copy over
    /// behaviors from the live server's behavior manager.
    ///
    /// # Arguments
    /// * `live_behaviors` - List of behaviors from the live server's BehaviorManager
    pub fn attach_forked_behaviors(&self, live_behaviors: &[EntityBehavior]) {
        let bm = self.behavior_manager.write();
        let mut attached = 0;

        for behavior in live_behaviors {
            // Only attach if this entity was forked into the playtest
            if !self.ships.contains_key(&behavior.entity_id) {
                continue;
            }

            // Attach the same behavior script to this entity
            let sector_id = behavior.sector_id;
            match bm.attach(
                behavior.entity_id,
                behavior.entity_type,
                &behavior.script_path,
                sector_id,
            ) {
                Ok(_behavior_id) => {
                    attached += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        playtest_id = %self.id,
                        entity_id = %behavior.entity_id,
                        script = %behavior.script_path,
                        error = %e,
                        "Failed to attach forked behavior"
                    );
                }
            }
        }

        tracing::info!(
            playtest_id = %self.id,
            attached = attached,
            "Attached forked behaviors"
        );
    }

    /// Get the current tick.
    pub fn get_tick(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    /// Increment the tick and return the new value.
    pub fn increment_tick(&self) -> u64 {
        self.tick.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Check if the playtest is paused.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Set the paused state.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// Get the time scale as a float (1.0 = normal speed).
    pub fn get_time_scale(&self) -> f32 {
        self.time_scale.load(Ordering::Relaxed) as f32 / 1000.0
    }

    /// Set the time scale (1.0 = normal, 2.0 = 2x speed, etc.).
    pub fn set_time_scale(&self, scale: f32) {
        let fixed = (scale * 1000.0).clamp(100.0, 10000.0) as u32;
        self.time_scale.store(fixed, Ordering::Relaxed);
    }

    /// Check if a player is a participant.
    pub fn is_participant(&self, player_id: Uuid) -> bool {
        self.participants.contains_key(&player_id)
    }

    /// Check if a player is the owner (GM).
    pub fn is_owner(&self, player_id: Uuid) -> bool {
        self.owner_id == player_id
    }

    /// Maximum participants per playtest (prevents DoS).
    pub const MAX_PARTICIPANTS: usize = 20;

    /// Add a participant to the playtest.
    pub fn add_participant(
        &self,
        player_id: Uuid,
        is_gm: bool,
        ship_id: Uuid,
        sector_id: Uuid,
    ) -> Result<(), PlaytestError> {
        if self.participants.contains_key(&player_id) {
            return Err(PlaytestError::AlreadyInPlaytest);
        }

        if self.participants.len() >= Self::MAX_PARTICIPANTS {
            return Err(PlaytestError::ParticipantLimitReached);
        }

        self.participants.insert(
            player_id,
            PlaytestParticipant {
                player_id,
                is_gm,
                connection: None,
                playtest_ship_id: ship_id,
                playtest_sector_id: sector_id,
            },
        );

        Ok(())
    }

    /// Remove a participant from the playtest.
    pub fn remove_participant(&self, player_id: Uuid) -> Option<PlaytestParticipant> {
        self.participants.remove(&player_id).map(|(_, p)| p)
    }

    /// Set a participant's connection.
    pub fn set_participant_connection(
        &self,
        player_id: Uuid,
        connection: Option<mpsc::Sender<ServerMessage>>,
    ) {
        if let Some(mut participant) = self.participants.get_mut(&player_id) {
            participant.connection = connection;
        }
    }

    /// Get participant count.
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Get ship count.
    pub fn ship_count(&self) -> usize {
        self.ships.len()
    }

    /// Get sector count.
    pub fn sector_count(&self) -> usize {
        self.sectors.len()
    }

    /// Broadcast a message to all participants.
    pub async fn broadcast(&self, msg: ServerMessage) {
        for participant in self.participants.iter() {
            if let Some(ref conn) = participant.connection {
                let _ = conn.send(msg.clone()).await;
            }
        }
    }

    /// Try to broadcast without blocking.
    pub fn try_broadcast(&self, msg: ServerMessage) {
        for participant in self.participants.iter() {
            if let Some(ref conn) = participant.connection {
                let _ = conn.try_send(msg.clone());
            }
        }
    }
}

/// A participant in a playtest session.
#[derive(Debug)]
pub struct PlaytestParticipant {
    /// Player's ID
    pub player_id: Uuid,

    /// Whether this participant has GM powers in the playtest
    pub is_gm: bool,

    /// Connection for this player IN the playtest
    pub connection: Option<mpsc::Sender<ServerMessage>>,

    /// Ship ID in the playtest (may differ from live)
    pub playtest_ship_id: Uuid,

    /// Sector ID in the playtest
    pub playtest_sector_id: Uuid,
}

/// Playtest-specific sector instance (simpler than live SectorInstance).
pub struct PlaytestSectorInstance {
    /// Sector definition
    pub sector: Sector,

    /// Ships currently in this sector
    pub ship_ids: DashMap<Uuid, ()>,

    /// Active missions in this sector
    pub missions: DashMap<Uuid, Mission>,

    /// Active combat engagements
    pub combats: DashMap<Uuid, CombatEngagement>,

    /// Connected player sessions (player_id -> connection)
    pub connections: DashMap<Uuid, mpsc::Sender<ServerMessage>>,

    /// Sector-specific broadcast
    pub broadcaster: broadcast::Sender<ServerMessage>,
}

impl PlaytestSectorInstance {
    /// Create a new playtest sector instance.
    pub fn new(sector: Sector) -> Self {
        let (broadcaster, _) = broadcast::channel(100);

        Self {
            sector,
            ship_ids: DashMap::new(),
            missions: DashMap::new(),
            combats: DashMap::new(),
            connections: DashMap::new(),
            broadcaster,
        }
    }

    /// Broadcast a message to all connected players in this sector.
    pub async fn broadcast(&self, msg: ServerMessage) {
        for conn in self.connections.iter() {
            let _ = conn.value().send(msg.clone()).await;
        }
    }

    /// Try to broadcast without blocking.
    pub fn try_broadcast(&self, msg: ServerMessage) {
        for conn in self.connections.iter() {
            let _ = conn.value().try_send(msg.clone());
        }
    }

    /// Get count of connected players.
    pub fn player_count(&self) -> usize {
        self.connections.len()
    }
}
