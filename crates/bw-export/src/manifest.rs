//! Export manifest types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Version of the export format
pub const EXPORT_FORMAT_VERSION: u32 = 1;

/// Export manifest stored at the root of the export archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    /// Format version
    pub version: u32,
    /// Export name
    pub name: String,
    /// Creation timestamp (Unix epoch seconds)
    pub created_at: u64,
    /// Source of the export
    pub source: ExportSource,
    /// Export contents summary
    pub contents: ExportContents,
    /// Game tick at time of export
    pub tick: u64,
    /// Optional notes/description
    pub notes: Option<String>,
}

impl ExportManifest {
    /// Create a new export manifest
    pub fn new(name: String, source: ExportSource, tick: u64) -> Self {
        Self {
            version: EXPORT_FORMAT_VERSION,
            name,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            source,
            contents: ExportContents::default(),
            tick,
            notes: None,
        }
    }

    /// Add notes to the manifest
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Source of the export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportSource {
    /// Live game server state
    Live,
    /// Playtest instance state
    Playtest { id: Uuid, name: String },
    /// Previous export (for diff/merge operations)
    Export { id: Uuid, path: String },
}

/// Summary of export contents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportContents {
    /// Whether SQLite snapshot is included
    pub has_sqlite: bool,
    /// Whether scripts are included
    pub has_scripts: bool,
    /// Whether archetype definitions are included
    pub has_definitions: bool,
    /// Whether simulation config is included
    pub has_config: bool,
    /// Entity counts by type
    pub entity_counts: HashMap<String, usize>,
    /// Script file count
    pub script_count: usize,
    /// Definition file count
    pub definition_count: usize,
    /// Total uncompressed size in bytes
    pub uncompressed_size: u64,
}

/// Entry in the export archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    /// Entry path within archive
    pub path: String,
    /// Entry type
    pub entry_type: EntryType,
    /// Uncompressed size
    pub size: u64,
    /// Checksum (CRC32)
    pub checksum: u32,
}

/// Type of export entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryType {
    /// The manifest itself
    Manifest,
    /// Entity data (CBOR)
    Entities { entity_type: String, count: usize },
    /// Script file (Rhai)
    Script,
    /// Definition file (Rhai)
    Definition,
    /// Simulation config
    Config,
    /// SQLite database snapshot
    Sqlite,
    /// Other/unknown
    Other,
}

/// Export configuration options
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Export name
    pub name: String,
    /// Sectors to include (empty = all)
    pub sectors: Vec<Uuid>,
    /// Include player entities
    pub include_players: bool,
    /// Include NPC entities
    pub include_npcs: bool,
    /// Include mission entities
    pub include_missions: bool,
    /// Include script files
    pub include_scripts: bool,
    /// Include archetype definitions
    pub include_definitions: bool,
    /// Include simulation config
    pub include_config: bool,
    /// Include SQLite snapshot
    pub include_sqlite: bool,
    /// Optional notes
    pub notes: Option<String>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            name: String::from("export"),
            sectors: Vec::new(),
            include_players: true,
            include_npcs: true,
            include_missions: true,
            include_scripts: true,
            include_definitions: true,
            include_config: true,
            include_sqlite: false,
            notes: None,
        }
    }
}

impl ExportConfig {
    /// Create a new export config with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Enable SQLite snapshot
    pub fn with_sqlite(mut self) -> Self {
        self.include_sqlite = true;
        self
    }

    /// Set sectors to include
    pub fn with_sectors(mut self, sectors: Vec<Uuid>) -> Self {
        self.sectors = sectors;
        self
    }

    /// Add notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Minimal export (entities only)
    pub fn minimal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            include_scripts: false,
            include_definitions: false,
            include_config: false,
            ..Default::default()
        }
    }

    /// Full export (everything including SQLite)
    pub fn full(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            include_sqlite: true,
            ..Default::default()
        }
    }
}
