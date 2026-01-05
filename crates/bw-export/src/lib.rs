//! Export format library for Blackwing game state
//!
//! This crate provides functionality to export and import game state in a
//! portable archive format. Exports are stored as ZIP files containing:
//!
//! - `manifest.cbor` - Export metadata and summary
//! - `entries.cbor` - Index of all archive entries
//! - `entities/*.cbor` - Entity data by type (ships, players, etc.)
//! - `scripts/*.rhai` - Script files (optional)
//! - `definitions/*.rhai` - Archetype definitions (optional)
//! - `config.cbor` - Simulation configuration (optional)
//! - `state.sqlite` - SQLite database snapshot (optional)
//!
//! # Example
//!
//! ```no_run
//! use bw_export::{ExportWriter, ExportReader, ExportConfig, ExportSource};
//!
//! // Create an export
//! let config = ExportConfig::new("my-export").with_sqlite();
//! let mut writer = ExportWriter::create("export.bwx", &config, ExportSource::Live, 12345)?;
//!
//! // Write entities (ships, players, etc.)
//! writer.write_entities("ships", &ships)?;
//! writer.write_entities("players", &players)?;
//!
//! // Finalize
//! writer.finish()?;
//!
//! // Read an export
//! let mut reader = ExportReader::open("export.bwx")?;
//! let manifest = reader.manifest();
//! println!("Export: {} (tick {})", manifest.name, manifest.tick);
//!
//! let ships: Vec<Ship> = reader.read_entities("ships")?;
//! # Ok::<(), bw_export::ExportError>(())
//! ```

pub mod error;
pub mod manifest;
pub mod reader;
pub mod writer;

pub use error::{ExportError, Result};
pub use manifest::{
    EntryType, ExportConfig, ExportContents, ExportEntry, ExportManifest, ExportSource,
    EXPORT_FORMAT_VERSION,
};
pub use reader::{ExportReader, ExportSummary};
pub use writer::ExportWriter;

/// File extension for export archives
pub const EXPORT_EXTENSION: &str = "bwx";

/// Generate a default export filename
pub fn generate_export_filename(name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}-{}.{}", name, timestamp, EXPORT_EXTENSION)
}
