//! Export error types

use thiserror::Error;

/// Export-related errors
#[derive(Debug, Error)]
pub enum ExportError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// ZIP archive error
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Unsupported export format version
    #[error("Unsupported export version {found}, max supported is {max_supported}")]
    UnsupportedVersion { found: u32, max_supported: u32 },

    /// Checksum mismatch
    #[error("Checksum mismatch for {path}: expected {expected:08x}, found {found:08x}")]
    ChecksumMismatch {
        path: String,
        expected: u32,
        found: u32,
    },

    /// Entry not found
    #[error("Entry not found: {0}")]
    EntryNotFound(String),

    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Invalid export
    #[error("Invalid export: {0}")]
    Invalid(String),

    /// UTF-8 decode error
    #[error("Invalid UTF-8 in {path}: {source}")]
    Utf8 {
        path: String,
        source: std::string::FromUtf8Error,
    },
}

/// Result type for export operations
pub type Result<T> = std::result::Result<T, ExportError>;
