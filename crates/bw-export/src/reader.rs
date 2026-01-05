//! Export reader - loads and inspects export archives

use crate::{
    error::{ExportError, Result},
    manifest::{EntryType, ExportEntry, ExportManifest, EXPORT_FORMAT_VERSION},
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

/// Export archive reader
pub struct ExportReader<R: Read + Seek> {
    archive: ZipArchive<R>,
    manifest: ExportManifest,
    entries: Vec<ExportEntry>,
    entries_by_path: HashMap<String, usize>,
}

impl ExportReader<File> {
    /// Open an export archive from a file path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        Self::new(file)
    }
}

impl<R: Read + Seek> ExportReader<R> {
    /// Create a new export reader from a reader
    pub fn new(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)?;

        // Read manifest
        let manifest: ExportManifest = {
            let mut file = archive.by_name("manifest.cbor")?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            ciborium::from_reader(&data[..])
                .map_err(|e| ExportError::Deserialization(e.to_string()))?
        };

        // Validate version
        if manifest.version > EXPORT_FORMAT_VERSION {
            return Err(ExportError::UnsupportedVersion {
                found: manifest.version,
                max_supported: EXPORT_FORMAT_VERSION,
            });
        }

        // Read entries index
        let entries: Vec<ExportEntry> = {
            let mut file = archive.by_name("entries.cbor")?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            ciborium::from_reader(&data[..])
                .map_err(|e| ExportError::Deserialization(e.to_string()))?
        };

        // Build path index
        let entries_by_path: HashMap<String, usize> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.path.clone(), i))
            .collect();

        Ok(Self {
            archive,
            manifest,
            entries,
            entries_by_path,
        })
    }

    /// Get the export manifest
    pub fn manifest(&self) -> &ExportManifest {
        &self.manifest
    }

    /// Get all entries
    pub fn entries(&self) -> &[ExportEntry] {
        &self.entries
    }

    /// Check if the archive contains an entry
    pub fn has_entry(&self, path: &str) -> bool {
        self.entries_by_path.contains_key(path)
    }

    /// Get an entry by path
    pub fn get_entry(&self, path: &str) -> Option<&ExportEntry> {
        self.entries_by_path.get(path).map(|&i| &self.entries[i])
    }

    /// Read raw bytes from an entry
    pub fn read_raw(&mut self, path: &str) -> Result<Vec<u8>> {
        // Get expected checksum before borrowing archive mutably
        let expected_checksum = self.get_entry(path).map(|e| e.checksum);

        let mut file = self.archive.by_name(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        drop(file); // Explicitly drop to end mutable borrow

        // Verify checksum if entry exists
        if let Some(expected) = expected_checksum {
            let checksum = crc32fast::hash(&data);
            if checksum != expected {
                return Err(ExportError::ChecksumMismatch {
                    path: path.to_string(),
                    expected,
                    found: checksum,
                });
            }
        }

        Ok(data)
    }

    /// Read and deserialize CBOR data from an entry
    pub fn read_cbor<T: DeserializeOwned>(&mut self, path: &str) -> Result<T> {
        let data = self.read_raw(path)?;
        ciborium::from_reader(&data[..])
            .map_err(|e| ExportError::Deserialization(e.to_string()))
    }

    /// Read entities of a specific type
    pub fn read_entities<T: DeserializeOwned>(&mut self, entity_type: &str) -> Result<Vec<T>> {
        let path = format!("entities/{}.cbor", entity_type);
        if !self.has_entry(&path) {
            return Ok(Vec::new());
        }
        self.read_cbor(&path)
    }

    /// Read a script file
    pub fn read_script(&mut self, script_path: &str) -> Result<String> {
        self.read_text_file(&format!("scripts/{}", script_path))
    }

    /// Read a definition file
    pub fn read_definition(&mut self, def_path: &str) -> Result<String> {
        self.read_text_file(&format!("definitions/{}", def_path))
    }

    /// Internal helper to read a text file from the archive
    fn read_text_file(&mut self, archive_path: &str) -> Result<String> {
        let data = self.read_raw(archive_path)?;
        String::from_utf8(data).map_err(|e| ExportError::Utf8 {
            path: archive_path.to_string(),
            source: e,
        })
    }

    /// Read the simulation config
    pub fn read_config<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.read_cbor("config.cbor")
    }

    /// Read the SQLite database bytes
    pub fn read_sqlite(&mut self) -> Result<Vec<u8>> {
        self.read_raw("state.sqlite")
    }

    /// List all script paths in the archive
    pub fn list_scripts(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| matches!(e.entry_type, EntryType::Script))
            .map(|e| e.path.strip_prefix("scripts/").unwrap_or(&e.path))
            .collect()
    }

    /// List all definition paths in the archive
    pub fn list_definitions(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| matches!(e.entry_type, EntryType::Definition))
            .map(|e| e.path.strip_prefix("definitions/").unwrap_or(&e.path))
            .collect()
    }

    /// List all entity types in the archive
    pub fn list_entity_types(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter_map(|e| match &e.entry_type {
                EntryType::Entities { entity_type, .. } => Some(entity_type.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Get entity count for a type
    pub fn entity_count(&self, entity_type: &str) -> usize {
        self.manifest
            .contents
            .entity_counts
            .get(entity_type)
            .copied()
            .unwrap_or(0)
    }

    /// Get total entity count across all types
    pub fn total_entity_count(&self) -> usize {
        self.manifest.contents.entity_counts.values().sum()
    }
}

/// Summary information about an export (without reading all data)
#[derive(Debug, Clone)]
pub struct ExportSummary {
    pub name: String,
    pub version: u32,
    pub created_at: u64,
    pub tick: u64,
    pub entity_types: Vec<String>,
    pub entity_counts: HashMap<String, usize>,
    pub total_entities: usize,
    pub has_sqlite: bool,
    pub has_scripts: bool,
    pub has_definitions: bool,
    pub has_config: bool,
    pub script_count: usize,
    pub definition_count: usize,
    pub uncompressed_size: u64,
    pub notes: Option<String>,
}

impl<R: Read + Seek> From<&ExportReader<R>> for ExportSummary {
    fn from(reader: &ExportReader<R>) -> Self {
        let m = &reader.manifest;
        let c = &m.contents;

        Self {
            name: m.name.clone(),
            version: m.version,
            created_at: m.created_at,
            tick: m.tick,
            entity_types: c.entity_counts.keys().cloned().collect(),
            entity_counts: c.entity_counts.clone(),
            total_entities: c.entity_counts.values().sum(),
            has_sqlite: c.has_sqlite,
            has_scripts: c.has_scripts,
            has_definitions: c.has_definitions,
            has_config: c.has_config,
            script_count: c.script_count,
            definition_count: c.definition_count,
            uncompressed_size: c.uncompressed_size,
            notes: m.notes.clone(),
        }
    }
}
