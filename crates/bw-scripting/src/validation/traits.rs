//! Shared traits for script validators.
//!
//! Provides common abstractions for validating different script types.

use std::path::Path;

/// A single validation error.
pub trait ValidationError: Clone {
    /// Error code (e.g., "E001", "E500").
    fn code(&self) -> &str;
    /// Human-readable error message.
    fn message(&self) -> &str;
    /// Source line number if available.
    fn line(&self) -> Option<usize>;
}

/// A single validation warning.
pub trait ValidationWarning: Clone {
    /// Warning code (e.g., "W001", "W500").
    fn code(&self) -> &str;
    /// Human-readable warning message.
    fn message(&self) -> &str;
    /// Source line number if available.
    fn line(&self) -> Option<usize>;
}

/// Result of validating a single script file.
pub trait ScriptValidation {
    type Error: ValidationError;
    type Warning: ValidationWarning;

    /// Path to the validated script.
    fn path(&self) -> &str;
    /// Whether validation passed (no errors).
    fn is_valid(&self) -> bool;
    /// Validation errors found.
    fn errors(&self) -> &[Self::Error];
    /// Validation warnings found.
    fn warnings(&self) -> &[Self::Warning];
}

/// Result of validating multiple scripts.
pub trait ValidationReport {
    type Validation: ScriptValidation;

    /// All script validations.
    fn scripts(&self) -> &[Self::Validation];
    /// Total error count across all scripts.
    fn total_errors(&self) -> usize;
    /// Total warning count across all scripts.
    fn total_warnings(&self) -> usize;
    /// Whether all scripts passed validation.
    fn is_valid(&self) -> bool {
        self.total_errors() == 0
    }
    /// Format a human-readable report.
    fn format_report(&self) -> String;
}

/// Trait for script validators.
///
/// Provides a common interface for validating different script types
/// (action scripts, definition scripts, etc.).
pub trait ScriptValidator {
    /// The validation result type for a single file.
    type Validation: ScriptValidation;
    /// The report type for directory validation.
    type Report: ValidationReport<Validation = Self::Validation>;

    /// Validate a single script file.
    fn validate_file(&self, path: &Path) -> Self::Validation;

    /// Validate all scripts in a directory (recursive).
    fn validate_directory(&self, dir: &Path) -> Self::Report;
}

/// Helper for recursively collecting script files.
pub fn collect_rhai_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    collect_rhai_files_recursive(dir, &mut files);
    files
}

fn collect_rhai_files_recursive(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rhai_files_recursive(&path, files);
            } else if path.extension().is_some_and(|e| e == "rhai") {
                files.push(path);
            }
        }
    }
}
