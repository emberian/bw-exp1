//! Schema state for archetype browser

use leptos::prelude::*;
use bw_shared::dto::{ArchetypeSchemaDto, FieldSchemaDto, ActionSchemaDto};
use std::collections::HashMap;

/// State for schema browsing
#[derive(Clone, Copy)]
pub struct SchemaState {
    /// Available archetype schemas
    pub schemas: RwSignal<Vec<ArchetypeSchemaDto>>,
    /// Currently selected archetype type
    pub selected_type: RwSignal<Option<String>>,
    /// Fields for the selected archetype
    pub fields: RwSignal<Vec<FieldSchemaDto>>,
    /// Action schemas
    pub actions: RwSignal<HashMap<String, ActionSchemaDto>>,
    /// Loading state
    pub loading: RwSignal<bool>,
    /// Error message
    pub error: RwSignal<Option<String>>,
}

impl SchemaState {
    pub fn new() -> Self {
        Self {
            schemas: RwSignal::new(vec![]),
            selected_type: RwSignal::new(None),
            fields: RwSignal::new(vec![]),
            actions: RwSignal::new(HashMap::new()),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
        }
    }

    /// Handle receiving archetype schemas
    pub fn on_schemas(&self, schemas: Vec<ArchetypeSchemaDto>) {
        self.schemas.set(schemas);
        self.loading.set(false);
    }

    /// Handle receiving schema detail
    pub fn on_schema_detail(&self, archetype_type: String, _name: String, fields: Vec<FieldSchemaDto>) {
        self.selected_type.set(Some(archetype_type));
        self.fields.set(fields);
        self.loading.set(false);
    }

    /// Handle receiving action schemas
    pub fn on_action_schemas(&self, actions: HashMap<String, ActionSchemaDto>) {
        self.actions.set(actions);
    }

    /// Clear selection
    pub fn clear_selection(&self) {
        self.selected_type.set(None);
        self.fields.set(vec![]);
    }
}

impl Default for SchemaState {
    fn default() -> Self {
        Self::new()
    }
}
