//! State mutations for scripts
//!
//! Scripts queue mutations which are validated and applied atomically
//! after script execution completes. This ensures state consistency.

use rhai::{Dynamic, Map};
use uuid::Uuid;

use bw_core::models::{Position, ShipClass, ShipStatus};
use crate::RhaiDeserialize;

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

// =============================================================================
// Helper functions for complex field parsing (used by RhaiDeserialize)
// =============================================================================

/// Parse Position from Dynamic map.
fn parse_position(v: Option<&Dynamic>) -> Option<Option<Position>> {
    let v = v?;
    let pos_map = v.clone().try_cast::<Map>()?;
    let x = pos_map.get("x").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
    let y = pos_map.get("y").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
    let z = pos_map.get("z").and_then(|v| v.as_float().ok()).unwrap_or(0.0);
    Some(Some(Position::new(x, y, z)))
}

/// Parse ShipStatusChange from Dynamic string.
fn parse_status(v: Option<&Dynamic>) -> Option<Option<ShipStatusChange>> {
    let v = v?;
    let status_str = v.clone().into_string().ok()?;
    Some(ShipStatusChange::from_string(&status_str))
}

/// Parse locked_target with special handling for clearing.
/// Returns Some(None) to clear target, Some(Some(uuid)) to set target.
fn parse_locked_target(v: Option<&Dynamic>) -> Option<Option<Option<Uuid>>> {
    let v = v?;
    if v.is_unit() {
        return Some(Some(None)); // Clear target
    }
    if let Ok(target_str) = v.clone().into_string() {
        if target_str.is_empty() {
            return Some(Some(None)); // Clear target
        }
        if let Ok(uuid) = Uuid::parse_str(&target_str) {
            return Some(Some(Some(uuid))); // Set target
        }
    }
    None // Invalid input
}

/// Parse CargoChange from Dynamic map.
fn parse_cargo_change(v: Option<&Dynamic>) -> Option<Option<CargoChange>> {
    let v = v?;
    let cargo_map = v.clone().try_cast::<Map>()?;
    let cargo_type = cargo_map.get("type")
        .and_then(|v| v.clone().into_string().ok())
        .unwrap_or_default();
    let quantity = cargo_map.get("quantity")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0) as u32;
    let purchase_price = cargo_map.get("price")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0);

    if cargo_type.is_empty() || quantity == 0 {
        return Some(None);
    }
    Some(Some(CargoChange { cargo_type, quantity, purchase_price }))
}

/// Parse CargoChange for removal (no price needed).
fn parse_cargo_removal(v: Option<&Dynamic>) -> Option<Option<CargoChange>> {
    let v = v?;
    let cargo_map = v.clone().try_cast::<Map>()?;
    let cargo_type = cargo_map.get("type")
        .and_then(|v| v.clone().into_string().ok())
        .unwrap_or_default();
    let quantity = cargo_map.get("quantity")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0) as u32;

    if cargo_type.is_empty() || quantity == 0 {
        return Some(None);
    }
    Some(Some(CargoChange { cargo_type, quantity, purchase_price: 0 }))
}

/// Parse UpgradeInstall from Dynamic map.
fn parse_upgrade_install(v: Option<&Dynamic>) -> Option<Option<UpgradeInstall>> {
    let v = v?;
    let upgrade_map = v.clone().try_cast::<Map>()?;
    let upgrade_id = upgrade_map.get("upgrade_id")
        .and_then(|v| v.clone().into_string().ok())?;
    let slot = upgrade_map.get("slot")
        .and_then(|v| v.clone().into_string().ok())?;
    Some(Some(UpgradeInstall { upgrade_id, slot }))
}

/// Changes to apply to a ship.
#[derive(Debug, Clone, Default, RhaiDeserialize)]
pub struct ShipChanges {
    #[rhai(default)]
    pub hull: Option<f32>,
    #[rhai(default)]
    pub shields: Option<f32>,
    #[rhai(default)]
    pub ammunition: Option<f32>,
    #[rhai(default)]
    pub fuel: Option<f32>,
    #[rhai(default)]
    pub morale: Option<f32>,
    #[rhai(default)]
    pub experience: Option<i32>,
    #[rhai(default, with = "parse_position")]
    pub position: Option<Position>,
    #[rhai(default, with = "parse_status")]
    pub status: Option<ShipStatusChange>,
    #[rhai(default)]
    pub combat_stance: Option<String>,
    #[rhai(default, with = "parse_locked_target")]
    pub locked_target: Option<Option<Uuid>>,
    #[rhai(default, with = "parse_cargo_change")]
    pub add_cargo: Option<CargoChange>,
    #[rhai(default, with = "parse_cargo_removal")]
    pub remove_cargo: Option<CargoChange>,
    #[rhai(default, with = "parse_upgrade_install")]
    pub install_upgrade: Option<UpgradeInstall>,
    #[rhai(default)]
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
#[derive(Debug, Clone, Default, RhaiDeserialize)]
pub struct PlayerChanges {
    #[rhai(default)]
    pub reputation: Option<i32>,
    #[rhai(default)]
    pub fame: Option<i32>,
    #[rhai(default)]
    pub reputation_delta: Option<i32>,
    #[rhai(default)]
    pub fame_delta: Option<i32>,
    #[rhai(default)]
    pub credits: Option<i64>,
    #[rhai(default)]
    pub credits_delta: Option<i64>,
}

impl PlayerChanges {
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
