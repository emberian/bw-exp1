//! Server state management

use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::broadcast;
use uuid::Uuid;

use bw_core::models::*;
use bw_scripting::ScriptEngine;
use bw_shared::ServerMessage;

/// Global game state.
pub struct GameState {
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

    /// Squadrons (squadron_id -> Squadron)
    pub squadrons: DashMap<Uuid, Squadron>,

    /// Broadcast channel for server-wide messages
    pub broadcaster: broadcast::Sender<ServerMessage>,

    /// Current server tick
    pub tick: std::sync::atomic::AtomicU64,
}

impl GameState {
    pub async fn new() -> anyhow::Result<Self> {
        let scripts = Arc::new(ScriptEngine::new("scripts"));
        let (broadcaster, _) = broadcast::channel(1000);

        let state = Self {
            scripts,
            sectors: DashMap::new(),
            player_data: DashMap::new(),
            players: DashMap::new(),
            ships: DashMap::new(),
            factions: DashMap::new(),
            squadrons: DashMap::new(),
            broadcaster,
            tick: std::sync::atomic::AtomicU64::new(0),
        };

        // Initialize default factions
        state.init_factions();

        // Initialize starting sector
        state.init_starting_sector();

        Ok(state)
    }

    fn init_factions(&self) {
        let factions = vec![
            Faction::continuity_compact(),
            Faction::argent_flotilla(),
            Faction::forgeborn(),
            Faction::illuminate(),
            Faction::remnant(),
            Faction::hollow_circuit(),
            Faction::sera(),
            Faction::drone_intelligence(),
            Faction::pirates(),
        ];

        for faction in factions {
            self.factions.insert(faction.id, faction);
        }
    }

    fn init_starting_sector(&self) {
        let mut sector = Sector::new("Thornwick Sector".to_string(), DangerLevel::Moderate);
        sector.description = "A frontier sector on the edge of Compact space. Home to Thornwick Station, the largest free port in the region.".to_string();
        sector.is_core_sector = false;

        // Add Thornwick Station
        let mut station = Location::new(
            "Thornwick Station".to_string(),
            LocationType::FreePort,
            Position::new(0.0, 0.0, 0.0),
        );
        station.description = "Former TCF naval supply depot, now a free port. The largest station in sector.".to_string();
        sector.locations.push(station);

        // Add a mining facility
        let mining = Location::new(
            "Dustfall Mining Complex".to_string(),
            LocationType::MiningFacility,
            Position::new(200.0, -150.0, 20.0),
        );
        sector.locations.push(mining);

        // Add a debris field (mission hook)
        let debris = Location::new(
            "The Wreckage".to_string(),
            LocationType::DebrisField,
            Position::new(-300.0, 100.0, -10.0),
        );
        sector.locations.push(debris);

        // Add jumpgate
        let mut jumpgate = Location::new(
            "Relay Point Alpha".to_string(),
            LocationType::Jumpgate,
            Position::new(400.0, 0.0, 0.0),
        );
        jumpgate.description = "Jumpgate connection to the inner systems.".to_string();
        sector.locations.push(jumpgate);

        // Create sector instance
        let instance = SectorInstance::new(sector);

        self.sectors.insert(instance.sector.id, instance);
    }

    pub fn get_tick(&self) -> u64 {
        self.tick.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn increment_tick(&self) -> u64 {
        self.tick.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
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
