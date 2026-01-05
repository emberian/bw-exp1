//! Export state for managing export operations

use leptos::prelude::*;
use bw_shared::dto::{ExportSummaryDto, ExportStatus};
use uuid::Uuid;

/// State for export operations
#[derive(Clone, Copy)]
pub struct ExportState {
    /// Available exports
    pub exports: RwSignal<Vec<ExportSummaryDto>>,
    /// Currently creating export ID
    pub creating_id: RwSignal<Option<Uuid>>,
    /// Creating export name
    pub creating_name: RwSignal<Option<String>>,
    /// Progress of current export (0-100)
    pub progress: RwSignal<u8>,
    /// Progress phase description
    pub progress_phase: RwSignal<String>,
    /// Loading exports list
    pub loading: RwSignal<bool>,
    /// Error message
    pub error: RwSignal<Option<String>>,
    /// Download URL for completed export
    pub download_url: RwSignal<Option<String>>,
}

impl ExportState {
    pub fn new() -> Self {
        Self {
            exports: RwSignal::new(vec![]),
            creating_id: RwSignal::new(None),
            creating_name: RwSignal::new(None),
            progress: RwSignal::new(0),
            progress_phase: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
            download_url: RwSignal::new(None),
        }
    }

    /// Handle export created
    pub fn on_export_created(&self, export_id: Uuid, name: String) {
        self.creating_id.set(Some(export_id));
        self.creating_name.set(Some(name));
        self.progress.set(0);
        self.progress_phase.set("Starting...".to_string());
        self.error.set(None);
    }

    /// Handle export progress
    pub fn on_progress(&self, _export_id: Uuid, phase: String, percent: u8) {
        self.progress_phase.set(phase);
        self.progress.set(percent);
    }

    /// Handle export completed
    pub fn on_completed(&self, export_id: Uuid, size_bytes: u64, download_url: String) {
        // Update export in list
        self.exports.update(|exports| {
            if let Some(export) = exports.iter_mut().find(|e| e.id == export_id) {
                export.status = ExportStatus::Completed;
                export.size_bytes = size_bytes;
            }
        });

        self.creating_id.set(None);
        self.creating_name.set(None);
        self.progress.set(100);
        self.progress_phase.set("Completed".to_string());
        self.download_url.set(Some(download_url));
    }

    /// Handle export failed
    pub fn on_failed(&self, export_id: Uuid, error: String) {
        // Update export in list
        self.exports.update(|exports| {
            if let Some(export) = exports.iter_mut().find(|e| e.id == export_id) {
                export.status = ExportStatus::Failed;
            }
        });

        self.creating_id.set(None);
        self.creating_name.set(None);
        self.progress.set(0);
        self.progress_phase.set(String::new());
        self.error.set(Some(error));
    }

    /// Handle export list
    pub fn on_export_list(&self, exports: Vec<ExportSummaryDto>) {
        self.exports.set(exports);
        self.loading.set(false);
    }

    /// Handle export deleted
    pub fn on_deleted(&self, export_id: Uuid) {
        self.exports.update(|exports| {
            exports.retain(|e| e.id != export_id);
        });
    }

    /// Handle export status update
    pub fn on_status(&self, export: ExportSummaryDto) {
        self.exports.update(|exports| {
            if let Some(existing) = exports.iter_mut().find(|e| e.id == export.id) {
                *existing = export;
            } else {
                exports.push(export);
            }
        });
    }

    /// Handle download URL
    pub fn on_download_url(&self, _export_id: Uuid, url: String, _expires_at: u64) {
        self.download_url.set(Some(url));
    }

    /// Check if an export is in progress
    pub fn is_exporting(&self) -> bool {
        self.creating_id.get().is_some()
    }

    /// Clear download URL
    pub fn clear_download(&self) {
        self.download_url.set(None);
    }

    /// Clear error
    pub fn clear_error(&self) {
        self.error.set(None);
    }
}

impl Default for ExportState {
    fn default() -> Self {
        Self::new()
    }
}
