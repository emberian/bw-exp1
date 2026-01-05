//! Ship model - Player and NPC vessels
//!
//! Ships are the primary game entity. For player artilects, the ship IS the player.
//! Hull damage is felt as pain. System failures are experienced as impairment.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CrewResources, ShipResources};

/// A ship in the game world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    /// Unique identifier
    pub id: Uuid,

    /// Ship name (chosen by player or generated for NPCs)
    pub name: String,

    /// Owner (player ID for player ships, None for unowned NPCs)
    pub owner_id: Option<Uuid>,

    /// Ship class determines base stats and capabilities
    pub ship_class: ShipClass,

    /// Current sector
    pub sector_id: Uuid,

    /// Position within sector
    pub position: Position,

    /// Ship resources (ammunition, fuel)
    pub resources: ShipResources,

    /// Crew resources (morale, experience)
    pub crew: CrewResources,

    /// Hull integrity (0.0 - 100.0)
    pub hull_integrity: f32,

    /// Shield strength (0.0 - 100.0)
    pub shield_strength: f32,

    /// Weapon mounts
    pub weapons: Vec<WeaponMount>,

    /// Current status
    pub status: ShipStatus,

    /// Whether this is a player-controlled ship
    pub is_player_ship: bool,

    /// Faction affiliation
    pub faction_id: Option<Uuid>,

    /// Squadron membership (for player ships)
    pub squadron_id: Option<Uuid>,
}

impl Ship {
    /// Create a new player ship with starting equipment.
    pub fn new_player_ship(
        name: String,
        owner_id: Uuid,
        ship_class: ShipClass,
        sector_id: Uuid,
        faction_id: Uuid,
    ) -> Self {
        let base_stats = ship_class.base_stats();

        Self {
            id: Uuid::new_v4(),
            name,
            owner_id: Some(owner_id),
            ship_class,
            sector_id,
            position: Position::default(),
            resources: ShipResources::new(),
            crew: CrewResources::green_crew(),
            hull_integrity: 100.0,
            shield_strength: base_stats.shield_capacity,
            weapons: ship_class.default_weapons(),
            status: ShipStatus::Idle,
            is_player_ship: true,
            faction_id: Some(faction_id),
            squadron_id: None,
        }
    }

    /// Create an NPC ship.
    pub fn new_npc_ship(
        name: String,
        ship_class: ShipClass,
        sector_id: Uuid,
        position: Position,
        faction_id: Option<Uuid>,
    ) -> Self {
        let base_stats = ship_class.base_stats();

        Self {
            id: Uuid::new_v4(),
            name,
            owner_id: None,
            ship_class,
            sector_id,
            position,
            resources: ShipResources::new(),
            crew: CrewResources::new(),
            hull_integrity: 100.0,
            shield_strength: base_stats.shield_capacity,
            weapons: ship_class.default_weapons(),
            status: ShipStatus::Idle,
            is_player_ship: false,
            faction_id,
            squadron_id: None,
        }
    }

    /// Get total combat effectiveness considering all modifiers.
    pub fn combat_effectiveness(&self) -> CombatStats {
        let base = self.ship_class.base_stats();
        let ammo_mod = self.resources.ammo_attack_modifier();
        let morale_mod = 1.0 + self.crew.morale_combat_modifier();
        let xp_mods = self.crew.experience_modifiers();
        let hull_mod = self.hull_integrity / 100.0;

        CombatStats {
            attack: base.attack * ammo_mod * morale_mod * xp_mods.attack * hull_mod,
            defense: base.defense * morale_mod * xp_mods.defense * hull_mod,
            speed: base.speed * self.resources.fuel_movement_modifier() * xp_mods.speed,
            sensor_range: base.sensor_range,
        }
    }

    /// Check if ship can attack.
    pub fn can_attack(&self) -> bool {
        !self.resources.is_ammo_depleted()
            && self.hull_integrity > 0.0
            && !matches!(self.status, ShipStatus::Disabled | ShipStatus::Destroyed)
    }

    /// Check if ship can move.
    pub fn can_move(&self) -> bool {
        self.resources.fuel > 0.0
            && !matches!(
                self.status,
                ShipStatus::Disabled | ShipStatus::Destroyed | ShipStatus::Docked { .. }
            )
    }

    /// Apply damage to the ship.
    pub fn apply_damage(&mut self, damage: f32) {
        // Shields absorb damage first
        if self.shield_strength > 0.0 {
            let absorbed = damage.min(self.shield_strength);
            self.shield_strength -= absorbed;
            let remaining = damage - absorbed;
            if remaining > 0.0 {
                self.hull_integrity = (self.hull_integrity - remaining).max(0.0);
            }
        } else {
            self.hull_integrity = (self.hull_integrity - damage).max(0.0);
        }

        // Check for destruction
        if self.hull_integrity <= 0.0 {
            self.status = ShipStatus::Destroyed;
        } else if self.hull_integrity < 25.0 {
            self.status = ShipStatus::Disabled;
        }
    }

    /// Repair the ship (at a station).
    pub fn repair(&mut self, hull_amount: f32, shield_amount: f32) {
        let base_stats = self.ship_class.base_stats();
        self.hull_integrity = (self.hull_integrity + hull_amount).min(100.0);
        self.shield_strength = (self.shield_strength + shield_amount).min(base_stats.shield_capacity);

        if self.hull_integrity >= 25.0 && matches!(self.status, ShipStatus::Disabled) {
            self.status = ShipStatus::Idle;
        }
    }
}

/// Ship class determines base capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipClass {
    // Player classes
    PatrolCorvette,
    Frigate,
    Destroyer,
    Cruiser,
    Carrier,

    // NPC civilian classes
    Freighter,
    Transport,
    MiningVessel,

    // NPC hostile classes
    PirateRaider,
    PirateFrigate,
    TerroristBomber,

    // Sera (enemy faction)
    SeraSwarm,
    SeraHunter,

    // Drone Intelligence
    DroneHarvester,
    DroneSwarm,
}

impl ShipClass {
    /// Get base stats for this ship class.
    pub fn base_stats(&self) -> ShipBaseStats {
        match self {
            // Player ships - balanced for patrol duty
            Self::PatrolCorvette => ShipBaseStats {
                attack: 30.0,
                defense: 20.0,
                speed: 80.0,
                shield_capacity: 50.0,
                sensor_range: 100.0,
                cargo_capacity: 50,
                fuel_per_sector: 5.0,
                ammo_per_attack: 2.0,
            },
            Self::Frigate => ShipBaseStats {
                attack: 50.0,
                defense: 40.0,
                speed: 60.0,
                shield_capacity: 80.0,
                sensor_range: 120.0,
                cargo_capacity: 100,
                fuel_per_sector: 8.0,
                ammo_per_attack: 3.0,
            },
            Self::Destroyer => ShipBaseStats {
                attack: 80.0,
                defense: 60.0,
                speed: 50.0,
                shield_capacity: 100.0,
                sensor_range: 150.0,
                cargo_capacity: 150,
                fuel_per_sector: 12.0,
                ammo_per_attack: 5.0,
            },
            Self::Cruiser => ShipBaseStats {
                attack: 100.0,
                defense: 80.0,
                speed: 40.0,
                shield_capacity: 150.0,
                sensor_range: 200.0,
                cargo_capacity: 300,
                fuel_per_sector: 15.0,
                ammo_per_attack: 8.0,
            },
            Self::Carrier => ShipBaseStats {
                attack: 40.0, // Relies on fighters
                defense: 100.0,
                speed: 30.0,
                shield_capacity: 200.0,
                sensor_range: 300.0,
                cargo_capacity: 500,
                fuel_per_sector: 20.0,
                ammo_per_attack: 10.0,
            },

            // Civilian ships - weak but valuable
            Self::Freighter => ShipBaseStats {
                attack: 5.0,
                defense: 15.0,
                speed: 40.0,
                shield_capacity: 20.0,
                sensor_range: 50.0,
                cargo_capacity: 1000,
                fuel_per_sector: 10.0,
                ammo_per_attack: 1.0,
            },
            Self::Transport => ShipBaseStats {
                attack: 10.0,
                defense: 20.0,
                speed: 50.0,
                shield_capacity: 30.0,
                sensor_range: 60.0,
                cargo_capacity: 500,
                fuel_per_sector: 8.0,
                ammo_per_attack: 1.0,
            },
            Self::MiningVessel => ShipBaseStats {
                attack: 5.0,
                defense: 25.0,
                speed: 30.0,
                shield_capacity: 40.0,
                sensor_range: 40.0,
                cargo_capacity: 800,
                fuel_per_sector: 12.0,
                ammo_per_attack: 1.0,
            },

            // Pirates - fast and aggressive
            Self::PirateRaider => ShipBaseStats {
                attack: 40.0,
                defense: 15.0,
                speed: 90.0,
                shield_capacity: 30.0,
                sensor_range: 80.0,
                cargo_capacity: 100,
                fuel_per_sector: 4.0,
                ammo_per_attack: 2.0,
            },
            Self::PirateFrigate => ShipBaseStats {
                attack: 60.0,
                defense: 35.0,
                speed: 55.0,
                shield_capacity: 60.0,
                sensor_range: 100.0,
                cargo_capacity: 200,
                fuel_per_sector: 7.0,
                ammo_per_attack: 4.0,
            },
            Self::TerroristBomber => ShipBaseStats {
                attack: 150.0, // High damage, suicide attacks
                defense: 10.0,
                speed: 70.0,
                shield_capacity: 10.0,
                sensor_range: 50.0,
                cargo_capacity: 50, // Bomb payload
                fuel_per_sector: 3.0,
                ammo_per_attack: 50.0, // One big attack
            },

            // Sera - alien threat, very dangerous
            Self::SeraSwarm => ShipBaseStats {
                attack: 20.0, // Many small attacks
                defense: 5.0,
                speed: 100.0,
                shield_capacity: 0.0, // No shields
                sensor_range: 150.0,
                cargo_capacity: 0,
                fuel_per_sector: 1.0,
                ammo_per_attack: 0.5,
            },
            Self::SeraHunter => ShipBaseStats {
                attack: 120.0,
                defense: 70.0,
                speed: 60.0,
                shield_capacity: 50.0,
                sensor_range: 250.0,
                cargo_capacity: 0,
                fuel_per_sector: 5.0,
                ammo_per_attack: 5.0,
            },

            // Drones - relentless harvesters
            Self::DroneHarvester => ShipBaseStats {
                attack: 30.0,
                defense: 50.0,
                speed: 35.0,
                shield_capacity: 30.0,
                sensor_range: 100.0,
                cargo_capacity: 2000,
                fuel_per_sector: 2.0,
                ammo_per_attack: 1.0,
            },
            Self::DroneSwarm => ShipBaseStats {
                attack: 15.0,
                defense: 10.0,
                speed: 80.0,
                shield_capacity: 0.0,
                sensor_range: 80.0,
                cargo_capacity: 0,
                fuel_per_sector: 0.5,
                ammo_per_attack: 0.2,
            },
        }
    }

    /// Get default weapons for this ship class.
    pub fn default_weapons(&self) -> Vec<WeaponMount> {
        match self {
            Self::PatrolCorvette => vec![
                WeaponMount::new(WeaponType::Railgun, 20.0, 0.7, 1.0),
                WeaponMount::new(WeaponType::PointDefense, 5.0, 0.9, 0.5),
            ],
            Self::Frigate => vec![
                WeaponMount::new(WeaponType::Railgun, 25.0, 0.75, 1.5),
                WeaponMount::new(WeaponType::MissileLauncher, 40.0, 0.6, 3.0),
                WeaponMount::new(WeaponType::PointDefense, 8.0, 0.9, 0.5),
            ],
            Self::Destroyer => vec![
                WeaponMount::new(WeaponType::Railgun, 35.0, 0.8, 2.0),
                WeaponMount::new(WeaponType::Railgun, 35.0, 0.8, 2.0),
                WeaponMount::new(WeaponType::MissileLauncher, 60.0, 0.65, 4.0),
                WeaponMount::new(WeaponType::PointDefense, 10.0, 0.9, 0.5),
            ],
            Self::Cruiser => vec![
                WeaponMount::new(WeaponType::LaserBattery, 50.0, 0.85, 3.0),
                WeaponMount::new(WeaponType::Railgun, 40.0, 0.8, 2.5),
                WeaponMount::new(WeaponType::MissileLauncher, 80.0, 0.7, 5.0),
                WeaponMount::new(WeaponType::PointDefense, 15.0, 0.95, 0.5),
            ],
            _ => vec![WeaponMount::new(WeaponType::Railgun, 15.0, 0.6, 1.0)],
        }
    }

    /// Whether this is a player-usable ship class.
    pub fn is_player_class(&self) -> bool {
        matches!(
            self,
            Self::PatrolCorvette
                | Self::Frigate
                | Self::Destroyer
                | Self::Cruiser
                | Self::Carrier
        )
    }

    /// Whether this is a hostile NPC class.
    pub fn is_hostile(&self) -> bool {
        matches!(
            self,
            Self::PirateRaider
                | Self::PirateFrigate
                | Self::TerroristBomber
                | Self::SeraSwarm
                | Self::SeraHunter
                | Self::DroneHarvester
                | Self::DroneSwarm
        )
    }
}

/// Base stats for a ship class.
#[derive(Debug, Clone, Copy)]
pub struct ShipBaseStats {
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub shield_capacity: f32,
    pub sensor_range: f32,
    pub cargo_capacity: u32,
    pub fuel_per_sector: f32,
    pub ammo_per_attack: f32,
}

/// Calculated combat stats after all modifiers.
#[derive(Debug, Clone, Copy)]
pub struct CombatStats {
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub sensor_range: f32,
}

/// Position in 3D space within a sector.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Calculate distance to another position.
    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Move towards a target position by a given amount.
    pub fn move_towards(&mut self, target: &Position, distance: f64) {
        let current_distance = self.distance_to(target);
        if current_distance <= distance {
            *self = *target;
        } else {
            let ratio = distance / current_distance;
            self.x += (target.x - self.x) * ratio;
            self.y += (target.y - self.y) * ratio;
            self.z += (target.z - self.z) * ratio;
        }
    }
}

/// Current ship status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShipStatus {
    /// Not doing anything specific
    Idle,

    /// Moving to a destination
    InTransit {
        destination: Position,
        target_id: Option<Uuid>, // Station/location ID if moving to a specific place
    },

    /// Engaged in combat
    InCombat { engagement_id: Uuid },

    /// Docked at a station
    Docked { station_id: Uuid },

    /// Hull critically damaged, cannot function
    Disabled,

    /// Ship destroyed
    Destroyed,
}

/// A weapon mounted on a ship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponMount {
    pub weapon_type: WeaponType,
    pub damage_base: f32,
    pub accuracy_base: f32,
    pub ammo_cost: f32,
}

impl WeaponMount {
    pub fn new(weapon_type: WeaponType, damage: f32, accuracy: f32, ammo_cost: f32) -> Self {
        Self {
            weapon_type,
            damage_base: damage,
            accuracy_base: accuracy,
            ammo_cost,
        }
    }
}

/// Weapon types with different characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponType {
    /// High velocity kinetic weapon. Accurate, moderate damage.
    Railgun,

    /// Guided explosive. High damage, lower accuracy, can be shot down.
    MissileLauncher,

    /// Energy weapon. Consistent damage, no ammo issues.
    LaserBattery,

    /// Anti-missile/fighter system. Low damage, very high accuracy.
    PointDefense,
}
