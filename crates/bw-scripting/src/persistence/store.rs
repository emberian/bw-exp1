//! Script State Storage
//!
//! Defines the trait and implementations for persistent script state.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Errors that can occur during script state operations.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("Failed to serialize state: {0}")]
    Serialization(String),

    #[error("Failed to deserialize state: {0}")]
    Deserialization(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Invalid namespace: {0}")]
    InvalidNamespace(String),
}

/// Stored script state with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptState {
    /// The actual data (serialized Rhai Dynamic).
    pub data: Vec<u8>,
    /// Namespace this state belongs to.
    pub namespace: String,
    /// Key within the namespace.
    pub key: String,
    /// When this state was created.
    pub created_at: u64,
    /// When this state was last updated.
    pub updated_at: u64,
    /// Optional version for conflict detection.
    pub version: u64,
}

impl ScriptState {
    /// Create a new script state entry.
    pub fn new(namespace: &str, key: &str, data: Vec<u8>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            data,
            namespace: namespace.to_string(),
            key: key.to_string(),
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Update the data and bump version.
    pub fn update(&mut self, data: Vec<u8>) {
        self.data = data;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.version += 1;
    }
}

/// Trait for script state storage backends.
pub trait ScriptStateStore: Send + Sync {
    /// Save state to storage.
    fn save(&self, namespace: &str, key: &str, data: &[u8]) -> Result<(), PersistenceError>;

    /// Load state from storage.
    fn load(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, PersistenceError>;

    /// Delete state from storage.
    fn delete(&self, namespace: &str, key: &str) -> Result<bool, PersistenceError>;

    /// List all keys in a namespace.
    fn list_keys(&self, namespace: &str) -> Result<Vec<String>, PersistenceError>;

    /// Clear all state in a namespace.
    fn clear_namespace(&self, namespace: &str) -> Result<usize, PersistenceError>;

    /// Check if a key exists.
    fn exists(&self, namespace: &str, key: &str) -> Result<bool, PersistenceError> {
        Ok(self.load(namespace, key)?.is_some())
    }

    /// Get metadata about a state entry.
    fn get_metadata(&self, namespace: &str, key: &str) -> Result<Option<ScriptState>, PersistenceError>;

    /// Create a checkpoint (snapshot) of current state.
    fn checkpoint(&self, name: &str) -> Result<(), PersistenceError>;

    /// Restore from a checkpoint.
    fn restore_checkpoint(&self, name: &str) -> Result<bool, PersistenceError>;

    /// List available checkpoints.
    fn list_checkpoints(&self) -> Result<Vec<String>, PersistenceError>;

    /// Delete a checkpoint.
    fn delete_checkpoint(&self, name: &str) -> Result<bool, PersistenceError>;
}

/// In-memory script state store.
///
/// Fast but not persistent across restarts. Useful for testing or
/// session-scoped state.
#[derive(Default)]
pub struct InMemoryStore {
    /// Main state storage: namespace -> (key -> state)
    data: RwLock<HashMap<String, HashMap<String, ScriptState>>>,
    /// Checkpoints: checkpoint_name -> snapshot of data
    checkpoints: RwLock<HashMap<String, HashMap<String, HashMap<String, ScriptState>>>>,
}

impl InMemoryStore {
    /// Create a new in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a shareable instance.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Get total number of stored entries.
    pub fn entry_count(&self) -> usize {
        self.data.read()
            .values()
            .map(|ns| ns.len())
            .sum()
    }

    /// Get number of namespaces.
    pub fn namespace_count(&self) -> usize {
        self.data.read().len()
    }
}

impl ScriptStateStore for InMemoryStore {
    fn save(&self, namespace: &str, key: &str, data: &[u8]) -> Result<(), PersistenceError> {
        let mut store = self.data.write();
        let ns = store.entry(namespace.to_string()).or_default();

        if let Some(state) = ns.get_mut(key) {
            state.update(data.to_vec());
        } else {
            ns.insert(key.to_string(), ScriptState::new(namespace, key, data.to_vec()));
        }

        Ok(())
    }

    fn load(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
        let store = self.data.read();
        Ok(store
            .get(namespace)
            .and_then(|ns| ns.get(key))
            .map(|state| state.data.clone()))
    }

    fn delete(&self, namespace: &str, key: &str) -> Result<bool, PersistenceError> {
        let mut store = self.data.write();
        if let Some(ns) = store.get_mut(namespace) {
            return Ok(ns.remove(key).is_some());
        }
        Ok(false)
    }

    fn list_keys(&self, namespace: &str) -> Result<Vec<String>, PersistenceError> {
        let store = self.data.read();
        Ok(store
            .get(namespace)
            .map(|ns| ns.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn clear_namespace(&self, namespace: &str) -> Result<usize, PersistenceError> {
        let mut store = self.data.write();
        if let Some(ns) = store.remove(namespace) {
            return Ok(ns.len());
        }
        Ok(0)
    }

    fn get_metadata(&self, namespace: &str, key: &str) -> Result<Option<ScriptState>, PersistenceError> {
        let store = self.data.read();
        Ok(store
            .get(namespace)
            .and_then(|ns| ns.get(key))
            .cloned())
    }

    fn checkpoint(&self, name: &str) -> Result<(), PersistenceError> {
        let data = self.data.read().clone();
        self.checkpoints.write().insert(name.to_string(), data);
        Ok(())
    }

    fn restore_checkpoint(&self, name: &str) -> Result<bool, PersistenceError> {
        let checkpoints = self.checkpoints.read();
        if let Some(snapshot) = checkpoints.get(name) {
            let mut data = self.data.write();
            *data = snapshot.clone();
            return Ok(true);
        }
        Ok(false)
    }

    fn list_checkpoints(&self) -> Result<Vec<String>, PersistenceError> {
        Ok(self.checkpoints.read().keys().cloned().collect())
    }

    fn delete_checkpoint(&self, name: &str) -> Result<bool, PersistenceError> {
        Ok(self.checkpoints.write().remove(name).is_some())
    }
}

/// File-based script state store.
///
/// Persists state to the filesystem. Each namespace is a directory,
/// and each key is a file within that directory.
pub struct FileStore {
    /// Base directory for storage.
    base_path: PathBuf,
    /// In-memory cache for performance.
    cache: RwLock<HashMap<String, HashMap<String, ScriptState>>>,
    /// Checkpoints directory name.
    checkpoints_dir: String,
}

impl FileStore {
    /// Create a new file store at the given path.
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let base_path = base_path.as_ref().to_path_buf();

        // Ensure base directory exists
        fs::create_dir_all(&base_path)?;

        let store = Self {
            base_path,
            cache: RwLock::new(HashMap::new()),
            checkpoints_dir: "_checkpoints".to_string(),
        };

        // Load existing data into cache
        store.load_all_to_cache()?;

        Ok(store)
    }

    /// Create a shareable instance.
    pub fn shared(base_path: impl AsRef<Path>) -> Result<Arc<Self>, PersistenceError> {
        Ok(Arc::new(Self::new(base_path)?))
    }

    /// Get the file path for a namespace/key pair.
    fn state_path(&self, namespace: &str, key: &str) -> PathBuf {
        self.base_path
            .join(sanitize_filename(namespace))
            .join(format!("{}.json", sanitize_filename(key)))
    }

    /// Get the directory path for a namespace.
    fn namespace_path(&self, namespace: &str) -> PathBuf {
        self.base_path.join(sanitize_filename(namespace))
    }

    /// Get the checkpoints directory.
    fn checkpoints_path(&self) -> PathBuf {
        self.base_path.join(&self.checkpoints_dir)
    }

    /// Load all existing state into the cache.
    fn load_all_to_cache(&self) -> Result<(), PersistenceError> {
        let mut cache = self.cache.write();

        if !self.base_path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let namespace = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip checkpoints directory
                if namespace == self.checkpoints_dir {
                    continue;
                }

                let ns_cache = cache.entry(namespace.clone()).or_default();

                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(contents) = fs::read_to_string(&file_path) {
                            if let Ok(state) = serde_json::from_str::<ScriptState>(&contents) {
                                ns_cache.insert(state.key.clone(), state);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Write a state entry to disk.
    fn write_to_disk(&self, state: &ScriptState) -> Result<(), PersistenceError> {
        let ns_path = self.namespace_path(&state.namespace);
        fs::create_dir_all(&ns_path)?;

        let file_path = self.state_path(&state.namespace, &state.key);
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        fs::write(file_path, json)?;
        Ok(())
    }

    /// Delete a state file from disk.
    fn delete_from_disk(&self, namespace: &str, key: &str) -> Result<bool, PersistenceError> {
        let file_path = self.state_path(namespace, key);
        if file_path.exists() {
            fs::remove_file(file_path)?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl ScriptStateStore for FileStore {
    fn save(&self, namespace: &str, key: &str, data: &[u8]) -> Result<(), PersistenceError> {
        let mut cache = self.cache.write();
        let ns = cache.entry(namespace.to_string()).or_default();

        let state = if let Some(existing) = ns.get_mut(key) {
            existing.update(data.to_vec());
            existing.clone()
        } else {
            let state = ScriptState::new(namespace, key, data.to_vec());
            ns.insert(key.to_string(), state.clone());
            state
        };

        // Write to disk (drop lock first to avoid deadlock)
        drop(cache);
        self.write_to_disk(&state)?;

        Ok(())
    }

    fn load(&self, namespace: &str, key: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
        let cache = self.cache.read();
        Ok(cache
            .get(namespace)
            .and_then(|ns| ns.get(key))
            .map(|state| state.data.clone()))
    }

    fn delete(&self, namespace: &str, key: &str) -> Result<bool, PersistenceError> {
        let mut cache = self.cache.write();
        let removed = if let Some(ns) = cache.get_mut(namespace) {
            ns.remove(key).is_some()
        } else {
            false
        };

        // Also remove from disk
        drop(cache);
        self.delete_from_disk(namespace, key)?;

        Ok(removed)
    }

    fn list_keys(&self, namespace: &str) -> Result<Vec<String>, PersistenceError> {
        let cache = self.cache.read();
        Ok(cache
            .get(namespace)
            .map(|ns| ns.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn clear_namespace(&self, namespace: &str) -> Result<usize, PersistenceError> {
        let mut cache = self.cache.write();
        let count = if let Some(ns) = cache.remove(namespace) {
            ns.len()
        } else {
            0
        };

        // Remove directory from disk
        drop(cache);
        let ns_path = self.namespace_path(namespace);
        if ns_path.exists() {
            fs::remove_dir_all(ns_path)?;
        }

        Ok(count)
    }

    fn get_metadata(&self, namespace: &str, key: &str) -> Result<Option<ScriptState>, PersistenceError> {
        let cache = self.cache.read();
        Ok(cache
            .get(namespace)
            .and_then(|ns| ns.get(key))
            .cloned())
    }

    fn checkpoint(&self, name: &str) -> Result<(), PersistenceError> {
        let cache = self.cache.read();
        let checkpoint_path = self.checkpoints_path().join(sanitize_filename(name));

        fs::create_dir_all(&checkpoint_path)?;

        // Copy all current state to checkpoint directory
        for (namespace, keys) in cache.iter() {
            let ns_path = checkpoint_path.join(sanitize_filename(namespace));
            fs::create_dir_all(&ns_path)?;

            for (key, state) in keys.iter() {
                let file_path = ns_path.join(format!("{}.json", sanitize_filename(key)));
                let json = serde_json::to_string_pretty(state)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
                fs::write(file_path, json)?;
            }
        }

        Ok(())
    }

    fn restore_checkpoint(&self, name: &str) -> Result<bool, PersistenceError> {
        let checkpoint_path = self.checkpoints_path().join(sanitize_filename(name));

        if !checkpoint_path.exists() {
            return Ok(false);
        }

        // Clear current state
        let mut cache = self.cache.write();
        cache.clear();

        // Clear current files (except checkpoints)
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name != self.checkpoints_dir {
                        fs::remove_dir_all(&path)?;
                    }
                }
            }
        }

        // Load checkpoint data
        for entry in fs::read_dir(&checkpoint_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let namespace = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let ns_cache = cache.entry(namespace.clone()).or_default();

                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(contents) = fs::read_to_string(&file_path) {
                            if let Ok(state) = serde_json::from_str::<ScriptState>(&contents) {
                                ns_cache.insert(state.key.clone(), state);
                            }
                        }
                    }
                }
            }
        }

        // Copy to main storage
        drop(cache);
        let cache = self.cache.read();
        for (_namespace, keys) in cache.iter() {
            for (_key, state) in keys.iter() {
                // Manually call write_to_disk to avoid deadlock
                let ns_path = self.namespace_path(&state.namespace);
                fs::create_dir_all(&ns_path)?;

                let file_path = self.state_path(&state.namespace, &state.key);
                let json = serde_json::to_string_pretty(state)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
                fs::write(file_path, json)?;
            }
        }

        Ok(true)
    }

    fn list_checkpoints(&self) -> Result<Vec<String>, PersistenceError> {
        let checkpoint_path = self.checkpoints_path();

        if !checkpoint_path.exists() {
            return Ok(vec![]);
        }

        let mut checkpoints = Vec::new();
        for entry in fs::read_dir(checkpoint_path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    checkpoints.push(name.to_string());
                }
            }
        }

        Ok(checkpoints)
    }

    fn delete_checkpoint(&self, name: &str) -> Result<bool, PersistenceError> {
        let checkpoint_path = self.checkpoints_path().join(sanitize_filename(name));

        if checkpoint_path.exists() {
            fs::remove_dir_all(checkpoint_path)?;
            return Ok(true);
        }

        Ok(false)
    }
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_in_memory_store_basic() {
        let store = InMemoryStore::new();

        // Save
        store.save("test", "key1", b"hello").unwrap();
        assert_eq!(store.entry_count(), 1);

        // Load
        let data = store.load("test", "key1").unwrap();
        assert_eq!(data, Some(b"hello".to_vec()));

        // Update
        store.save("test", "key1", b"world").unwrap();
        let data = store.load("test", "key1").unwrap();
        assert_eq!(data, Some(b"world".to_vec()));

        // Delete
        assert!(store.delete("test", "key1").unwrap());
        assert_eq!(store.load("test", "key1").unwrap(), None);
    }

    #[test]
    fn test_in_memory_store_namespaces() {
        let store = InMemoryStore::new();

        store.save("ns1", "key1", b"a").unwrap();
        store.save("ns1", "key2", b"b").unwrap();
        store.save("ns2", "key1", b"c").unwrap();

        assert_eq!(store.namespace_count(), 2);

        let keys = store.list_keys("ns1").unwrap();
        assert_eq!(keys.len(), 2);

        let cleared = store.clear_namespace("ns1").unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(store.list_keys("ns1").unwrap().len(), 0);
    }

    #[test]
    fn test_in_memory_checkpoints() {
        let store = InMemoryStore::new();

        store.save("test", "key1", b"original").unwrap();
        store.checkpoint("cp1").unwrap();

        store.save("test", "key1", b"modified").unwrap();
        assert_eq!(store.load("test", "key1").unwrap(), Some(b"modified".to_vec()));

        store.restore_checkpoint("cp1").unwrap();
        assert_eq!(store.load("test", "key1").unwrap(), Some(b"original".to_vec()));
    }

    #[test]
    fn test_file_store_basic() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).unwrap();

        // Save
        store.save("test", "key1", b"hello").unwrap();

        // Verify file exists
        let file_path = temp_dir.path().join("test").join("key1.json");
        assert!(file_path.exists());

        // Load
        let data = store.load("test", "key1").unwrap();
        assert_eq!(data, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_file_store_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // Save with first store instance
        {
            let store = FileStore::new(temp_dir.path()).unwrap();
            store.save("test", "key1", b"persistent").unwrap();
        }

        // Load with new store instance
        {
            let store = FileStore::new(temp_dir.path()).unwrap();
            let data = store.load("test", "key1").unwrap();
            assert_eq!(data, Some(b"persistent".to_vec()));
        }
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal"), "normal");
        assert_eq!(sanitize_filename("path/with/slashes"), "path_with_slashes");
        assert_eq!(sanitize_filename("file:name"), "file_name");
    }
}
