//! Server state management

use std::sync::Arc;
use anyhow::Context;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Default capacity for the script log buffer.
/// Increase if you need to retain more history for debugging.
pub const DEFAULT_SCRIPT_LOG_CAPACITY: usize = 1000;

use bw_core::models::*;
use bw_scripting::{
    ScriptEngine, BehaviorManager, CoroutineScheduler, EventRegistry, EventDispatcher,
    ActionRegistry, ActionDispatcher,
    StateAccessor, StateProvider, ShipSnapshot, PlayerSnapshot, SectorSnapshot,
    StateMutation, MutationResult, FileStore,
    debug::DebugController,
};
use bw_shared::ServerMessage;

use crate::persistence::{Database, Persistence, TrackedDashMap};
use crate::playtest::{ForkConfig, PlaytestInstance, PlaytestSectorInstance, PlaytestManager};
use crate::scripting::ScriptLogBuffer;
use crate::simulation::metrics::MetricsStore;

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

    /// Streaming persistence worker handle
    pub persist: Persistence,

    /// Script engine
    pub scripts: Arc<ScriptEngine>,

    /// Active sectors (sector_id -> SectorInstance)
    pub sectors: DashMap<Uuid, SectorInstance>,

    /// Player data storage (player_id -> Player)
    /// Contains the full player model with reputation, fame, stats, etc.
    /// Uses TrackedDashMap for automatic dirty tracking.
    pub player_data: TrackedDashMap<Uuid, Player>,

    /// Player sessions (player_id -> PlayerSession)
    /// Contains connection and location tracking for online players
    pub players: DashMap<Uuid, PlayerSession>,

    /// All ships (ship_id -> Ship)
    /// Uses TrackedDashMap for automatic dirty tracking.
    pub ships: TrackedDashMap<Uuid, Ship>,

    /// Factions (faction_id -> Faction)
    pub factions: DashMap<Uuid, Faction>,

    /// Faction tag index for O(1) lookup (tag -> faction_id)
    pub faction_tags: DashMap<String, Uuid>,

    /// Squadrons (squadron_id -> Squadron)
    /// Uses TrackedDashMap for automatic dirty tracking.
    pub squadrons: TrackedDashMap<Uuid, Squadron>,

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

    /// Action registry for script-handled client actions
    pub action_registry: Arc<ActionRegistry>,

    /// Action dispatcher for routing ScriptAction messages
    pub action_dispatcher: RwLock<ActionDispatcher>,

    /// State accessor for scripts (set after initialization)
    pub state_accessor: RwLock<Option<Arc<StateAccessor>>>,

    /// Script execution logs (ring buffer)
    pub script_logs: RwLock<ScriptLogBuffer>,

    /// Performance metrics store
    pub metrics: MetricsStore,

    /// Playtest manager for GM testing sessions
    pub playtest_manager: PlaytestManager,

    /// Debug controller for script debugging sessions
    pub debug_controller: Arc<DebugController>,
}

impl GameState {
    /// Create a new GameState with database connection.
    ///
    /// Loads factions and sectors from the database into the in-memory cache.
    pub async fn new(db: Database) -> anyhow::Result<Self> {
        // Spawn persistence worker before wrapping db in Arc
        let persist = db.spawn_persistence();

        let db = Arc::new(db);
        let scripts = Arc::new(ScriptEngine::new("scripts"));
        let (broadcaster, _) = broadcast::channel(1000);

        // Create scripting systems
        let event_registry = Arc::new(EventRegistry::new());
        let action_registry = Arc::new(ActionRegistry::new());
        let debug_controller = Arc::new(DebugController::new());
        let mut behavior_manager = BehaviorManager::new(scripts.clone());
        behavior_manager.set_debug_controller(debug_controller.clone());
        let coroutine_scheduler = CoroutineScheduler::new(scripts.clone());
        let event_dispatcher = EventDispatcher::new(event_registry.clone(), scripts.clone());
        let action_dispatcher = ActionDispatcher::new(action_registry.clone(), scripts.clone());

        // Create playtest manager and wire debug controller for cleanup on destruction
        let mut playtest_manager = PlaytestManager::new(10); // Max 10 concurrent playtests
        playtest_manager.set_debug_controller(debug_controller.clone());

        let state = Self {
            db,
            persist,
            scripts,
            sectors: DashMap::new(),
            player_data: TrackedDashMap::new(),
            players: DashMap::new(),
            ships: TrackedDashMap::new(),
            factions: DashMap::new(),
            faction_tags: DashMap::new(),
            squadrons: TrackedDashMap::new(),
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
            action_registry,
            action_dispatcher: RwLock::new(action_dispatcher),
            state_accessor: RwLock::new(None),
            script_logs: RwLock::new(ScriptLogBuffer::new(DEFAULT_SCRIPT_LOG_CAPACITY)),
            metrics: MetricsStore::new(),
            playtest_manager,
            debug_controller,
        };

        // Load factions from database
        state.load_factions().await
            .context("Failed to load factions from database")?;

        // Load sectors from database
        state.load_sectors().await
            .context("Failed to load sectors from database")?;

        Ok(state)
    }

    /// Initialize scripting systems after Arc<GameState> is created.
    ///
    /// This must be called after the GameState is wrapped in Arc because
    /// the StateAccessor needs a reference to the GameState as a StateProvider.
    pub fn initialize_scripting(self: &Arc<Self>) {
        // Create state accessor with self as provider
        let accessor = Arc::new(StateAccessor::new(self.clone()));

        // Create script state store for persistence
        let state_store = match FileStore::shared("./data/script_state") {
            Ok(store) => Some(store),
            Err(e) => {
                tracing::warn!("Failed to create script state store: {}. Behavior state will not persist.", e);
                None
            }
        };

        // Wire up behavior manager
        {
            let mut bm = self.behavior_manager.write();
            bm.set_state_accessor(accessor.clone());
            bm.set_event_registry(self.event_registry.clone());
            if let Some(store) = state_store {
                bm.set_state_store(store);
            }
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

        // Wire up action dispatcher
        {
            let mut ad = self.action_dispatcher.write();
            ad.set_state_accessor(accessor.clone());
        }

        // Store accessor for direct access
        *self.state_accessor.write() = Some(accessor);

        tracing::info!("Scripting systems initialized");
    }

    /// Initialize action scripts by calling their init() functions.
    ///
    /// This registers action handlers defined in scripts under `scripts/actions/`.
    /// Must be called after `initialize_scripting()`.
    pub fn initialize_action_scripts(self: &Arc<Self>) {
        use bw_scripting::{ScriptExecutionContext, ExecutionGuard};

        let accessor = match self.state_accessor.read().clone() {
            Some(a) => a,
            None => {
                tracing::error!("Cannot initialize action scripts: state accessor not set");
                return;
            }
        };

        // Find action scripts
        let action_scripts: Vec<String> = self.scripts.loaded_scripts()
            .into_iter()
            .filter(|name| name.starts_with("actions/"))
            .collect();

        if action_scripts.is_empty() {
            tracing::info!("No action scripts found to initialize");
            return;
        }

        let mut initialized = 0;
        let mut failed = 0;

        for script_name in &action_scripts {
            // Set up execution context with action registry
            let ctx = ScriptExecutionContext::new(accessor.clone())
                .with_script_path(script_name)
                .with_action_registry(self.action_registry.clone());

            // Enter execution context
            let _guard = match ExecutionGuard::enter(ctx) {
                Ok(g) => g,
                Err(e) => {
                    tracing::warn!(
                        script = %script_name,
                        error = %e,
                        "Failed to enter execution context for action script"
                    );
                    failed += 1;
                    continue;
                }
            };

            // Call init() if it exists
            match self.scripts.call_function::<()>(script_name, "init", ()) {
                Ok(()) => {
                    tracing::debug!(script = %script_name, "Action script initialized");
                    initialized += 1;
                }
                Err(e) => {
                    // Check if it's just "function not found" - that's okay
                    let err_str = format!("{}", e);
                    if err_str.contains("not found") || err_str.contains("Function not found") {
                        tracing::debug!(
                            script = %script_name,
                            "Action script has no init() function, skipping"
                        );
                    } else {
                        tracing::warn!(
                            script = %script_name,
                            error = %e,
                            "Failed to initialize action script"
                        );
                        failed += 1;
                    }
                }
            }
        }

        let registered = self.action_registry.count();
        tracing::info!(
            scripts = initialized,
            failed = failed,
            actions = registered,
            "Action scripts initialized"
        );
    }

    /// Reinitialize a single action script after reload.
    ///
    /// This unregisters old handlers from the script and calls init() again.
    /// Should be called after a script file is reloaded.
    pub fn reinitialize_action_script(self: &Arc<Self>, script_path: &str) {
        // Only process action scripts
        if !script_path.starts_with("actions/") {
            return;
        }

        use bw_scripting::{ScriptExecutionContext, ExecutionGuard};

        let accessor = match self.state_accessor.read().clone() {
            Some(a) => a,
            None => {
                tracing::warn!("Cannot reinitialize action script: state accessor not set");
                return;
            }
        };

        // Unregister old handlers from this script
        self.action_registry.unregister_for_script(script_path);

        // Set up execution context
        let ctx = ScriptExecutionContext::new(accessor)
            .with_script_path(script_path)
            .with_action_registry(self.action_registry.clone());

        let _guard = match ExecutionGuard::enter(ctx) {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!(
                    script = %script_path,
                    error = %e,
                    "Failed to enter execution context for action script reinit"
                );
                return;
            }
        };

        // Call init() if it exists
        match self.scripts.call_function::<()>(script_path, "init", ()) {
            Ok(()) => {
                tracing::debug!(script = %script_path, "Action script reinitialized");
            }
            Err(e) => {
                let err_str = format!("{}", e);
                if !err_str.contains("not found") && !err_str.contains("Function not found") {
                    tracing::warn!(
                        script = %script_path,
                        error = %e,
                        "Failed to reinitialize action script"
                    );
                }
            }
        }
    }

    /// Reinitialize all action scripts after a full reload.
    ///
    /// This clears all action registrations and re-runs init() on all action scripts.
    pub fn reinitialize_all_action_scripts(self: &Arc<Self>) {
        // Clear all existing action handlers
        let old_count = self.action_registry.count();
        self.action_registry.clear();
        tracing::debug!(count = old_count, "Cleared action handlers for full reload");

        // Re-initialize all action scripts
        self.initialize_action_scripts();
    }

    /// Load all factions from database into memory.
    async fn load_factions(&self) -> anyhow::Result<()> {
        let factions = self.db.find_all_factions().await?;
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
        let sectors = self.db.find_all_sectors().await?;
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
                    if let Some(ref stance_str) = changes.combat_stance {
                        use bw_core::models::CombatStance;
                        ship.combat_stance = match stance_str.to_lowercase().as_str() {
                            "aggressive" => CombatStance::Aggressive,
                            "defensive" => CombatStance::Defensive,
                            "evasive" => CombatStance::Evasive,
                            _ => CombatStance::Balanced,
                        };
                    }
                    if let Some(ref locked) = changes.locked_target {
                        ship.locked_target = *locked;
                    }
                    if let Some(ref cargo_add) = changes.add_cargo {
                        ship.add_cargo(
                            cargo_add.cargo_type.clone(),
                            cargo_add.quantity,
                            cargo_add.purchase_price,
                        );
                    }
                    if let Some(ref cargo_remove) = changes.remove_cargo {
                        ship.remove_cargo(&cargo_remove.cargo_type, cargo_remove.quantity);
                    }
                    if let Some(ref upgrade_install) = changes.install_upgrade {
                        ship.install_upgrade(upgrade_install.upgrade_id.clone(), upgrade_install.slot.clone());
                    }
                    if let Some(ref slot) = changes.remove_upgrade_slot {
                        ship.remove_upgrade(slot);
                    }
                    // Persistence is automatic via TrackedDashMap dirty tracking
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
                    if let Some(credits) = changes.credits {
                        player.credits = credits.max(0);
                    }
                    if let Some(credits_delta) = changes.credits_delta {
                        player.credits = (player.credits + credits_delta).max(0);
                    }
                    // Persistence is automatic via TrackedDashMap dirty tracking
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
                            // Delete from database if player ship
                            if ship.is_player_ship {
                                self.persist.delete_ship(*entity_id);
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

            StateMutation::SendNotification { player_id, message, notification_type } => {
                // Queue notification for delivery via WebSocket
                // The actual sending happens in the game loop when processing mutations
                tracing::debug!(
                    player_id = %player_id,
                    notification_type = %notification_type,
                    message = %message,
                    "Script queued notification"
                );
                // For now, try to send immediately if player is connected
                if let Some(session) = self.players.get(&player_id) {
                    if let Some(ref conn) = session.connection {
                        let msg = ServerMessage::Notification {
                            message: message.clone(),
                            notification_type: notification_type.clone(),
                        };
                        let _ = conn.try_send(msg);
                    }
                }
                MutationResult::success(mutation)
            }

            StateMutation::SendChoice { player_id, choice_id, description, choices } => {
                tracing::debug!(
                    player_id = %player_id,
                    choice_id = %choice_id,
                    num_choices = choices.len(),
                    "Script queued choice dialog"
                );
                // For now, try to send immediately if player is connected
                if let Some(session) = self.players.get(&player_id) {
                    if let Some(ref conn) = session.connection {
                        let msg = ServerMessage::ChoiceRequired {
                            choice_id: choice_id.clone(),
                            description: description.clone(),
                            choices: choices.iter().map(|c| bw_shared::dto::ChoiceDto {
                                id: c.id.clone(),
                                text: c.text.clone(),
                                is_available: c.is_available,
                                requirement_text: c.requirement_text.clone(),
                            }).collect(),
                        };
                        let _ = conn.try_send(msg);
                    }
                }
                MutationResult::success(mutation)
            }

            StateMutation::BroadcastToSector { sector_id, message, notification_type } => {
                tracing::debug!(
                    sector_id = %sector_id,
                    notification_type = %notification_type,
                    message = %message,
                    "Script queued sector broadcast"
                );
                // Broadcast to all players in the sector
                if let Some(sector) = self.sectors.get(&sector_id) {
                    let msg = ServerMessage::Notification {
                        message: message.clone(),
                        notification_type: notification_type.clone(),
                    };
                    for conn in sector.connections.iter() {
                        let _ = conn.value().try_send(msg.clone());
                    }
                }
                MutationResult::success(mutation)
            }
        }
    }

    // === Playtest Forking ===

    /// Fork state from this GameState into a PlaytestInstance.
    ///
    /// This populates the playtest with copies of ships, sectors, and other
    /// mutable state according to the ForkConfig.
    pub fn fork_to_playtest(&self, playtest: &PlaytestInstance, config: &ForkConfig) {
        // Determine which sectors to fork
        let sector_ids: Vec<Uuid> = if config.sectors.is_empty() {
            self.sectors.iter().map(|s| s.sector.id).collect()
        } else {
            config.sectors.clone()
        };

        // Fork sectors
        for sector_id in &sector_ids {
            if let Some(live_sector) = self.sectors.get(sector_id) {
                // Create playtest sector instance
                let playtest_sector = PlaytestSectorInstance::new(live_sector.sector.clone());

                // Fork missions if configured
                if config.include_missions {
                    for mission in live_sector.missions.iter() {
                        playtest_sector.missions.insert(mission.id, mission.clone());
                    }
                }

                playtest.sectors.insert(*sector_id, playtest_sector);
            }
        }

        // Fork ships
        for ship_entry in self.ships.iter() {
            let ship = ship_entry.value();

            // Check if ship is in a forked sector
            if !sector_ids.contains(&ship.sector_id) {
                continue;
            }

            // Check specific ships filter first
            if !config.specific_ships.is_empty() {
                if !config.specific_ships.contains(&ship.id) {
                    continue;
                }
            } else if config.include_ships {
                // Apply NPC filter
                if !config.include_npcs && !ship.is_player_ship {
                    continue;
                }

                // Apply other players filter
                if !config.include_other_players && ship.is_player_ship {
                    // Only include owner's ship
                    if ship.owner_id != Some(playtest.owner_id) {
                        continue;
                    }
                }
            } else {
                // include_ships is false and not in specific_ships
                continue;
            }

            // Clone ship
            playtest.ships.insert(ship.id, ship.clone());

            // Add to sector
            if let Some(sector) = playtest.sectors.get(&ship.sector_id) {
                sector.ship_ids.insert(ship.id, ());
            }
        }

        // Fork player data for any player ships we included
        for ship in playtest.ships.iter() {
            if ship.is_player_ship {
                if let Some(owner_id) = ship.owner_id {
                    if !playtest.player_data.contains_key(&owner_id) {
                        if let Some(player) = self.player_data.get(&owner_id) {
                            playtest.player_data.insert(owner_id, player.clone());
                        }
                    }
                }
            }
        }

        // Fork squadrons that have members in the playtest
        let squadron_ids: std::collections::HashSet<Uuid> = playtest.player_data.iter()
            .filter_map(|p| p.squadron_id)
            .collect();
        for squadron_id in squadron_ids {
            if let Some(squadron) = self.squadrons.get(&squadron_id) {
                playtest.squadrons.insert(squadron_id, squadron.clone());
            }
        }

        tracing::info!(
            playtest_id = %playtest.id,
            sectors = playtest.sectors.len(),
            ships = playtest.ships.len(),
            players = playtest.player_data.len(),
            "Forked state to playtest"
        );
    }

    /// Get Arc-wrapped faction data for sharing with playtests.
    ///
    /// Note: This creates a new Arc with cloned data. For true sharing without
    /// cloning, the factions field would need to be Arc<DashMap<...>> in GameState.
    pub fn get_factions_arc(&self) -> Arc<DashMap<Uuid, Faction>> {
        let factions = Arc::new(DashMap::new());
        for entry in self.factions.iter() {
            factions.insert(*entry.key(), entry.value().clone());
        }
        factions
    }

    /// Get Arc-wrapped faction tag index for sharing with playtests.
    pub fn get_faction_tags_arc(&self) -> Arc<DashMap<String, Uuid>> {
        let tags = Arc::new(DashMap::new());
        for entry in self.faction_tags.iter() {
            tags.insert(entry.key().clone(), *entry.value());
        }
        tags
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
    pub combats: DashMap<Uuid, bw_game::systems::CombatEngagement>,

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
    /// Unique identifier for the current connection (used to prevent race conditions on reconnect)
    pub connection_id: Option<Uuid>,
    /// If Some, player is currently in a playtest instance
    pub playtest_id: Option<Uuid>,
}
