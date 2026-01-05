//! Entity behavior system
//!
//! Allows scripts to be attached to entities (ships, stations, etc.) with
//! lifecycle hooks: on_spawn, on_update, on_destroy.
//!
//! # Behavior Trees
//!
//! Entities can optionally have behavior trees for AI decision making.
//! Set the behavior tree in `on_spawn`:
//!
//! ```rhai
//! fn on_spawn(ctx) {
//!     ctx.local_data.behavior_tree = bt_selector([
//!         bt_sequence([
//!             bt_condition("is_low_health"),
//!             bt_action("flee")
//!         ]),
//!         bt_action("patrol")
//!     ]);
//! }
//! ```

mod manager;

pub use manager::*;

use rhai::{Dynamic, Map};
use uuid::Uuid;

use bw_ai::BtNode;
pub use bw_game::EntityType;

/// A behavior script attached to an entity.
#[derive(Debug, Clone)]
pub struct EntityBehavior {
    /// Unique ID for this behavior instance
    pub id: Uuid,
    /// ID of the entity this behavior is attached to
    pub entity_id: Uuid,
    /// Type of entity
    pub entity_type: EntityType,
    /// Script file to use
    pub script_path: String,
    /// Current state of the behavior
    pub state: BehaviorState,
    /// Persistent local data for the script
    pub local_data: Map,
    /// Sector the entity is in
    pub sector_id: Option<Uuid>,
    /// Optional behavior tree for AI decision making
    pub behavior_tree: Option<BtNode>,
}

impl EntityBehavior {
    /// Create a new behavior attachment.
    pub fn new(
        entity_id: Uuid,
        entity_type: EntityType,
        script_path: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_id,
            entity_type,
            script_path: script_path.into(),
            state: BehaviorState::Initializing,
            local_data: Map::new(),
            sector_id: None,
            behavior_tree: None,
        }
    }

    /// Set the sector.
    pub fn with_sector(mut self, sector_id: Uuid) -> Self {
        self.sector_id = Some(sector_id);
        self
    }

    /// Set the behavior tree.
    pub fn with_behavior_tree(mut self, tree: BtNode) -> Self {
        self.behavior_tree = Some(tree);
        self
    }

    /// Check if behavior is active.
    pub fn is_active(&self) -> bool {
        matches!(self.state, BehaviorState::Active)
    }

    /// Check if behavior has a behavior tree.
    pub fn has_behavior_tree(&self) -> bool {
        self.behavior_tree.is_some()
    }
}

/// State of a behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorState {
    /// Behavior is being initialized (on_spawn not yet called)
    Initializing,
    /// Behavior is active and receiving updates
    Active,
    /// Behavior is temporarily paused
    Paused,
    /// Behavior is being destroyed (on_destroy will be called)
    Destroying,
    /// Behavior has been destroyed and removed
    Destroyed,
}

/// Context passed to behavior scripts.
#[derive(Clone)]
pub struct BehaviorContext {
    /// The behavior instance ID
    pub behavior_id: Uuid,
    /// The entity this behavior is attached to
    pub entity_id: Uuid,
    /// Entity type
    pub entity_type: EntityType,
    /// Current sector
    pub sector_id: Uuid,
    /// Current game tick
    pub tick: u64,
    /// Time since last update (seconds)
    pub delta_time: f64,
    /// Persistent local data for the script
    pub local_data: Map,
}

impl BehaviorContext {
    /// Convert to Rhai Dynamic map.
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("behavior_id".into(), self.behavior_id.to_string().into());
        map.insert("entity_id".into(), self.entity_id.to_string().into());
        map.insert("entity_type".into(), self.entity_type.as_str().into());
        map.insert("sector_id".into(), self.sector_id.to_string().into());
        map.insert("tick".into(), (self.tick as i64).into());
        map.insert("delta_time".into(), self.delta_time.into());
        map.insert("local_data".into(), Dynamic::from(self.local_data.clone()));
        Dynamic::from(map)
    }

    /// Update local_data from a Rhai map.
    pub fn update_local_data(&mut self, map: &Map) {
        if let Some(data) = map.get("local_data") {
            if let Some(new_data) = data.clone().try_cast::<Map>() {
                self.local_data = new_data;
            }
        }
    }
}

/// Result from executing a behavior hook.
#[derive(Debug)]
pub struct BehaviorExecResult {
    pub behavior_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
    pub updated_local_data: Option<Map>,
}

impl BehaviorExecResult {
    pub fn success(behavior_id: Uuid) -> Self {
        Self {
            behavior_id,
            success: true,
            error: None,
            updated_local_data: None,
        }
    }

    pub fn success_with_data(behavior_id: Uuid, data: Map) -> Self {
        Self {
            behavior_id,
            success: true,
            error: None,
            updated_local_data: Some(data),
        }
    }

    pub fn failure(behavior_id: Uuid, error: impl Into<String>) -> Self {
        Self {
            behavior_id,
            success: false,
            error: Some(error.into()),
            updated_local_data: None,
        }
    }
}
