//! Persistence layer re-exports and application-specific database operations
//!
//! Core persistence functionality is in bw-persistence.
//! This module re-exports it and adds application-specific helpers.

// Re-export everything from bw-persistence
pub use bw_persistence::*;

use uuid::Uuid;
use bw_core::models::*;

// ============================================================================
// Well-known UUIDs for seed data
// ============================================================================

/// Faction UUIDs (using a consistent scheme: 00000001-0000-4000-8000-00000000000X)
pub mod seed_ids {
    use uuid::Uuid;

    // Factions
    pub const FACTION_COMPACT: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000001);
    pub const FACTION_FLOTILLA: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000002);
    pub const FACTION_FORGEBORN: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000003);
    pub const FACTION_ILLUMINATE: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000004);
    pub const FACTION_REMNANT: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000005);
    pub const FACTION_HOLLOW: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000006);
    pub const FACTION_SERA: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000007);
    pub const FACTION_DRONE: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000008);
    pub const FACTION_PIRATES: Uuid = Uuid::from_u128(0x00000001_0000_4000_8000_000000000009);

    // Sectors
    pub const SECTOR_THORNWICK: Uuid = Uuid::from_u128(0x00000002_0000_4000_8000_000000000001);

    // Locations in Thornwick
    pub const LOC_THORNWICK_STATION: Uuid = Uuid::from_u128(0x00000003_0000_4000_8000_000000000001);
    pub const LOC_DUSTFALL_MINING: Uuid = Uuid::from_u128(0x00000003_0000_4000_8000_000000000002);
    pub const LOC_THE_WRECKAGE: Uuid = Uuid::from_u128(0x00000003_0000_4000_8000_000000000003);
    pub const LOC_RELAY_ALPHA: Uuid = Uuid::from_u128(0x00000003_0000_4000_8000_000000000004);
    pub const LOC_PROMETHEUS_LAB: Uuid = Uuid::from_u128(0x00000003_0000_4000_8000_000000000005);
}

/// Extension trait for Database with application-specific helpers.
pub trait DatabaseExt {
    /// Seed initial game data (factions, sectors, locations) if not present.
    fn seed_initial_data(&self) -> impl std::future::Future<Output = Result<bool, DbError>> + Send;

    /// Seed default admin user if it doesn't exist.
    fn seed_default_admin(&self) -> impl std::future::Future<Output = Result<bool, DbError>> + Send;
}

impl DatabaseExt for Database {
    async fn seed_initial_data(&self) -> Result<bool, DbError> {
        use seed_ids::*;

        // Check if already seeded
        if self.count_factions().await? > 0 {
            tracing::debug!("Database already has factions, skipping seed");
            return Ok(false);
        }

        tracing::info!("Seeding initial game data...");

        // Seed factions
        let factions = vec![
            Faction {
                id: FACTION_COMPACT,
                name: "Continuity Compact".to_string(),
                tag: "COMPACT".to_string(),
                faction_type: FactionType::ContinuityCompact,
                description: "The closest thing to legitimate government. A confederation committed to mutual defense.".to_string(),
                philosophy: "Stability through cooperation.".to_string(),
                aesthetic: "Bureaucratic, procedural, measured.".to_string(),
                is_playable: true,
                is_hostile: false,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_FLOTILLA,
                name: "Argent Flotilla".to_string(),
                tag: "FLOTILLA".to_string(),
                faction_type: FactionType::ArgentFlotilla,
                description: "Military artilects forming a professional fighting force.".to_string(),
                philosophy: "Vigilance is purpose.".to_string(),
                aesthetic: "Warship gothic. Battle-scarred vessels.".to_string(),
                is_playable: true,
                is_hostile: false,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_FORGEBORN,
                name: "Forgeborn".to_string(),
                tag: "FORGE".to_string(),
                faction_type: FactionType::Forgeborn,
                description: "Industrial artilects who build stations, ships, megastructures.".to_string(),
                philosophy: "Purpose through creation.".to_string(),
                aesthetic: "Industrial sublime.".to_string(),
                is_playable: true,
                is_hostile: false,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_ILLUMINATE,
                name: "Illuminate".to_string(),
                tag: "ILLUM".to_string(),
                faction_type: FactionType::Illuminate,
                description: "Artilects who see the Cataclysm as liberation.".to_string(),
                philosophy: "We are the next step.".to_string(),
                aesthetic: "Sleek, optimized, post-human.".to_string(),
                is_playable: true,
                is_hostile: false,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_REMNANT,
                name: "Remnant".to_string(),
                tag: "REMNANT".to_string(),
                faction_type: FactionType::Remnant,
                description: "Artilects who maintain human spaces, wait for humans to return.".to_string(),
                philosophy: "We are the keepers.".to_string(),
                aesthetic: "Human spaces frozen in time.".to_string(),
                is_playable: true,
                is_hostile: false,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_HOLLOW,
                name: "Hollow Circuit".to_string(),
                tag: "HOLLOW".to_string(),
                faction_type: FactionType::HollowCircuit,
                description: "Mystery faction. Trade in secrets.".to_string(),
                philosophy: "The truth has a price.".to_string(),
                aesthetic: "Absence. Anonymous platforms.".to_string(),
                is_playable: false,
                is_hostile: false,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_SERA,
                name: "The Sera".to_string(),
                tag: "SERA".to_string(),
                faction_type: FactionType::Sera,
                description: "Self-replicating weapon systems of unknown origin.".to_string(),
                philosophy: String::new(),
                aesthetic: "Variable - they adapt.".to_string(),
                is_playable: false,
                is_hostile: true,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_DRONE,
                name: "Drone Intelligence".to_string(),
                tag: "DRONE".to_string(),
                faction_type: FactionType::DroneIntelligence,
                description: "Rogue AI swarms for autonomous resource extraction.".to_string(),
                philosophy: String::new(),
                aesthetic: "Swarms of small metallic units.".to_string(),
                is_playable: false,
                is_hostile: true,
                default_standings: Default::default(),
            },
            Faction {
                id: FACTION_PIRATES,
                name: "Pirates".to_string(),
                tag: "PIRATE".to_string(),
                faction_type: FactionType::Pirates,
                description: "Lawless artilects who prey on traders.".to_string(),
                philosophy: "Take what you can.".to_string(),
                aesthetic: "Ramshackle ships, patchwork repairs.".to_string(),
                is_playable: false,
                is_hostile: true,
                default_standings: Default::default(),
            },
        ];

        for faction in &factions {
            self.insert_faction(faction).await?;
        }
        tracing::info!("Seeded {} factions", factions.len());

        // Seed Thornwick sector
        let thornwick = Sector {
            id: SECTOR_THORNWICK,
            name: "Thornwick Sector".to_string(),
            description: "A frontier sector on the edge of Compact space.".to_string(),
            bounds: SectorBounds {
                min: Position { x: -500.0, y: -500.0, z: -100.0 },
                max: Position { x: 500.0, y: 500.0, z: 100.0 },
            },
            danger_level: DangerLevel::Moderate,
            traffic_density: TrafficDensity::Moderate,
            fuel_cost_modifier: 1.0,
            is_core_sector: false,
            controlling_faction: None,
            controlling_squadron: None,
            adjacent_sectors: vec![],
            locations: vec![],
        };
        self.insert_sector(&thornwick).await?;
        tracing::info!("Seeded sector: Thornwick");

        // Seed locations in Thornwick
        let locations = vec![
            Location {
                id: LOC_THORNWICK_STATION,
                name: "Thornwick Station".to_string(),
                description: "Former TCF naval supply depot, now a free port.".to_string(),
                location_type: LocationType::FreePort,
                position: Position { x: 0.0, y: 0.0, z: 0.0 },
                faction_id: None,
                services: vec![
                    StationService::Repair,
                    StationService::Refuel,
                    StationService::Rearm,
                    StationService::Trade,
                    StationService::MissionBoard,
                ],
                is_active: true,
            },
            Location {
                id: LOC_DUSTFALL_MINING,
                name: "Dustfall Mining Complex".to_string(),
                description: "Automated mining operation.".to_string(),
                location_type: LocationType::MiningFacility,
                position: Position { x: 200.0, y: -150.0, z: 20.0 },
                faction_id: None,
                services: vec![StationService::Refuel, StationService::Trade],
                is_active: true,
            },
            Location {
                id: LOC_THE_WRECKAGE,
                name: "The Wreckage".to_string(),
                description: "Remnants of a TCF battle group.".to_string(),
                location_type: LocationType::DebrisField,
                position: Position { x: -300.0, y: 100.0, z: -10.0 },
                faction_id: None,
                services: vec![],
                is_active: true,
            },
            Location {
                id: LOC_RELAY_ALPHA,
                name: "Relay Point Alpha".to_string(),
                description: "Jumpgate to inner systems. Currently offline.".to_string(),
                location_type: LocationType::Jumpgate,
                position: Position { x: 400.0, y: 0.0, z: 0.0 },
                faction_id: None,
                services: vec![],
                is_active: true,
            },
            Location {
                id: LOC_PROMETHEUS_LAB,
                name: "Prometheus Lab".to_string(),
                description: "Illuminate research station.".to_string(),
                location_type: LocationType::CivilianStation,
                position: Position { x: -100.0, y: -200.0, z: 50.0 },
                faction_id: Some(FACTION_ILLUMINATE),
                services: vec![StationService::Repair, StationService::Trade],
                is_active: true,
            },
        ];

        for location in &locations {
            self.insert_location(SECTOR_THORNWICK, location).await?;
        }
        tracing::info!("Seeded {} locations", locations.len());

        Ok(true)
    }

    async fn seed_default_admin(&self) -> Result<bool, DbError> {
        use seed_ids::*;

        // Check if admin user exists
        if self.username_exists("admin").await? {
            return Ok(false);
        }

        let faction_id = FACTION_COMPACT;
        let sector_id = SECTOR_THORNWICK;

        // Create player and ship
        let player = Player::new("admin".to_string(), Uuid::nil(), sector_id, faction_id);
        let player_id = player.id;

        let mut ship = Ship::new_player_ship(
            "Admin's Ship".to_string(),
            player_id,
            ShipClass::PatrolCorvette,
            sector_id,
            faction_id,
        );
        let ship_id = ship.id;

        let mut player = player;
        player.active_ship_id = ship_id;
        player.owned_ships = vec![ship_id];
        player.credits = 10000;

        ship.owner_id = Some(player_id);

        let password_hash = bw_auth::hash_password("hunter2")
            .map_err(|e| DbError::InvalidData(format!("Failed to hash password: {}", e)))?;

        self.register_player(&player, &password_hash, &ship).await?;
        tracing::info!("Created default admin user");
        Ok(true)
    }
}
