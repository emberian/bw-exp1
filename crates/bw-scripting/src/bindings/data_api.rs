//! Data API bindings for Rhai
//!
//! Provides access to game data files (TOML) from scripts.
//! Data is loaded once and cached for fast access.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use rhai::{Engine, Dynamic, Map};

/// Thread-local data accessor for script execution.
thread_local! {
    static CURRENT_DATA: RefCell<Option<Arc<DataStore>>> = const { RefCell::new(None) };
}

/// Set the data store for the current thread during script execution.
pub fn set_current_data(data: Arc<DataStore>) {
    CURRENT_DATA.with(|cell| {
        *cell.borrow_mut() = Some(data);
    });
}

/// Clear the data store after script execution.
pub fn clear_current_data() {
    CURRENT_DATA.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Get data from the current store.
fn with_data<T, F: FnOnce(&DataStore) -> T>(f: F) -> Option<T> {
    CURRENT_DATA.with(|cell| {
        cell.borrow().as_ref().map(|data| f(data))
    })
}

/// Store for game data loaded from TOML files.
pub struct DataStore {
    /// Data indexed by category (file name without extension) and key
    data: RwLock<HashMap<String, HashMap<String, Dynamic>>>,
    /// Path to data directory
    data_dir: PathBuf,
}

impl DataStore {
    /// Create a new data store and load data from the directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        let store = Self {
            data: RwLock::new(HashMap::new()),
            data_dir,
        };
        store.load_all();
        store
    }

    /// Load all TOML files from the data directory.
    pub fn load_all(&self) {
        let dir = &self.data_dir;

        if !dir.exists() {
            tracing::warn!("Data directory {:?} does not exist", dir);
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::error!("Failed to read data directory {:?}: {}", dir, e);
                return;
            }
        };

        let mut data = self.data.write();
        data.clear();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                if let Some(category) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_file(&path) {
                        Ok(entries) => {
                            tracing::info!("Loaded {} entries from data/{}.toml", entries.len(), category);
                            data.insert(category.to_string(), entries);
                        }
                        Err(e) => {
                            tracing::error!("Failed to load data file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    /// Load a single TOML file and convert to Dynamic maps.
    fn load_file(&self, path: &Path) -> anyhow::Result<HashMap<String, Dynamic>> {
        let contents = std::fs::read_to_string(path)?;
        let table: toml::Table = toml::from_str(&contents)?;

        let mut result = HashMap::new();
        for (key, value) in table {
            result.insert(key, toml_to_dynamic(value));
        }

        Ok(result)
    }

    /// Reload all data files.
    pub fn reload(&self) {
        tracing::info!("Reloading data files from {:?}", self.data_dir);
        self.load_all();
    }

    /// Get data by category and key.
    pub fn get(&self, category: &str, key: &str) -> Option<Dynamic> {
        let data = self.data.read();
        data.get(category)
            .and_then(|entries| entries.get(key))
            .cloned()
    }

    /// Get all entries in a category.
    pub fn get_category(&self, category: &str) -> Option<HashMap<String, Dynamic>> {
        let data = self.data.read();
        data.get(category).cloned()
    }

    /// List all keys in a category.
    pub fn list_keys(&self, category: &str) -> Vec<String> {
        let data = self.data.read();
        data.get(category)
            .map(|entries| entries.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// Convert a TOML value to a Rhai Dynamic.
fn toml_to_dynamic(value: toml::Value) -> Dynamic {
    match value {
        toml::Value::String(s) => Dynamic::from(s),
        toml::Value::Integer(i) => Dynamic::from(i),
        toml::Value::Float(f) => Dynamic::from(f),
        toml::Value::Boolean(b) => Dynamic::from(b),
        toml::Value::Datetime(dt) => Dynamic::from(dt.to_string()),
        toml::Value::Array(arr) => {
            let arr: rhai::Array = arr.into_iter().map(toml_to_dynamic).collect();
            Dynamic::from(arr)
        }
        toml::Value::Table(table) => {
            let mut map = Map::new();
            for (k, v) in table {
                map.insert(k.into(), toml_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}

/// Register data API functions with the engine.
pub fn register(engine: &mut Engine) {
    // get_data(category: String, key: String) -> Map
    // Returns data entry or empty map if not found
    engine.register_fn("get_data", |category: String, key: String| -> Dynamic {
        with_data(|data| {
            data.get(&category, &key).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_ship_def(ship_class: String) -> Map
    // Convenience function for ship data
    engine.register_fn("get_ship_def", |ship_class: String| -> Dynamic {
        with_data(|data| {
            data.get("ships", &ship_class).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_upgrade_def(upgrade_id: String) -> Map
    // Convenience function for upgrade data
    engine.register_fn("get_upgrade_def", |upgrade_id: String| -> Dynamic {
        with_data(|data| {
            data.get("upgrades", &upgrade_id).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_cargo_def(cargo_type: String) -> Map
    // Convenience function for cargo data
    engine.register_fn("get_cargo_def", |cargo_type: String| -> Dynamic {
        with_data(|data| {
            data.get("cargo", &cargo_type).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_stance_def(stance: String) -> Map
    // Convenience function for combat stance data
    engine.register_fn("get_stance_def", |stance: String| -> Dynamic {
        with_data(|data| {
            data.get("combat_stances", &stance).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // list_data_keys(category: String) -> Array
    // Returns array of keys in a data category
    engine.register_fn("list_data_keys", |category: String| -> rhai::Array {
        with_data(|data| {
            let keys = data.list_keys(&category);
            keys.into_iter().map(Dynamic::from).collect()
        }).unwrap_or_default()
    });

    // has_data(category: String, key: String) -> bool
    // Check if data entry exists
    engine.register_fn("has_data", |category: String, key: String| -> bool {
        with_data(|data| {
            data.get(&category, &key).is_some()
        }).unwrap_or(false)
    });
}
