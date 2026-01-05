//! Staged changes state management

use leptos::prelude::*;
use uuid::Uuid;

use bw_shared::{ChangePreview, StagedChange, ValidationError};

/// A staged change entry with metadata
#[derive(Clone, Debug)]
pub struct StagedChangeEntry {
    /// Local tracking ID
    pub id: Uuid,
    /// The actual change
    pub change: StagedChange,
    /// When this was staged
    pub created_at: f64,
    /// Preview from server (if fetched)
    pub preview: Option<ChangePreview>,
    /// Whether validation failed
    pub has_error: bool,
    /// Error message if validation failed
    pub error_message: Option<String>,
}

impl StagedChangeEntry {
    pub fn new(change: StagedChange) -> Self {
        Self {
            id: Uuid::new_v4(),
            change,
            created_at: js_sys::Date::now(),
            preview: None,
            has_error: false,
            error_message: None,
        }
    }

    /// Get a human-readable description of this change
    pub fn description(&self) -> String {
        match &self.change {
            StagedChange::ScriptUpdate { path, .. } => format!("Script: {}", path),
            StagedChange::SimConfigUpdate { section, .. } => format!("Config: {:?}", section),
            StagedChange::EntityUpdate { entity_type, id, .. } => {
                format!("{:?}: {}", entity_type, id)
            }
            StagedChange::EntitySpawn { entity_type, .. } => format!("Spawn {:?}", entity_type),
            StagedChange::EntityDelete { entity_type, id } => {
                format!("Delete {:?}: {}", entity_type, id)
            }
            StagedChange::SectorUpdate { id, .. } => format!("Sector: {}", id),
            StagedChange::LocationAdd { sector_id, .. } => {
                format!("Add location to sector {}", sector_id)
            }
            StagedChange::LocationRemove { location_id, .. } => {
                format!("Remove location {}", location_id)
            }
        }
    }
}

/// Reactive state for staged changes
#[derive(Clone, Copy)]
pub struct StagedChangesState {
    /// All staged changes
    pub changes: RwSignal<Vec<StagedChangeEntry>>,
    /// Whether preview is being fetched
    pub preview_loading: RwSignal<bool>,
    /// Whether commit is in progress
    pub commit_loading: RwSignal<bool>,
    /// Last commit result message
    pub last_result: RwSignal<Option<String>>,
}

impl StagedChangesState {
    pub fn new() -> Self {
        Self {
            changes: RwSignal::new(vec![]),
            preview_loading: RwSignal::new(false),
            commit_loading: RwSignal::new(false),
            last_result: RwSignal::new(None),
        }
    }

    /// Add a new staged change
    pub fn add(&self, change: StagedChange) {
        let entry = StagedChangeEntry::new(change);
        self.changes.update(|c| c.push(entry));
    }

    /// Remove a staged change by ID
    pub fn remove(&self, id: Uuid) {
        self.changes.update(|c| c.retain(|e| e.id != id));
    }

    /// Clear all staged changes
    pub fn clear(&self) {
        self.changes.set(vec![]);
        self.last_result.set(None);
    }

    /// Get all changes for committing
    pub fn get_changes(&self) -> Vec<StagedChange> {
        self.changes.get().into_iter().map(|e| e.change).collect()
    }

    /// Update previews from server response
    pub fn update_previews(&self, previews: Vec<ChangePreview>, errors: Vec<ValidationError>) {
        self.changes.update(|changes| {
            for (i, entry) in changes.iter_mut().enumerate() {
                entry.preview = previews.get(i).cloned();
                if let Some(err) = errors.iter().find(|e| e.change_index == i) {
                    entry.has_error = true;
                    entry.error_message = Some(err.message.clone());
                } else {
                    entry.has_error = false;
                    entry.error_message = None;
                }
            }
        });
    }

    /// Check if any changes have validation errors
    pub fn has_errors(&self) -> bool {
        self.changes.get().iter().any(|e| e.has_error)
    }

    /// Get the count of staged changes
    pub fn count(&self) -> usize {
        self.changes.get().len()
    }
}

impl Default for StagedChangesState {
    fn default() -> Self {
        Self::new()
    }
}
