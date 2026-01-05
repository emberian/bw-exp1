//! GM Editor state management

mod debug_state;
mod export_state;
mod inspector_state;
mod schema_state;
mod staged_changes;
mod validation_state;

pub use debug_state::*;
pub use export_state::*;
pub use inspector_state::*;
pub use schema_state::*;
pub use staged_changes::*;
pub use validation_state::*;

use leptos::prelude::*;
use uuid::Uuid;

use bw_shared::{ScriptFileInfo, EntitySummary, SectorSummaryAdmin};

/// A script error entry for display
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)] // Fields read by UI components (not yet implemented)
pub struct ScriptErrorEntry {
    pub script: String,
    pub function: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub tick: u64,
    pub timestamp_ms: u64,
}

/// A notification entry for display
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields read by UI components (not yet implemented)
pub struct NotificationEntry {
    pub message: String,
    pub notification_type: String, // "success", "warning", "error"
    pub timestamp_ms: u64,
}

const MAX_SCRIPT_ERRORS: usize = 100;
const MAX_NOTIFICATIONS: usize = 20;

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

    /// Script errors (newest first)
    pub script_errors: RwSignal<Vec<ScriptErrorEntry>>,
    /// Notifications (newest first)
    pub notifications: RwSignal<Vec<NotificationEntry>>,
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

            script_errors: RwSignal::new(vec![]),
            notifications: RwSignal::new(vec![]),
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

    /// Add a script error to the list
    pub fn add_script_error(
        &self,
        script: String,
        function: String,
        message: String,
        line: usize,
        column: usize,
        tick: u64,
    ) {
        let timestamp_ms = js_sys::Date::now() as u64;
        let entry = ScriptErrorEntry {
            script,
            function,
            message,
            line,
            column,
            tick,
            timestamp_ms,
        };

        self.script_errors.update(|errors| {
            errors.insert(0, entry);
            if errors.len() > MAX_SCRIPT_ERRORS {
                errors.truncate(MAX_SCRIPT_ERRORS);
            }
        });
    }

    /// Add a notification
    pub fn add_notification(&self, message: String, notification_type: String) {
        let timestamp_ms = js_sys::Date::now() as u64;
        let entry = NotificationEntry {
            message,
            notification_type,
            timestamp_ms,
        };

        self.notifications.update(|notifications| {
            notifications.insert(0, entry);
            if notifications.len() > MAX_NOTIFICATIONS {
                notifications.truncate(MAX_NOTIFICATIONS);
            }
        });

        // Auto-clear notifications after delay
        let notifications_signal = self.notifications;
        gloo_timers::callback::Timeout::new(10000, move || {
            notifications_signal.update(|n| {
                // Remove entries older than 10 seconds
                let cutoff = js_sys::Date::now() as u64 - 10000;
                n.retain(|e| e.timestamp_ms > cutoff);
            });
        })
        .forget();
    }

    /// Clear all script errors
    pub fn clear_script_errors(&self) {
        self.script_errors.set(vec![]);
    }

    /// Clear all notifications
    pub fn clear_notifications(&self) {
        self.notifications.set(vec![]);
    }
}

impl Default for GMEditorState {
    fn default() -> Self {
        Self::new()
    }
}
