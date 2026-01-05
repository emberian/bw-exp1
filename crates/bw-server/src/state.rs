//! Server state management

use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;

use bw_core::models::*;
use bw_scripting::{
    ScriptEngine, BehaviorManager, CoroutineScheduler, EventRegistry, EventDispatcher,
    StateAccessor, StateProvider, ShipSnapshot, PlayerSnapshot, SectorSnapshot,
    StateMutation, MutationResult,
};
use bw_shared::ServerMessage;

use crate::persistence::Database;
use crate::scripting::ScriptLogBuffer;

/// A pending squadron invitation.
#[derive(Clone, Debug)]
pub struct SquadronInvite {
    pub id: Uuid,
    pub squadron_id: Uuid,
    pub squadron_name: String,
    pub inviter_id: Uuid,
    pub inviter_name: String,
    pub invitee_id: Uuid,
    pub created_at: u64, // Tick when created
}

/// A pending alliance proposal.
#[derive(Clone, Debug)]
pub struct AllianceProposal {
    pub id: Uuid,
    pub from_squadron_id: Uuid,
    pub from_squadron_name: String,
    pub to_squadron_id: Uuid,
    pub created_at: u64,
}

/// A contested sector status.
#[derive(Clone, Debug)]
pub struct ContestedSector {
    pub sector_id: Uuid,
    pub defending_squadron_id: Uuid,
    pub attacking_squadron_id: Uuid,
    pub defender_influence: u32,
    pub attacker_influence: u32,
    pub started_at: u64,
}

/// Global game state.
pub struct GameState {
    /// Database connection
    pub db: Arc<Database>,

    /// Script engine
    pub scripts: Arc<ScriptEngine>,

    /// Active sectors (sector_id -> SectorInstance)
    pub sectors: DashMap<Uuid, SectorInstance>,

    /// Player data storage (player_id -> Player)
    /// Contains the full player model with reputation, fame, stats, etc.
    pub player_data: DashMap<Uuid, Player>,

    /// Player sessions (player_id -> PlayerSession)
    /// Contains connection and location tracking for online players
    pub players: DashMap<Uuid, PlayerSession>,

    /// All ships (ship_id -> Ship)
    pub ships: DashMap<Uuid, Ship>,

    /// Factions (faction_id -> Faction)
    pub factions: DashMap<Uuid, Faction>,

    /// Faction tag index for O(1) lookup (tag -> faction_id)
    pub faction_tags: DashMap<String, Uuid>,

    /// Squadrons (squadron_id -> Squadron)
    pub squadrons: DashMap<Uuid, Squadron>,

    /// Pending squadron invitations (invitee_id -> SquadronInvite)
    pub pending_squadron_invites: DashMap<Uuid, SquadronInvite>,

    /// Pending alliance proposals (to_squadron_id -> AllianceProposal)
    pub pending_alliances: DashMap<Uuid, AllianceProposal>,

    /// Contested sectors (sector_id -> ContestedSector)
    pub contested_sectors: DashMap<Uuid, ContestedSector>,

    /// Broadcast channel for server-wide messages
    pub broadcaster: broadcast::Sender<ServerMessage>,

    /// Current server tick
    pub tick: std::sync::atomic::AtomicU64,

    // === Scripting systems ===

    /// Behavior manager for entity AI scripts
    pub behavior_manager: RwLock<BehaviorManager>,

    /// Coroutine scheduler for async script operations
    pub coroutine_scheduler: RwLock<CoroutineScheduler>,

    /// Event registry for script subscriptions
    pub event_registry: Arc<EventRegistry>,

    /// Event dispatcher for triggering handlers
    pub event_dispatcher: RwLock<EventDispatcher>,

    /// State accessor for scripts (set after initialization)
    pub state_accessor: RwLock<Option<Arc<StateAccessor>>>,

    /// Script execution logs (ring buffer)
    pub script_logs: RwLock<ScriptLogBuffer>,
}

impl GameState {
    /// Create a new GameState with database connection.
    ///
    /// Loads factions and sectors from the database into the in-memory cache.
    pub async fn new(db: Database) -> anyhow::Result<Self> {
        let db = Arc::new(db);
        let scripts = Arc::new(ScriptEngine::new("scripts"));
        let (broadcaster, _) = broadcast::channel(1000);

        // Create scripting systems
        let event_registry = Arc::new(EventRegistry::new());
        let behavior_manager = BehaviorManager::new(scripts.clone());
        let coroutine_scheduler = CoroutineScheduler::new(scripts.clone());
        let event_dispatcher = EventDispatcher::new(event_registry.clone(), scripts.clone());

        let state = Self {
            db,
            scripts,
            sectors: DashMap::new(),
            player_data: DashMap::new(),
            players: DashMap::new(),
            ships: DashMap::new(),
            factions: DashMap::new(),
            faction_tags: DashMap::new(),
            squadrons: DashMap::new(),
            pending_squadron_invites: DashMap::new(),
            pending_alliances: DashMap::new(),
            contested_sectors: DashMap::new(),
            broadcaster,
            tick: std::sync::atomic::AtomicU64::new(0),
            // Scripting systems
            behavior_manager: RwLock::new(behavior_manager),
            coroutine_scheduler: RwLock::new(coroutine_scheduler),
            event_registry,
            event_dispatcher: RwLock::new(event_dispatcher),
            state_accessor: RwLock::new(None),
            script_logs: RwLock::new(ScriptLogBuffer::new(1000)),
        };

        // Load factions from database
        state.load_factions().await?;

        // Load sectors from database
        state.load_sectors().await?;

        Ok(state)
    }

    /// Initialize scripting systems after Arc<GameState> is created.
    ///
    /// This must be called after the GameState is wrapped in Arc because
    /// the StateAccessor needs a reference to the GameState as a StateProvider.
    pub fn initialize_scripting(self: &Arc<Self>) {
        // Create state accessor with self as provider
        let accessor = Arc::new(StateAccessor::new(self.clone()));

        // Wire up behavior manager
        {
            let mut bm = self.behavior_manager.write();
            bm.set_state_accessor(accessor.clone());
            bm.set_event_registry(self.event_registry.clone());
        }

        // Wire up coroutine scheduler
        {
            let mut cs = self.coroutine_scheduler.write();
            cs.set_state_accessor(accessor.clone());
        }

        // Wire up event dispatcher
        {
            let mut ed = self.event_dispatcher.write();
            ed.set_state_accessor(accessor.clone());
        }

        // Store accessor for direct access
        *self.state_accessor.write() = Some(accessor);

        tracing::info!("Scripting systems initialized");
    }

    /// Load all factions from database into memory.
    async fn load_factions(&self) -> anyhow::Result<()> {
        let factions = self.db.factions().find_all().await?;
        for faction in factions {
            // Index by tag for O(1) lookup
            self.faction_tags.insert(faction.tag.clone(), faction.id);
            self.factions.insert(faction.id, faction);
        }
        tracing::info!("Loaded {} factions", self.factions.len());
        Ok(())
    }

    /// Load all sectors and their locations from database into memory.
    async fn load_sectors(&self) -> anyhow::Result<()> {
        let sectors = self.db.sectors().find_all_with_locations().await?;
        for sector in sectors {
            let instance = SectorInstance::new(sector);
            self.sectors.insert(instance.sector.id, instance);
        }
        tracing::info!("Loaded {} sectors", self.sectors.len());
        Ok(())
    }

    pub fn get_tick(&self) -> u64 {
        self.tick.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn increment_tick(&self) -> u64 {
        self.tick.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }

    // === Script logging helpers ===

    /// Log a script info message.
    pub fn log_script_info(&self, message: impl Into<String>) {
        self.script_logs.write().log(crate::scripting::LogLevel::Info, message.into());
    }

    /// Log a script warning message.
    pub fn log_script_warning(&self, message: impl Into<String>) {
        self.script_logs.write().log(crate::scripting::LogLevel::Warning, message.into());
    }

    /// Log a script error message.
    pub fn log_script_error(&self, message: impl Into<String>) {
        self.script_logs.write().log(crate::scripting::LogLevel::Error, message.into());
    }

    // === Mutation application helpers ===

    /// Apply a single mutation to game state.
    fn apply_mutation(&self, mutation: StateMutation) -> MutationResult {
        match &mutation {
            StateMutation::ModifyShip { ship_id, changes } => {
                if let Some(mut ship) = self.ships.get_mut(ship_id) {
                    if let Some(hull) = changes.hull {
                        ship.hull_integrity = hull.clamp(0.0, 100.0);
                    }
                    if let Some(shields) = changes.shields {
                        ship.shield_strength = shields.clamp(0.0, 100.0);
                    }
                    if let Some(ammo) = changes.ammunition {
                        ship.resources.ammunition = ammo.clamp(0.0, 100.0);
                    }
                    if let Some(fuel) = changes.fuel {
                        ship.resources.fuel = fuel.clamp(0.0, 100.0);
                    }
                    if let Some(morale) = changes.morale {
                        ship.crew.morale = morale.clamp(0.0, 100.0);
                    }
                    if let Some(exp) = changes.experience {
                        ship.crew.experience = exp;
                    }
                    if let Some(ref pos) = changes.position {
                        ship.position = *pos;
                    }
                    if let Some(ref status_change) = changes.status {
                        ship.status = status_change.to_ship_status();
                    }
                    MutationResult::success(mutation)
                } else {
                    MutationResult::failure(mutation, "Ship not found")
                }
            }

            StateMutation::ModifyPlayer { player_id, changes } => {
                if let Some(mut player) = self.player_data.get_mut(player_id) {
                    if let Some(rep) = changes.reputation {
                        player.resources.reputation = rep;
                    }
                    if let Some(fame) = changes.fame {
                        player.resources.fame = fame;
                    }
                    if let Some(rep_delta) = changes.reputation_delta {
                        player.resources.apply_reputation_change(rep_delta);
                    }
                    if let Some(fame_delta) = changes.fame_delta {
                        player.resources.fame = (player.resources.fame + fame_delta).max(0);
                    }
                    MutationResult::success(mutation)
                } else {
                    MutationResult::failure(mutation, "Player not found")
                }
            }

            StateMutation::SpawnShip { config } => {
                // Create the NPC ship
                let ship = Ship::new_npc_ship(
                    config.name.clone(),
                    config.ship_class,
                    config.sector_id,
                    config.position,
                    config.faction_id,
                );
                let ship_id = ship.id;

                // Add to ships map
                self.ships.insert(ship_id, ship);

                // Add to sector
                if let Some(sector) = self.sectors.get(&config.sector_id) {
                    sector.ship_ids.insert(ship_id, ());
                }

                MutationResult::success_with_id(mutation, ship_id)
            }

            StateMutation::DestroyEntity { entity_id, entity_type } => {
                match entity_type {
                    bw_scripting::state::EntityType::Ship => {
                        if let Some((_, ship)) = self.ships.remove(entity_id) {
                            // Remove from sector
                            if let Some(sector) = self.sectors.get(&ship.sector_id) {
                                sector.ship_ids.remove(entity_id);
                            }
                            MutationResult::success(mutation)
                        } else {
                            MutationResult::failure(mutation, "Ship not found")
                        }
                    }
                    bw_scripting::state::EntityType::Mission => {
                        // Find and remove mission from any sector
                        for sector in self.sectors.iter() {
                            if sector.missions.remove(entity_id).is_some() {
                                return MutationResult::success(mutation);
                            }
                        }
                        MutationResult::failure(mutation, "Mission not found")
                    }
                    bw_scripting::state::EntityType::Station => {
                        // Find and remove station (location) from any sector
                        for mut sector in self.sectors.iter_mut() {
                            let initial_len = sector.sector.locations.len();
                            sector.sector.locations.retain(|loc| loc.id != *entity_id);
                            if sector.sector.locations.len() < initial_len {
                                return MutationResult::success(mutation);
                            }
                        }
                        MutationResult::failure(mutation, "Station not found")
                    }
                    bw_scripting::state::EntityType::Sector => {
                        if let Some((_, sector)) = self.sectors.remove(entity_id) {
                            // Move all ships in this sector to limbo (remove from tracking)
                            for ship_entry in sector.ship_ids.iter() {
                                if let Some(mut ship) = self.ships.get_mut(ship_entry.key()) {
                                    // Could relocate to a default sector instead
                                    ship.sector_id = Uuid::nil();
                                }
                            }
                            MutationResult::success(mutation)
                        } else {
                            MutationResult::failure(mutation, "Sector not found")
                        }
                    }
                }
            }

            StateMutation::EmitEvent { event_type, data: _, actor_id, target_id } => {
                // For now, just log the event. Full event dispatch will be done elsewhere.
                tracing::debug!(
                    event_type = %event_type,
                    actor = ?actor_id,
                    target = ?target_id,
                    "Script emitted event"
                );
                MutationResult::success(mutation)
            }
        }
    }
}

// === StateProvider Implementation ===

impl StateProvider for GameState {
    fn get_ship(&self, ship_id: Uuid) -> Option<ShipSnapshot> {
        self.ships.get(&ship_id)
            .map(|ship| ShipSnapshot::from_ship(&ship))
    }

    fn get_ships_in_sector(&self, sector_id: Uuid) -> Vec<ShipSnapshot> {
        self.sectors.get(&sector_id)
            .map(|sector| {
                sector.ship_ids.iter()
                    .filter_map(|entry| self.ships.get(entry.key()))
                    .map(|ship| ShipSnapshot::from_ship(&ship))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_ships_in_range(&self, sector_id: Uuid, position: Position, range: f64) -> Vec<ShipSnapshot> {
        self.sectors.get(&sector_id)
            .map(|sector| {
                sector.ship_ids.iter()
                    .filter_map(|entry| self.ships.get(entry.key()))
                    .filter(|ship| ship.position.distance_to(&position) <= range)
                    .map(|ship| ShipSnapshot::from_ship(&ship))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_player(&self, player_id: Uuid) -> Option<PlayerSnapshot> {
        self.player_data.get(&player_id)
            .map(|player| PlayerSnapshot::from_player(&player))
    }

    fn get_sector(&self, sector_id: Uuid) -> Option<SectorSnapshot> {
        self.sectors.get(&sector_id)
            .map(|si| SectorSnapshot::from_sector(&si.sector))
    }

    fn apply_mutations(&self, mutations: Vec<StateMutation>) -> Vec<MutationResult> {
        mutations.into_iter()
            .map(|m| self.apply_mutation(m))
            .collect()
    }
}

/// A sector instance with runtime state.
pub struct SectorInstance {
    /// Sector data
    pub sector: Sector,

    /// Ships currently in this sector
    pub ship_ids: DashMap<Uuid, ()>,

    /// Active missions in this sector
    pub missions: DashMap<Uuid, Mission>,

    /// Active combat engagements
    pub combats: DashMap<Uuid, bw_core::systems::CombatEngagement>,

    /// Connected player sessions
    pub connections: DashMap<Uuid, tokio::sync::mpsc::Sender<ServerMessage>>,

    /// Sector-specific broadcast
    pub broadcaster: broadcast::Sender<ServerMessage>,
}

impl SectorInstance {
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

    /// Get count of online players in sector.
    pub fn player_count(&self) -> usize {
        self.connections.len()
    }
}

/// A player's session state.
pub struct PlayerSession {
    pub player_id: Uuid,
    pub ship_id: Uuid,
    pub sector_id: Uuid,
    pub connection: Option<tokio::sync::mpsc::Sender<ServerMessage>>,
}
