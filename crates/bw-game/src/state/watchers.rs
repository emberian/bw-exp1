//! Property Watch System
//!
//! Enables reactive callbacks when entity properties change.
//!
//! # Example
//!
//! ```rhai
//! // Watch for critical damage
//! watch_property(ship.id, "hull", "less_than", 25.0, "on_critical_damage");
//!
//! // Watch for any position changes
//! watch_property(target.id, "position", "changed", (), "on_target_moved");
//!
//! // One-shot watch (automatically removed after triggering)
//! watch_once(enemy.id, "hull", "less_than", 0.0, "on_enemy_destroyed");
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use rhai::Dynamic;

/// A property watch that triggers callbacks on state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyWatch {
    /// Unique identifier for this watch.
    pub id: Uuid,
    /// The entity being watched.
    pub entity_id: Uuid,
    /// The property path to watch (e.g., "hull", "position.x", "cargo.count").
    pub property: String,
    /// The condition that triggers the callback.
    pub condition: WatchCondition,
    /// Name of the callback function to invoke.
    pub callback: String,
    /// Whether this watch should be removed after triggering.
    pub one_shot: bool,
    /// The script that created this watch.
    pub owner_script: String,
    /// Optional sector scope (only watch if entity is in this sector).
    pub sector_id: Option<Uuid>,
    /// Last known value (for change detection).
    #[serde(skip)]
    pub last_value: Option<WatchValue>,
    /// Whether this watch is active.
    pub active: bool,
    /// Number of times this watch has triggered.
    pub trigger_count: u32,
    /// Creation timestamp.
    pub created_at: u64,
}

impl PropertyWatch {
    /// Create a new property watch.
    pub fn new(
        entity_id: Uuid,
        property: impl Into<String>,
        condition: WatchCondition,
        callback: impl Into<String>,
        owner_script: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_id,
            property: property.into(),
            condition,
            callback: callback.into(),
            one_shot: false,
            owner_script: owner_script.into(),
            sector_id: None,
            last_value: None,
            active: true,
            trigger_count: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Set this watch to one-shot mode.
    pub fn one_shot(mut self) -> Self {
        self.one_shot = true;
        self
    }

    /// Set the sector scope.
    pub fn in_sector(mut self, sector_id: Uuid) -> Self {
        self.sector_id = Some(sector_id);
        self
    }

    /// Check if the watch should trigger based on the new value.
    pub fn should_trigger(&mut self, new_value: &WatchValue) -> bool {
        if !self.active {
            return false;
        }

        let triggered = match &self.condition {
            WatchCondition::Changed => {
                self.last_value.as_ref() != Some(new_value)
            }
            WatchCondition::LessThan(threshold) => {
                if let WatchValue::Float(v) = new_value {
                    *v < *threshold
                } else if let WatchValue::Int(v) = new_value {
                    (*v as f64) < *threshold
                } else {
                    false
                }
            }
            WatchCondition::GreaterThan(threshold) => {
                if let WatchValue::Float(v) = new_value {
                    *v > *threshold
                } else if let WatchValue::Int(v) = new_value {
                    (*v as f64) > *threshold
                } else {
                    false
                }
            }
            WatchCondition::Equals(expected) => {
                new_value == expected
            }
            WatchCondition::NotEquals(expected) => {
                new_value != expected
            }
            WatchCondition::CrossedThreshold(threshold) => {
                if let (Some(WatchValue::Float(old)), WatchValue::Float(new)) = (&self.last_value, new_value) {
                    (*old < *threshold && *new >= *threshold) || (*old >= *threshold && *new < *threshold)
                } else if let (Some(WatchValue::Int(old)), WatchValue::Int(new)) = (&self.last_value, new_value) {
                    let t = *threshold as i64;
                    (*old < t && *new >= t) || (*old >= t && *new < t)
                } else {
                    false
                }
            }
            WatchCondition::InRange(min, max) => {
                if let WatchValue::Float(v) = new_value {
                    *v >= *min && *v <= *max
                } else if let WatchValue::Int(v) = new_value {
                    let vf = *v as f64;
                    vf >= *min && vf <= *max
                } else {
                    false
                }
            }
            WatchCondition::OutOfRange(min, max) => {
                if let WatchValue::Float(v) = new_value {
                    *v < *min || *v > *max
                } else if let WatchValue::Int(v) = new_value {
                    let vf = *v as f64;
                    vf < *min || vf > *max
                } else {
                    false
                }
            }
        };

        // Update last value
        self.last_value = Some(new_value.clone());

        if triggered {
            self.trigger_count += 1;
            if self.one_shot {
                self.active = false;
            }
        }

        triggered
    }
}

/// Conditions that can trigger a watch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchCondition {
    /// Triggers when the value changes from its previous value.
    Changed,
    /// Triggers when the value is less than the threshold.
    LessThan(f64),
    /// Triggers when the value is greater than the threshold.
    GreaterThan(f64),
    /// Triggers when the value equals the expected value.
    Equals(WatchValue),
    /// Triggers when the value does not equal the expected value.
    NotEquals(WatchValue),
    /// Triggers when the value crosses the threshold (in either direction).
    CrossedThreshold(f64),
    /// Triggers when the value is within the range [min, max].
    InRange(f64, f64),
    /// Triggers when the value is outside the range [min, max].
    OutOfRange(f64, f64),
}

impl WatchCondition {
    /// Parse a condition from string and optional threshold.
    pub fn parse(condition_type: &str, threshold: Option<f64>) -> Option<Self> {
        match condition_type.to_lowercase().as_str() {
            "changed" => Some(Self::Changed),
            "less_than" | "lt" | "<" => threshold.map(Self::LessThan),
            "greater_than" | "gt" | ">" => threshold.map(Self::GreaterThan),
            "crossed" | "crossed_threshold" => threshold.map(Self::CrossedThreshold),
            _ => None,
        }
    }
}

/// Values that can be watched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    String(String),
    Uuid(Uuid),
    Unit,
}

impl WatchValue {
    /// Convert from Rhai Dynamic.
    pub fn from_dynamic(value: &Dynamic) -> Self {
        if value.is_unit() {
            Self::Unit
        } else if let Some(b) = value.clone().try_cast::<bool>() {
            Self::Bool(b)
        } else if let Some(i) = value.clone().try_cast::<i64>() {
            Self::Int(i)
        } else if let Some(f) = value.clone().try_cast::<f64>() {
            Self::Float(f)
        } else if let Some(s) = value.clone().try_cast::<String>() {
            // Try to parse as UUID
            if let Ok(uuid) = Uuid::parse_str(&s) {
                Self::Uuid(uuid)
            } else {
                Self::String(s)
            }
        } else {
            Self::String(value.to_string())
        }
    }

    /// Convert to Rhai Dynamic.
    pub fn to_dynamic(&self) -> Dynamic {
        match self {
            Self::Float(f) => Dynamic::from(*f),
            Self::Int(i) => Dynamic::from(*i),
            Self::Bool(b) => Dynamic::from(*b),
            Self::String(s) => Dynamic::from(s.clone()),
            Self::Uuid(u) => Dynamic::from(u.to_string()),
            Self::Unit => Dynamic::UNIT,
        }
    }
}

/// Event emitted when a watch triggers.
#[derive(Debug, Clone)]
pub struct WatchTriggerEvent {
    /// The watch that triggered.
    pub watch_id: Uuid,
    /// The entity being watched.
    pub entity_id: Uuid,
    /// The property that changed.
    pub property: String,
    /// The old value (if available).
    pub old_value: Option<WatchValue>,
    /// The new value.
    pub new_value: WatchValue,
    /// The callback to invoke.
    pub callback: String,
    /// The script that owns the watch.
    pub owner_script: String,
}

/// Registry for managing property watches.
#[derive(Default)]
pub struct WatchRegistry {
    /// All registered watches, indexed by ID.
    watches: RwLock<HashMap<Uuid, PropertyWatch>>,
    /// Watches indexed by entity ID for fast lookup.
    by_entity: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Watches indexed by owner script.
    by_script: RwLock<HashMap<String, Vec<Uuid>>>,
}

impl WatchRegistry {
    /// Create a new watch registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a shareable instance.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Register a new property watch.
    pub fn register(&self, watch: PropertyWatch) -> Uuid {
        let id = watch.id;
        let entity_id = watch.entity_id;
        let owner_script = watch.owner_script.clone();

        self.watches.write().insert(id, watch);
        self.by_entity.write()
            .entry(entity_id)
            .or_default()
            .push(id);
        self.by_script.write()
            .entry(owner_script)
            .or_default()
            .push(id);

        tracing::debug!(watch_id = %id, entity_id = %entity_id, "Property watch registered");
        id
    }

    /// Unregister a watch by ID.
    pub fn unregister(&self, watch_id: Uuid) -> bool {
        let watch = self.watches.write().remove(&watch_id);

        if let Some(watch) = watch {
            // Remove from entity index
            if let Some(ids) = self.by_entity.write().get_mut(&watch.entity_id) {
                ids.retain(|id| *id != watch_id);
            }

            // Remove from script index
            if let Some(ids) = self.by_script.write().get_mut(&watch.owner_script) {
                ids.retain(|id| *id != watch_id);
            }

            tracing::debug!(watch_id = %watch_id, "Property watch unregistered");
            true
        } else {
            false
        }
    }

    /// Unregister all watches for an entity.
    pub fn unregister_for_entity(&self, entity_id: Uuid) {
        let watch_ids: Vec<Uuid> = self.by_entity.read()
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        for id in watch_ids {
            self.unregister(id);
        }

        self.by_entity.write().remove(&entity_id);
    }

    /// Unregister all watches for a script.
    pub fn unregister_for_script(&self, script_path: &str) {
        let watch_ids: Vec<Uuid> = self.by_script.read()
            .get(script_path)
            .cloned()
            .unwrap_or_default();

        for id in watch_ids {
            self.unregister(id);
        }

        self.by_script.write().remove(script_path);
    }

    /// Check all watches for an entity and return triggered events.
    ///
    /// The `get_value` closure should fetch the current value for a property.
    pub fn check_entity<F>(
        &self,
        entity_id: Uuid,
        get_value: F,
    ) -> Vec<WatchTriggerEvent>
    where
        F: Fn(&str) -> Option<WatchValue>,
    {
        let watch_ids: Vec<Uuid> = self.by_entity.read()
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        let mut events = Vec::new();
        let mut to_remove = Vec::new();

        for watch_id in watch_ids {
            let mut watches = self.watches.write();
            if let Some(watch) = watches.get_mut(&watch_id)
                && let Some(new_value) = get_value(&watch.property)
            {
                let old_value = watch.last_value.clone();

                if watch.should_trigger(&new_value) {
                    events.push(WatchTriggerEvent {
                        watch_id: watch.id,
                        entity_id: watch.entity_id,
                        property: watch.property.clone(),
                        old_value,
                        new_value,
                        callback: watch.callback.clone(),
                        owner_script: watch.owner_script.clone(),
                    });

                    if !watch.active {
                        to_remove.push(watch_id);
                    }
                }
            }
        }

        // Remove one-shot watches that triggered
        drop(self.watches.write());
        for id in to_remove {
            self.unregister(id);
        }

        events
    }

    /// Get a watch by ID.
    pub fn get(&self, watch_id: Uuid) -> Option<PropertyWatch> {
        self.watches.read().get(&watch_id).cloned()
    }

    /// List all watches for an entity.
    pub fn list_for_entity(&self, entity_id: Uuid) -> Vec<PropertyWatch> {
        let ids = self.by_entity.read()
            .get(&entity_id)
            .cloned()
            .unwrap_or_default();

        let watches = self.watches.read();
        ids.iter()
            .filter_map(|id| watches.get(id).cloned())
            .collect()
    }

    /// Get total number of watches.
    pub fn count(&self) -> usize {
        self.watches.read().len()
    }

    /// Pause a watch.
    pub fn pause(&self, watch_id: Uuid) -> bool {
        if let Some(watch) = self.watches.write().get_mut(&watch_id) {
            watch.active = false;
            true
        } else {
            false
        }
    }

    /// Resume a watch.
    pub fn resume(&self, watch_id: Uuid) -> bool {
        if let Some(watch) = self.watches.write().get_mut(&watch_id) {
            watch.active = true;
            true
        } else {
            false
        }
    }

    /// Clear all watches.
    pub fn clear(&self) {
        self.watches.write().clear();
        self.by_entity.write().clear();
        self.by_script.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_condition_less_than() {
        let mut watch = PropertyWatch::new(
            Uuid::new_v4(),
            "hull",
            WatchCondition::LessThan(25.0),
            "on_low_health",
            "test.rhai",
        );

        // Should not trigger when value is above threshold
        assert!(!watch.should_trigger(&WatchValue::Float(50.0)));

        // Should trigger when value is below threshold
        assert!(watch.should_trigger(&WatchValue::Float(20.0)));
    }

    #[test]
    fn test_watch_condition_changed() {
        let mut watch = PropertyWatch::new(
            Uuid::new_v4(),
            "position",
            WatchCondition::Changed,
            "on_move",
            "test.rhai",
        );

        // First value - triggers because last_value was None
        assert!(watch.should_trigger(&WatchValue::Float(10.0)));

        // Same value - should not trigger
        assert!(!watch.should_trigger(&WatchValue::Float(10.0)));

        // Different value - should trigger
        assert!(watch.should_trigger(&WatchValue::Float(20.0)));
    }

    #[test]
    fn test_watch_condition_crossed_threshold() {
        let mut watch = PropertyWatch::new(
            Uuid::new_v4(),
            "shields",
            WatchCondition::CrossedThreshold(50.0),
            "on_shields_crossed",
            "test.rhai",
        );

        // Set initial value
        watch.last_value = Some(WatchValue::Float(60.0));

        // Crossing down - should trigger
        assert!(watch.should_trigger(&WatchValue::Float(40.0)));

        // Crossing up - should trigger
        assert!(watch.should_trigger(&WatchValue::Float(60.0)));

        // Not crossing - should not trigger
        assert!(!watch.should_trigger(&WatchValue::Float(70.0)));
    }

    #[test]
    fn test_one_shot_watch() {
        let mut watch = PropertyWatch::new(
            Uuid::new_v4(),
            "hull",
            WatchCondition::LessThan(0.0),
            "on_destroy",
            "test.rhai",
        ).one_shot();

        // Should trigger
        assert!(watch.should_trigger(&WatchValue::Float(-1.0)));

        // Should not trigger again (one-shot)
        assert!(!watch.should_trigger(&WatchValue::Float(-2.0)));
        assert!(!watch.active);
    }

    #[test]
    fn test_registry_basic() {
        let registry = WatchRegistry::new();
        let entity_id = Uuid::new_v4();

        let watch = PropertyWatch::new(
            entity_id,
            "hull",
            WatchCondition::LessThan(25.0),
            "on_low_health",
            "test.rhai",
        );

        let watch_id = registry.register(watch);
        assert_eq!(registry.count(), 1);

        let watches = registry.list_for_entity(entity_id);
        assert_eq!(watches.len(), 1);
        assert_eq!(watches[0].id, watch_id);

        registry.unregister(watch_id);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_check_entity() {
        let registry = WatchRegistry::new();
        let entity_id = Uuid::new_v4();

        let watch = PropertyWatch::new(
            entity_id,
            "hull",
            WatchCondition::LessThan(25.0),
            "on_low_health",
            "test.rhai",
        );

        registry.register(watch);

        // Check with value above threshold - no trigger
        let events = registry.check_entity(entity_id, |prop| {
            if prop == "hull" { Some(WatchValue::Float(50.0)) } else { None }
        });
        assert!(events.is_empty());

        // Check with value below threshold - triggers
        let events = registry.check_entity(entity_id, |prop| {
            if prop == "hull" { Some(WatchValue::Float(20.0)) } else { None }
        });
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].callback, "on_low_health");
    }

    #[test]
    fn test_watch_value_from_dynamic() {
        assert_eq!(WatchValue::from_dynamic(&Dynamic::from(42_i64)), WatchValue::Int(42));
        assert_eq!(WatchValue::from_dynamic(&Dynamic::from(3.14_f64)), WatchValue::Float(3.14));
        assert_eq!(WatchValue::from_dynamic(&Dynamic::from(true)), WatchValue::Bool(true));
        assert_eq!(WatchValue::from_dynamic(&Dynamic::from("test".to_string())), WatchValue::String("test".to_string()));
        assert_eq!(WatchValue::from_dynamic(&Dynamic::UNIT), WatchValue::Unit);
    }
}
