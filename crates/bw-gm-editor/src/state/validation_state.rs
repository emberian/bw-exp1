//! Validation state for script/definition validation

use leptos::prelude::*;
use bw_shared::dto::ValidationIssueDto;

/// State for validation results
#[derive(Clone, Copy)]
pub struct ValidationState {
    /// Current script path being validated
    pub current_path: RwSignal<Option<String>>,
    /// Validation errors
    pub errors: RwSignal<Vec<ValidationIssueDto>>,
    /// Validation warnings
    pub warnings: RwSignal<Vec<ValidationIssueDto>>,
    /// Whether validation passed
    pub is_valid: RwSignal<bool>,
    /// Whether validation is in progress
    pub validating: RwSignal<bool>,
}

impl ValidationState {
    pub fn new() -> Self {
        Self {
            current_path: RwSignal::new(None),
            errors: RwSignal::new(vec![]),
            warnings: RwSignal::new(vec![]),
            is_valid: RwSignal::new(true),
            validating: RwSignal::new(false),
        }
    }

    /// Handle validation result
    pub fn on_validation_result(
        &self,
        path: String,
        errors: Vec<ValidationIssueDto>,
        warnings: Vec<ValidationIssueDto>,
        is_valid: bool,
    ) {
        self.current_path.set(Some(path));
        self.errors.set(errors);
        self.warnings.set(warnings);
        self.is_valid.set(is_valid);
        self.validating.set(false);
    }

    /// Clear validation state
    pub fn clear(&self) {
        self.current_path.set(None);
        self.errors.set(vec![]);
        self.warnings.set(vec![]);
        self.is_valid.set(true);
        self.validating.set(false);
    }

    /// Get all issues (errors + warnings) sorted by line
    pub fn all_issues(&self) -> Vec<ValidationIssueDto> {
        let mut issues: Vec<_> = self.errors.get().into_iter()
            .chain(self.warnings.get())
            .collect();
        issues.sort_by_key(|i| (i.line, i.column));
        issues
    }
}

impl Default for ValidationState {
    fn default() -> Self {
        Self::new()
    }
}
