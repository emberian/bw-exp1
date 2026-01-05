//! GM Editor state management

mod staged_changes;

pub use staged_changes::*;

use leptos::prelude::*;
use uuid::Uuid;

use bw_shared::{ScriptFileInfo, EntitySummary, SectorSummaryAdmin};

/// Main GM Editor state
#[derive(Clone, Copy)]
pub struct GMEditorState {
    /// List of script files
    pub scripts: RwSignal<Vec<ScriptFileInfo>>,
    /// Currently selected script path
    pub selected_script: RwSignal<Option<String>>,
    /// Content of selected script
    pub script_content: RwSignal<String>,
    /// Original content (for detecting changes)
    pub script_original: RwSignal<String>,

    /// Current simulation config as JSON
    pub sim_config: RwSignal<Option<serde_json::Value>>,

    /// Entity list (current query results)
    pub entities: RwSignal<Vec<EntitySummary>>,
    /// Selected entity ID
    pub selected_entity: RwSignal<Option<Uuid>>,
    /// Selected entity details
    pub entity_details: RwSignal<Option<serde_json::Value>>,

    /// Sector list
    pub sectors: RwSignal<Vec<SectorSummaryAdmin>>,

    /// Loading states
    pub loading_scripts: RwSignal<bool>,
    pub loading_config: RwSignal<bool>,
    pub loading_entities: RwSignal<bool>,

    /// Error message
    pub error: RwSignal<Option<String>>,
}

impl GMEditorState {
    pub fn new() -> Self {
        Self {
            scripts: RwSignal::new(vec![]),
            selected_script: RwSignal::new(None),
            script_content: RwSignal::new(String::new()),
            script_original: RwSignal::new(String::new()),

            sim_config: RwSignal::new(None),

            entities: RwSignal::new(vec![]),
            selected_entity: RwSignal::new(None),
            entity_details: RwSignal::new(None),

            sectors: RwSignal::new(vec![]),

            loading_scripts: RwSignal::new(false),
            loading_config: RwSignal::new(false),
            loading_entities: RwSignal::new(false),

            error: RwSignal::new(None),
        }
    }

    /// Check if the current script has unsaved changes
    pub fn script_is_modified(&self) -> bool {
        self.script_content.get() != self.script_original.get()
    }

    /// Clear error after a delay
    pub fn clear_error_delayed(&self) {
        let error = self.error;
        gloo_timers::callback::Timeout::new(5000, move || {
            error.set(None);
        })
        .forget();
    }
}

impl Default for GMEditorState {
    fn default() -> Self {
        Self::new()
    }
}
