//! Export writer - creates export archives

use crate::{
    error::{ExportError, Result},
    manifest::{
        EntryType, ExportConfig, ExportContents, ExportEntry, ExportManifest, ExportSource,
    },
};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Seek, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Export archive writer
pub struct ExportWriter<W: Write + Seek> {
    zip: ZipWriter<W>,
    manifest: ExportManifest,
    entries: Vec<ExportEntry>,
    options: SimpleFileOptions,
}

impl ExportWriter<File> {
    /// Create a new export archive at the given path
    pub fn create(path: impl AsRef<Path>, config: &ExportConfig, source: ExportSource, tick: u64) -> Result<Self> {
        let file = File::create(path.as_ref())?;
        Self::new(file, config, source, tick)
    }
}

impl<W: Write + Seek> ExportWriter<W> {
    /// Create a new export writer wrapping the given writer
    pub fn new(writer: W, config: &ExportConfig, source: ExportSource, tick: u64) -> Result<Self> {
        let mut manifest = ExportManifest::new(config.name.clone(), source, tick);
        if let Some(notes) = &config.notes {
            manifest = manifest.with_notes(notes.clone());
        }

        let zip = ZipWriter::new(writer);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6));

        Ok(Self {
            zip,
            manifest,
            entries: Vec::new(),
            options,
        })
    }

    /// Write entities to the archive
    pub fn write_entities<T: Serialize>(
        &mut self,
        entity_type: &str,
        entities: &[T],
    ) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        let path = format!("entities/{}.cbor", entity_type);

        // Serialize to CBOR
        let mut data = Vec::new();
        ciborium::into_writer(entities, &mut data)
            .map_err(|e| ExportError::Serialization(e.to_string()))?;

        let size = data.len() as u64;
        let checksum = crc32fast::hash(&data);

        // Write to archive
        self.zip.start_file(&path, self.options)?;
        self.zip.write_all(&data)?;

        // Track entry
        self.entries.push(ExportEntry {
            path: path.clone(),
            entry_type: EntryType::Entities {
                entity_type: entity_type.to_string(),
                count: entities.len(),
            },
            size,
            checksum,
        });

        // Update manifest
        self.manifest
            .contents
            .entity_counts
            .insert(entity_type.to_string(), entities.len());
        self.manifest.contents.uncompressed_size += size;

        Ok(())
    }

    /// Write a script file to the archive
    pub fn write_script(&mut self, script_path: &str, content: &str) -> Result<()> {
        self.write_text_file(
            &format!("scripts/{}", script_path),
            content,
            EntryType::Script,
            |contents| {
                contents.has_scripts = true;
                contents.script_count += 1;
            },
        )
    }

    /// Write a definition file to the archive
    pub fn write_definition(&mut self, def_path: &str, content: &str) -> Result<()> {
        self.write_text_file(
            &format!("definitions/{}", def_path),
            content,
            EntryType::Definition,
            |contents| {
                contents.has_definitions = true;
                contents.definition_count += 1;
            },
        )
    }

    /// Internal helper to write a text file to the archive
    fn write_text_file(
        &mut self,
        archive_path: &str,
        content: &str,
        entry_type: EntryType,
        update_contents: impl FnOnce(&mut ExportContents),
    ) -> Result<()> {
        let data = content.as_bytes();
        let size = data.len() as u64;
        let checksum = crc32fast::hash(data);

        self.zip.start_file(archive_path, self.options)?;
        self.zip.write_all(data)?;

        self.entries.push(ExportEntry {
            path: archive_path.to_string(),
            entry_type,
            size,
            checksum,
        });

        update_contents(&mut self.manifest.contents);
        self.manifest.contents.uncompressed_size += size;

        Ok(())
    }

    /// Write simulation config to the archive
    pub fn write_config<T: Serialize>(&mut self, config: &T) -> Result<()> {
        let path = "config.cbor";

        let mut data = Vec::new();
        ciborium::into_writer(config, &mut data)
            .map_err(|e| ExportError::Serialization(e.to_string()))?;

        let size = data.len() as u64;
        let checksum = crc32fast::hash(&data);

        self.zip.start_file(path, self.options)?;
        self.zip.write_all(&data)?;

        self.entries.push(ExportEntry {
            path: path.to_string(),
            entry_type: EntryType::Config,
            size,
            checksum,
        });

        self.manifest.contents.has_config = true;
        self.manifest.contents.uncompressed_size += size;

        Ok(())
    }

    /// Write raw SQLite database bytes to the archive
    pub fn write_sqlite(&mut self, data: &[u8]) -> Result<()> {
        let path = "state.sqlite";
        let size = data.len() as u64;
        let checksum = crc32fast::hash(data);

        // Use stored (no compression) for SQLite as it's already compact
        let sqlite_options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        self.zip.start_file(path, sqlite_options)?;
        self.zip.write_all(data)?;

        self.entries.push(ExportEntry {
            path: path.to_string(),
            entry_type: EntryType::Sqlite,
            size,
            checksum,
        });

        self.manifest.contents.has_sqlite = true;
        self.manifest.contents.uncompressed_size += size;

        Ok(())
    }

    /// Write arbitrary CBOR data to the archive
    pub fn write_cbor<T: Serialize>(&mut self, path: &str, data: &T) -> Result<()> {
        let mut bytes = Vec::new();
        ciborium::into_writer(data, &mut bytes)
            .map_err(|e| ExportError::Serialization(e.to_string()))?;

        let size = bytes.len() as u64;
        let checksum = crc32fast::hash(&bytes);

        self.zip.start_file(path, self.options)?;
        self.zip.write_all(&bytes)?;

        self.entries.push(ExportEntry {
            path: path.to_string(),
            entry_type: EntryType::Other,
            size,
            checksum,
        });

        self.manifest.contents.uncompressed_size += size;

        Ok(())
    }

    /// Finalize the export and write the manifest
    pub fn finish(mut self) -> Result<W> {
        // Write manifest as the last entry
        let mut manifest_data = Vec::new();
        ciborium::into_writer(&self.manifest, &mut manifest_data)
            .map_err(|e| ExportError::Serialization(e.to_string()))?;

        self.zip.start_file("manifest.cbor", self.options)?;
        self.zip.write_all(&manifest_data)?;

        // Write entries index
        let mut entries_data = Vec::new();
        ciborium::into_writer(&self.entries, &mut entries_data)
            .map_err(|e| ExportError::Serialization(e.to_string()))?;

        self.zip.start_file("entries.cbor", self.options)?;
        self.zip.write_all(&entries_data)?;

        Ok(self.zip.finish()?)
    }

    /// Get a reference to the manifest being built
    pub fn manifest(&self) -> &ExportManifest {
        &self.manifest
    }

    /// Get the current entity counts
    pub fn entity_counts(&self) -> &HashMap<String, usize> {
        &self.manifest.contents.entity_counts
    }
}
