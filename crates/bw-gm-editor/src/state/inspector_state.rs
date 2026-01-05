//! State inspector state for live state viewing
#![allow(dead_code)] // Public API methods not yet used by all UI components

use leptos::prelude::*;
use bw_shared::{EntityType, dto::{StateChangeDto, WatchDto}};
use uuid::Uuid;

/// Maximum number of entities to store in memory
const MAX_ENTITIES: usize = 1000;
/// Maximum number of recent changes to keep
const MAX_CHANGES: usize = 100;

/// State for the state inspector component
#[derive(Clone, Copy)]
pub struct InspectorState {
    /// Currently selected entity type
    pub selected_type: RwSignal<Option<EntityType>>,
    /// Entities in current snapshot (limited to MAX_ENTITIES)
    pub entities: RwSignal<Vec<serde_json::Value>>,
    /// Total count for pagination
    pub total_count: RwSignal<usize>,
    /// Current offset for pagination
    pub offset: RwSignal<usize>,
    /// Page size
    pub page_size: RwSignal<usize>,
    /// Selected entity ID
    pub selected_entity: RwSignal<Option<Uuid>>,
    /// Selected entity details
    pub entity_detail: RwSignal<Option<serde_json::Value>>,
    /// Recent state changes (limited to MAX_CHANGES)
    pub changes: RwSignal<Vec<StateChangeDto>>,
    /// Current tick
    pub current_tick: RwSignal<u64>,
    /// Whether subscribed to updates
    pub subscribed: RwSignal<bool>,
    /// Loading state
    pub loading: RwSignal<bool>,
    /// Watch expressions
    pub watches: RwSignal<Vec<WatchDto>>,
}

impl InspectorState {
    pub fn new() -> Self {
        Self {
            selected_type: RwSignal::new(None),
            entities: RwSignal::new(vec![]),
            total_count: RwSignal::new(0),
            offset: RwSignal::new(0),
            page_size: RwSignal::new(50),
            selected_entity: RwSignal::new(None),
            entity_detail: RwSignal::new(None),
            changes: RwSignal::new(vec![]),
            current_tick: RwSignal::new(0),
            subscribed: RwSignal::new(false),
            loading: RwSignal::new(false),
            watches: RwSignal::new(vec![]),
        }
    }

    /// Handle state snapshot
    pub fn on_snapshot(
        &self,
        tick: u64,
        _entity_type: EntityType,
        entities: Vec<serde_json::Value>,
        total_count: usize,
    ) {
        // Limit entity storage to prevent unbounded memory growth
        let entities = if entities.len() > MAX_ENTITIES {
            tracing::warn!(
                "[inspector] Truncating entity list from {} to {} items",
                entities.len(),
                MAX_ENTITIES
            );
            entities.into_iter().take(MAX_ENTITIES).collect()
        } else {
            entities
        };

        self.entities.set(entities);
        self.total_count.set(total_count);
        self.current_tick.set(tick);
        self.loading.set(false);
    }

    /// Handle state update
    pub fn on_state_update(&self, tick: u64, _entity_type: EntityType, changes: Vec<StateChangeDto>) {
        self.current_tick.set(tick);

        // Prepend new changes, keep max MAX_CHANGES
        self.changes.update(|c| {
            for change in changes.into_iter().rev() {
                c.insert(0, change);
            }
            c.truncate(MAX_CHANGES);
        });
    }

    /// Handle subscription confirmation
    pub fn on_subscribed(&self, _entity_types: Vec<EntityType>) {
        self.subscribed.set(true);
    }

    /// Handle unsubscribe confirmation
    pub fn on_unsubscribed(&self) {
        self.subscribed.set(false);
        self.changes.set(vec![]);
    }

    /// Handle watch list
    pub fn on_watch_list(&self, watches: Vec<WatchDto>) {
        self.watches.set(watches);
    }

    /// Handle watch created
    pub fn on_watch_created(&self, watch: WatchDto) {
        self.watches.update(|w| w.push(watch));
    }

    /// Handle watch removed
    pub fn on_watch_removed(&self, watch_id: Uuid) {
        self.watches.update(|w| w.retain(|watch| watch.id != watch_id));
    }

    /// Handle watch value update
    pub fn on_watch_value(&self, watch_id: Uuid, _tick: u64, value: serde_json::Value, error: Option<String>) {
        self.watches.update(|watches| {
            if let Some(watch) = watches.iter_mut().find(|w| w.id == watch_id)
                && error.is_none()
            {
                watch.last_value = Some(value);
            }
        });
    }

    /// Select entity type and request snapshot
    pub fn select_type(&self, entity_type: EntityType) {
        self.selected_type.set(Some(entity_type));
        self.offset.set(0);
        self.loading.set(true);
    }

    /// Clear selection
    pub fn clear(&self) {
        self.selected_type.set(None);
        self.entities.set(vec![]);
        self.selected_entity.set(None);
        self.entity_detail.set(None);
    }
}

impl Default for InspectorState {
    fn default() -> Self {
        Self::new()
    }
}
