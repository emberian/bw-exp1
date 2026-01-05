//! Event system for scripts
//!
//! Allows scripts to subscribe to game events and react to them.
//! Events are dispatched to handlers in priority order.

mod dispatcher;

pub use dispatcher::*;

use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

use bw_core::events::GameEventType;

/// A script's subscription to events.
#[derive(Debug, Clone)]
pub struct EventSubscription {
    /// Unique ID for this subscription
    pub id: Uuid,
    /// Script that owns this subscription
    pub script_path: String,
    /// Function to call when event occurs
    pub handler_function: String,
    /// Event types this subscription matches
    pub event_types: Vec<GameEventType>,
    /// Optional filter to narrow event scope
    pub filter: Option<EventFilter>,
    /// Priority (lower runs first)
    pub priority: i32,
    /// Whether this subscription is active
    pub enabled: bool,
    /// Entity that owns this subscription (for cleanup)
    pub owner_entity_id: Option<Uuid>,
    /// Sector filter (only events from this sector)
    pub sector_id: Option<Uuid>,
}

impl EventSubscription {
    /// Create a new subscription.
    pub fn new(
        script_path: impl Into<String>,
        handler_function: impl Into<String>,
        event_types: Vec<GameEventType>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            script_path: script_path.into(),
            handler_function: handler_function.into(),
            event_types,
            filter: None,
            priority: 0,
            enabled: true,
            owner_entity_id: None,
            sector_id: None,
        }
    }

    /// Set the owner entity.
    pub fn with_owner(mut self, entity_id: Uuid) -> Self {
        self.owner_entity_id = Some(entity_id);
        self
    }

    /// Set the sector filter.
    pub fn with_sector(mut self, sector_id: Uuid) -> Self {
        self.sector_id = Some(sector_id);
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set filter.
    pub fn with_filter(mut self, filter: EventFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// Filter for narrowing event scope.
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// Only events from a specific sector
    SectorId(Uuid),
    /// Only events with a specific actor
    ActorId(Uuid),
    /// Only events targeting a specific entity
    TargetId(Uuid),
    /// Custom filter function name (called in script)
    Custom(String),
}

impl EventFilter {
    /// Check if an event passes this filter.
    pub fn matches(
        &self,
        sector_id: Uuid,
        actor_id: Option<Uuid>,
        target_id: Option<Uuid>,
    ) -> bool {
        match self {
            Self::SectorId(id) => sector_id == *id,
            Self::ActorId(id) => actor_id == Some(*id),
            Self::TargetId(id) => target_id == Some(*id),
            Self::Custom(_) => true, // Custom filters checked in script
        }
    }
}

/// Registry of event subscriptions.
pub struct EventRegistry {
    /// Subscriptions indexed by event type for fast lookup
    by_event_type: RwLock<HashMap<GameEventType, Vec<EventSubscription>>>,
    /// All subscriptions by ID for management
    by_id: RwLock<HashMap<Uuid, EventSubscription>>,
    /// Subscriptions by owner entity for cleanup
    by_owner: RwLock<HashMap<Uuid, Vec<Uuid>>>,
}

impl EventRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            by_event_type: RwLock::new(HashMap::new()),
            by_id: RwLock::new(HashMap::new()),
            by_owner: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to events.
    pub fn subscribe(&self, sub: EventSubscription) -> Uuid {
        let id = sub.id;

        // Add to event type index
        {
            let mut by_type = self.by_event_type.write();
            for event_type in &sub.event_types {
                by_type.entry(*event_type)
                    .or_default()
                    .push(sub.clone());
            }
        }

        // Add to owner index
        if let Some(owner_id) = sub.owner_entity_id {
            self.by_owner.write()
                .entry(owner_id)
                .or_default()
                .push(id);
        }

        // Add to ID index
        self.by_id.write().insert(id, sub);

        tracing::debug!(subscription_id = %id, "Event subscription added");

        id
    }

    /// Unsubscribe by ID.
    pub fn unsubscribe(&self, id: Uuid) -> bool {
        let sub = self.by_id.write().remove(&id);

        if let Some(sub) = sub {
            // Remove from event type index
            let mut by_type = self.by_event_type.write();
            for event_type in &sub.event_types {
                if let Some(subs) = by_type.get_mut(event_type) {
                    subs.retain(|s| s.id != id);
                }
            }

            // Remove from owner index
            if let Some(owner_id) = sub.owner_entity_id {
                if let Some(owner_subs) = self.by_owner.write().get_mut(&owner_id) {
                    owner_subs.retain(|sid| *sid != id);
                }
            }

            tracing::debug!(subscription_id = %id, "Event subscription removed");
            true
        } else {
            false
        }
    }

    /// Unsubscribe all subscriptions for an entity.
    pub fn unsubscribe_for_entity(&self, entity_id: Uuid) {
        let sub_ids: Vec<Uuid> = self.by_owner.read()
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        for id in sub_ids {
            self.unsubscribe(id);
        }

        self.by_owner.write().remove(&entity_id);
    }

    /// Get all handlers for an event type.
    pub fn get_handlers(&self, event_type: GameEventType) -> Vec<EventSubscription> {
        self.by_event_type.read()
            .get(&event_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Get handlers sorted by priority.
    pub fn get_handlers_sorted(&self, event_type: GameEventType) -> Vec<EventSubscription> {
        let mut handlers = self.get_handlers(event_type);
        handlers.sort_by_key(|h| h.priority);
        handlers
    }

    /// Get a subscription by ID.
    pub fn get(&self, id: Uuid) -> Option<EventSubscription> {
        self.by_id.read().get(&id).cloned()
    }

    /// Enable/disable a subscription.
    pub fn set_enabled(&self, id: Uuid, enabled: bool) {
        if let Some(sub) = self.by_id.write().get_mut(&id) {
            sub.enabled = enabled;
        }

        // Update in event type index
        let mut by_type = self.by_event_type.write();
        for subs in by_type.values_mut() {
            for sub in subs.iter_mut() {
                if sub.id == id {
                    sub.enabled = enabled;
                }
            }
        }
    }

    /// Get count of subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.by_id.read().len()
    }

    /// Get count of subscriptions (alias for subscription_count).
    pub fn count(&self) -> usize {
        self.subscription_count()
    }

    /// Get count of subscriptions for an entity.
    pub fn count_for_entity(&self, entity_id: Uuid) -> usize {
        self.by_owner.read()
            .get(&entity_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// List all subscriptions.
    pub fn list_all(&self) -> Vec<EventSubscription> {
        self.by_id.read().values().cloned().collect()
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert event type string to GameEventType.
pub fn parse_event_type(s: &str) -> Option<GameEventType> {
    match s.to_lowercase().replace("_", "").as_str() {
        "playerjoined" => Some(GameEventType::PlayerJoined),
        "playerleft" => Some(GameEventType::PlayerLeft),
        "playerdocked" => Some(GameEventType::PlayerDocked),
        "playerundocked" => Some(GameEventType::PlayerUndocked),
        "shipspawned" => Some(GameEventType::ShipSpawned),
        "shipdestroyed" => Some(GameEventType::ShipDestroyed),
        "shipdamaged" => Some(GameEventType::ShipDamaged),
        "shiprepaired" => Some(GameEventType::ShipRepaired),
        "shiprefueled" => Some(GameEventType::ShipRefueled),
        "shiprearmed" => Some(GameEventType::ShipRearmed),
        "combatstarted" => Some(GameEventType::CombatStarted),
        "combatended" => Some(GameEventType::CombatEnded),
        "attackhit" => Some(GameEventType::AttackHit),
        "attackmissed" => Some(GameEventType::AttackMissed),
        "missionspawned" => Some(GameEventType::MissionSpawned),
        "missionaccepted" => Some(GameEventType::MissionAccepted),
        "missioncompleted" => Some(GameEventType::MissionCompleted),
        "missionfailed" => Some(GameEventType::MissionFailed),
        "missionexpired" => Some(GameEventType::MissionExpired),
        "missionchoicemade" => Some(GameEventType::MissionChoiceMade),
        "reputationgained" => Some(GameEventType::ReputationGained),
        "reputationlost" => Some(GameEventType::ReputationLost),
        "famegained" => Some(GameEventType::FameGained),
        "famelost" => Some(GameEventType::FameLost),
        "playerdisgraced" => Some(GameEventType::PlayerDisgraced),
        "squadroncreated" => Some(GameEventType::SquadronCreated),
        "squadrondissolved" => Some(GameEventType::SquadronDissolved),
        "squadronmemberjoined" => Some(GameEventType::SquadronMemberJoined),
        "squadronmemberleft" => Some(GameEventType::SquadronMemberLeft),
        "squadronwardeclared" => Some(GameEventType::SquadronWarDeclared),
        "squadronpeacedeclared" => Some(GameEventType::SquadronPeaceDeclared),
        "sectorcontrolchanged" => Some(GameEventType::SectorControlChanged),
        "stationbuilt" => Some(GameEventType::StationBuilt),
        "stationdestroyed" => Some(GameEventType::StationDestroyed),
        "seraincursion" => Some(GameEventType::SeraIncursion),
        "droneswarmdetected" => Some(GameEventType::DroneSwarmDetected),
        "asteroidalert" => Some(GameEventType::AsteroidAlert),
        "distresssignalreceived" => Some(GameEventType::DistressSignalReceived),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_subscribe_unsubscribe() {
        let registry = EventRegistry::new();

        let sub = EventSubscription::new(
            "test.rhai",
            "on_combat",
            vec![GameEventType::CombatStarted],
        );
        let id = sub.id;

        registry.subscribe(sub);
        assert_eq!(registry.subscription_count(), 1);

        let handlers = registry.get_handlers(GameEventType::CombatStarted);
        assert_eq!(handlers.len(), 1);

        registry.unsubscribe(id);
        assert_eq!(registry.subscription_count(), 0);
    }

    #[test]
    fn test_parse_event_type() {
        assert_eq!(parse_event_type("ShipSpawned"), Some(GameEventType::ShipSpawned));
        assert_eq!(parse_event_type("ship_spawned"), Some(GameEventType::ShipSpawned));
        assert_eq!(parse_event_type("combat_started"), Some(GameEventType::CombatStarted));
        assert_eq!(parse_event_type("invalid"), None);
    }
}
