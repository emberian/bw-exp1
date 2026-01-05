//! State mutations for scripts
//!
//! Scripts queue mutations which are validated and applied atomically
//! after script execution completes. This ensures state consistency.

use rhai::{Dynamic, Map};
use uuid::Uuid;

use bw_core::models::{Position, ShipClass, ShipStatus};

/// A pending state mutation queued by a script.
#[derive(Debug, Clone)]
pub enum StateMutation {
    /// Modify a ship's properties
    ModifyShip {
        ship_id: Uuid,
        changes: ShipChanges,
    },

    /// Modify a player's properties
    ModifyPlayer {
        player_id: Uuid,
        changes: PlayerChanges,
    },

    /// Spawn a new NPC ship
    SpawnShip {
        config: ShipSpawnConfig,
    },

    /// Mark an entity for destruction
    DestroyEntity {
        entity_id: Uuid,
        entity_type: EntityType,
    },

    /// Emit a game event
    EmitEvent {
        event_type: String,
        data: Map,
        actor_id: Option<Uuid>,
        target_id: Option<Uuid>,
    },

    /// Send a notification to a player
    SendNotification {
        player_id: Uuid,
        message: String,
        notification_type: String, // "info", "warning", "error", "success"
    },

    /// Send a choice dialog to a player
    SendChoice {
        player_id: Uuid,
        choice_id: String,
        description: String,
        choices: Vec<ChoiceOption>,
    },

    /// Broadcast a message to all players in a sector
    BroadcastToSector {
        sector_id: Uuid,
        message: String,
        notification_type: String,
    },
}

/// A choice option for player dialogs.
#[derive(Debug, Clone)]
pub struct ChoiceOption {
    pub id: String,
    pub text: String,
    pub is_available: bool,
    pub requirement_text: Option<String>,
}

// EntityType is defined in behaviors module
pub use crate::behaviors::EntityType;

/// Changes to apply to a ship.
#[derive(Debug, Clone, Default)]
pub struct ShipChanges {
    pub hull: Option<f32>,
    pub shields: Option<f32>,
    pub ammunition: Option<f32>,
    pub fuel: Option<f32>,
    pub morale: Option<f32>,
    pub experience: Option<i32>,
    pub position: Option<Position>,
    pub status: Option<ShipStatusChange>,
    pub combat_stance: Option<String>,
    pub locked_target: Option<Option<Uuid>>, // Some(None) to clear, Some(Some(id)) to set
    pub add_cargo: Option<CargoChange>,
    pub remove_cargo: Option<CargoChange>,
    pub install_upgrade: Option<UpgradeInstall>,
    pub remove_upgrade_slot: Option<String>,
}

/// Upgrade installation request.
#[derive(Debug, Clone)]
pub struct UpgradeInstall {
    pub upgrade_id: String,
    pub slot: String,
}

/// Cargo change for add/remove operations.
#[derive(Debug, Clone)]
pub struct CargoChange {
    pub cargo_type: String,
    pub quantity: u32,
    pub purchase_price: i64,
}

impl ShipChanges {
    /// Parse from Rhai Dynamic map.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;
        let mut changes = Self::default();

        if let Some(v) = map.get("hull") {
            changes.hull = v.as_float().ok().map(|f| f as f32);
        }
        if let Some(v) = map.get("shields") {
            changes.shields = v.as_float().ok().map(|f| f as f32);
        }
        if let Some(v) = map.get("ammunition") {
            changes.ammunition = v.as_float().ok().map(|f| f as f32);
        }
        if let Some(v) = map.get("fuel") {
            changes.fuel = v.as_float().ok().map(|f| f as f32);
        }
        if let Some(v) = map.get("morale") {
            changes.morale = v.as_float().ok().map(|f| f as f32);
        }
        if let Some(v) = map.get("experience") {
            changes.experience = v.as_int().ok().map(|i| i as i32);
        }
        if let Some(v) = map.get("position") {
            if let Some(pos_map) = v.clone().try_cast::<Map>() {
                let x = pos_map.get("x").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
                let y = pos_map.get("y").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
                let z = pos_map.get("z").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
                changes.position = Some(Position::new(x, y, z));
            }
        }
        if let Some(v) = map.get("status") {
            if let Ok(status_str) = v.clone().into_string() {
                changes.status = ShipStatusChange::from_string(&status_str);
            }
        }
        if let Some(v) = map.get("combat_stance") {
            changes.combat_stance = v.clone().into_string().ok();
        }
        if let Some(v) = map.get("locked_target") {
            if v.is_unit() {
                changes.locked_target = Some(None); // Clear target
            } else if let Ok(target_str) = v.clone().into_string() {
                if target_str.is_empty() {
                    changes.locked_target = Some(None);
                } else if let Ok(uuid) = Uuid::parse_str(&target_str) {
                    changes.locked_target = Some(Some(uuid));
                }
            }
        }
        if let Some(v) = map.get("add_cargo") {
            if let Some(cargo_map) = v.clone().try_cast::<Map>() {
                let cargo_type = cargo_map.get("type")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_default();
                let quantity = cargo_map.get("quantity")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0) as u32;
                let purchase_price = cargo_map.get("price")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0);
                if !cargo_type.is_empty() && quantity > 0 {
                    changes.add_cargo = Some(CargoChange {
                        cargo_type,
                        quantity,
                        purchase_price,
                    });
                }
            }
        }
        if let Some(v) = map.get("remove_cargo") {
            if let Some(cargo_map) = v.clone().try_cast::<Map>() {
                let cargo_type = cargo_map.get("type")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_default();
                let quantity = cargo_map.get("quantity")
                    .and_then(|v| v.as_int().ok())
                    .unwrap_or(0) as u32;
                if !cargo_type.is_empty() && quantity > 0 {
                    changes.remove_cargo = Some(CargoChange {
                        cargo_type,
                        quantity,
                        purchase_price: 0,
                    });
                }
            }
        }

        Some(changes)
    }

    /// Check if there are any changes.
    pub fn is_empty(&self) -> bool {
        self.hull.is_none()
            && self.shields.is_none()
            && self.ammunition.is_none()
            && self.fuel.is_none()
            && self.morale.is_none()
            && self.experience.is_none()
            && self.position.is_none()
            && self.status.is_none()
            && self.combat_stance.is_none()
            && self.locked_target.is_none()
            && self.add_cargo.is_none()
            && self.remove_cargo.is_none()
            && self.install_upgrade.is_none()
            && self.remove_upgrade_slot.is_none()
    }
}

/// Ship status change request.
#[derive(Debug, Clone)]
pub enum ShipStatusChange {
    Idle,
    InTransit { destination: Position, target_id: Option<Uuid> },
    Disabled,
}

impl ShipStatusChange {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "idle" => Some(Self::Idle),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    pub fn to_ship_status(&self) -> ShipStatus {
        match self {
            Self::Idle => ShipStatus::Idle,
            Self::InTransit { destination, target_id } => ShipStatus::InTransit {
                destination: *destination,
                target_id: *target_id,
            },
            Self::Disabled => ShipStatus::Disabled,
        }
    }
}

/// Changes to apply to a player.
#[derive(Debug, Clone, Default)]
pub struct PlayerChanges {
    pub reputation: Option<i32>,
    pub fame: Option<i32>,
    pub reputation_delta: Option<i32>,
    pub fame_delta: Option<i32>,
    pub credits: Option<i64>,
    pub credits_delta: Option<i64>,
}

impl PlayerChanges {
    /// Parse from Rhai Dynamic map.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;
        let mut changes = Self::default();

        if let Some(v) = map.get("reputation") {
            changes.reputation = v.as_int().ok().map(|i| i as i32);
        }
        if let Some(v) = map.get("fame") {
            changes.fame = v.as_int().ok().map(|i| i as i32);
        }
        if let Some(v) = map.get("reputation_delta") {
            changes.reputation_delta = v.as_int().ok().map(|i| i as i32);
        }
        if let Some(v) = map.get("fame_delta") {
            changes.fame_delta = v.as_int().ok().map(|i| i as i32);
        }
        if let Some(v) = map.get("credits") {
            changes.credits = v.as_int().ok();
        }
        if let Some(v) = map.get("credits_delta") {
            changes.credits_delta = v.as_int().ok();
        }

        Some(changes)
    }

    pub fn is_empty(&self) -> bool {
        self.reputation.is_none()
            && self.fame.is_none()
            && self.reputation_delta.is_none()
            && self.fame_delta.is_none()
            && self.credits.is_none()
            && self.credits_delta.is_none()
    }
}

/// Configuration for spawning a new ship.
#[derive(Debug, Clone)]
pub struct ShipSpawnConfig {
    pub name: String,
    pub ship_class: ShipClass,
    pub sector_id: Uuid,
    pub position: Position,
    pub faction_id: Option<Uuid>,
    pub behavior_script: Option<String>,
}

impl ShipSpawnConfig {
    /// Parse from Rhai Dynamic map.
    pub fn from_dynamic(value: Dynamic, default_sector_id: Uuid) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let name = map.get("name")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_else(|| "Unknown".to_string());

        let ship_class = map.get("ship_class")
            .and_then(|v| v.clone().into_string().ok())
            .and_then(|s| parse_ship_class(&s))
            .unwrap_or(ShipClass::PirateRaider);

        let sector_id = map.get("sector_id")
            .and_then(|v| v.clone().into_string().ok())
            .and_then(|s| Uuid::parse_str(&s).ok())
            .unwrap_or(default_sector_id);

        let position = map.get("position")
            .and_then(|v| v.clone().try_cast::<Map>())
            .map(|pos_map| {
                let x = pos_map.get("x").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
                let y = pos_map.get("y").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
                let z = pos_map.get("z").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
                Position::new(x, y, z)
            })
            .unwrap_or_default();

        let faction_id = map.get("faction_id")
            .and_then(|v| v.clone().into_string().ok())
            .and_then(|s| Uuid::parse_str(&s).ok());

        let behavior_script = map.get("behavior_script")
            .and_then(|v| v.clone().into_string().ok());

        Some(Self {
            name,
            ship_class,
            sector_id,
            position,
            faction_id,
            behavior_script,
        })
    }
}

/// Result of applying a mutation.
#[derive(Debug, Clone)]
pub struct MutationResult {
    pub mutation: StateMutation,
    pub success: bool,
    pub error: Option<String>,
    pub spawned_id: Option<Uuid>,
}

impl MutationResult {
    pub fn success(mutation: StateMutation) -> Self {
        Self {
            mutation,
            success: true,
            error: None,
            spawned_id: None,
        }
    }

    pub fn success_with_id(mutation: StateMutation, id: Uuid) -> Self {
        Self {
            mutation,
            success: true,
            error: None,
            spawned_id: Some(id),
        }
    }

    pub fn failure(mutation: StateMutation, error: impl Into<String>) -> Self {
        Self {
            mutation,
            success: false,
            error: Some(error.into()),
            spawned_id: None,
        }
    }
}

// Helper function to parse ship class from string
fn parse_ship_class(s: &str) -> Option<ShipClass> {
    match s.to_lowercase().as_str() {
        "patrol_corvette" | "patrolcorvette" => Some(ShipClass::PatrolCorvette),
        "frigate" => Some(ShipClass::Frigate),
        "destroyer" => Some(ShipClass::Destroyer),
        "cruiser" => Some(ShipClass::Cruiser),
        "carrier" => Some(ShipClass::Carrier),
        "freighter" => Some(ShipClass::Freighter),
        "transport" => Some(ShipClass::Transport),
        "mining_vessel" | "miningvessel" => Some(ShipClass::MiningVessel),
        "pirate_raider" | "pirateraider" => Some(ShipClass::PirateRaider),
        "pirate_frigate" | "piratefrigate" => Some(ShipClass::PirateFrigate),
        "terrorist_bomber" | "terroristbomber" => Some(ShipClass::TerroristBomber),
        "sera_swarm" | "seraswarm" => Some(ShipClass::SeraSwarm),
        "sera_hunter" | "serahunter" => Some(ShipClass::SeraHunter),
        "drone_harvester" | "droneharvester" => Some(ShipClass::DroneHarvester),
        "drone_swarm" | "droneswarm" => Some(ShipClass::DroneSwarm),
        _ => None,
    }
}
