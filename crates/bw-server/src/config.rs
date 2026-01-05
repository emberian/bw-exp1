//! Server configuration
//!
//! Centralized configuration loaded from TOML with hot-reload support.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind, event::ModifyKind};
use parking_lot::RwLock;
use serde::Deserialize;

// Re-export simulation config types
pub use crate::simulation::sim_config::{
    SimulationConfig, NpcSpawningConfig, DangerMultipliers, CombatConfig,
};

/// Default config file path.
pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

/// Server configuration loaded from TOML.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ServerConfig {
    /// Server settings
    pub server: ServerSettings,
    /// Admin settings
    pub admin: AdminSettings,
    /// Scripting settings
    pub scripting: ScriptingSettings,
    /// Simulation settings (spawning, combat, etc.)
    pub simulation: SimulationConfig,
}

/// General server settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    /// Server bind address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Database URL (can be overridden by DATABASE_URL env var)
    pub database_url: Option<String>,
    /// Allowed CORS origins (empty = allow all, which is insecure)
    pub cors_origins: Vec<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            database_url: None,
            cors_origins: vec![],
        }
    }
}

/// Admin/GM settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct AdminSettings {
    /// Usernames with admin privileges (case-insensitive)
    pub usernames: Vec<String>,
}

impl AdminSettings {
    /// Check if a username has admin privileges.
    pub fn is_admin(&self, username: &str) -> bool {
        let lower = username.to_lowercase();
        self.usernames.iter().any(|u| u.to_lowercase() == lower)
    }
}

/// Scripting settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ScriptingSettings {
    /// Path to scripts directory
    pub scripts_dir: String,
    /// Enable hot-reload in debug builds
    pub hot_reload: bool,
    /// Maximum script log entries to keep
    pub max_log_entries: usize,
}

impl Default for ScriptingSettings {
    fn default() -> Self {
        Self {
            scripts_dir: "scripts".to_string(),
            hot_reload: true,
            max_log_entries: 1000,
        }
    }
}

impl ServerConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            tracing::info!("Config file not found at {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)?;
        let config: ServerConfig = toml::from_str(&contents)?;

        tracing::info!("Loaded config from {:?}", path);
        tracing::debug!("Admin usernames: {:?}", config.admin.usernames);

        Ok(config)
    }

    /// Load configuration, falling back to defaults on error.
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        match Self::load(path.as_ref()) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("Failed to load config from {:?}: {}, using defaults", path.as_ref(), e);
                Self::default()
            }
        }
    }
}

/// Thread-safe configuration holder with hot-reload support.
pub struct ConfigManager {
    /// Current configuration
    config: RwLock<Arc<ServerConfig>>,
    /// Path to config file
    path: PathBuf,
    /// File watcher (kept alive to maintain watch)
    _watcher: Option<RecommendedWatcher>,
}

impl ConfigManager {
    /// Create a new config manager and load the initial config.
    pub fn new(path: impl Into<PathBuf>) -> anyhow::Result<Arc<Self>> {
        let path = path.into();
        let config = ServerConfig::load_or_default(&path);

        let manager = Arc::new(Self {
            config: RwLock::new(Arc::new(config)),
            path: path.clone(),
            _watcher: None,
        });

        Ok(manager)
    }

    /// Create a config manager with hot-reload enabled.
    pub fn with_hot_reload(path: impl Into<PathBuf>) -> anyhow::Result<Arc<Self>> {
        let path = path.into();
        let config = ServerConfig::load_or_default(&path);

        // Create manager without watcher first
        let manager = Arc::new(Self {
            config: RwLock::new(Arc::new(config)),
            path: path.clone(),
            _watcher: None,
        });

        // Set up file watcher
        let manager_weak = Arc::downgrade(&manager);

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            match res {
                Ok(event) => {
                    // Only reload on modify events
                    if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any)) {
                        if let Some(manager) = manager_weak.upgrade() {
                            manager.reload();
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Config watcher error: {}", e);
                }
            }
        })?;

        // Watch the config file's parent directory (watching single file can be unreliable)
        // Note: path.parent() returns Some("") for "config.toml", so check if parent is non-empty
        let watch_path = path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        watcher.watch(&watch_path, RecursiveMode::NonRecursive)?;

        tracing::info!("Config hot-reload enabled for {:?}", path);

        // We need to set the watcher, but we can't mutate Arc
        // Use a workaround: store watcher in a static or leak it
        // For simplicity, we'll leak the watcher to keep it alive
        std::mem::forget(watcher);

        Ok(manager)
    }

    /// Get the current configuration.
    pub fn get(&self) -> Arc<ServerConfig> {
        self.config.read().clone()
    }

    /// Reload configuration from disk.
    pub fn reload(&self) {
        match ServerConfig::load(&self.path) {
            Ok(new_config) => {
                let old_config = self.config.read().clone();
                *self.config.write() = Arc::new(new_config.clone());

                // Log what changed
                Self::log_changes(&old_config, &new_config);

                tracing::info!("Configuration reloaded from {:?}", self.path);
            }
            Err(e) => {
                tracing::error!("Failed to reload config: {}", e);
            }
        }
    }

    /// Log configuration changes.
    fn log_changes(old: &ServerConfig, new: &ServerConfig) {
        // Check admin changes
        let old_admins: HashSet<_> = old.admin.usernames.iter().map(|s| s.to_lowercase()).collect();
        let new_admins: HashSet<_> = new.admin.usernames.iter().map(|s| s.to_lowercase()).collect();

        let added: Vec<_> = new_admins.difference(&old_admins).collect();
        let removed: Vec<_> = old_admins.difference(&new_admins).collect();

        if !added.is_empty() {
            tracing::info!("Admin users added: {:?}", added);
        }
        if !removed.is_empty() {
            tracing::info!("Admin users removed: {:?}", removed);
        }
    }

    /// Check if a username is an admin.
    pub fn is_admin(&self, username: &str) -> bool {
        self.config.read().admin.is_admin(username)
    }
}

/// Global config manager instance.
static CONFIG: std::sync::OnceLock<Arc<ConfigManager>> = std::sync::OnceLock::new();

/// Initialize the global config manager.
pub fn init_config(path: impl Into<PathBuf>, hot_reload: bool) -> anyhow::Result<Arc<ConfigManager>> {
    let path = path.into();

    let manager = if hot_reload {
        ConfigManager::with_hot_reload(path)?
    } else {
        ConfigManager::new(path)?
    };

    // Try to set the global, ignore if already set
    let _ = CONFIG.set(manager.clone());

    Ok(manager)
}

/// Get the global config manager.
///
/// Panics if config has not been initialized.
pub fn config() -> &'static Arc<ConfigManager> {
    CONFIG.get().expect("Config not initialized. Call init_config() first.")
}

/// Try to get the global config manager.
pub fn try_config() -> Option<&'static Arc<ConfigManager>> {
    CONFIG.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.port, 3000);
        assert!(config.admin.usernames.is_empty());
    }

    #[test]
    fn test_load_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"
[admin]
usernames = ["admin", "gm", "superuser"]

[server]
port = 8080
"#).unwrap();

        let config = ServerConfig::load(file.path()).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.admin.usernames.len(), 3);
        assert!(config.admin.is_admin("admin"));
        assert!(config.admin.is_admin("ADMIN")); // case insensitive
        assert!(config.admin.is_admin("gm"));
        assert!(!config.admin.is_admin("regularuser"));
    }

    #[test]
    fn test_missing_config_uses_defaults() {
        let config = ServerConfig::load_or_default("/nonexistent/path/config.toml");
        assert_eq!(config.server.port, 3000);
    }

    #[test]
    fn test_admin_settings_is_admin() {
        let settings = AdminSettings {
            usernames: vec!["Admin".to_string(), "GM".to_string()],
        };

        assert!(settings.is_admin("admin"));
        assert!(settings.is_admin("ADMIN"));
        assert!(settings.is_admin("gm"));
        assert!(settings.is_admin("GM"));
        assert!(!settings.is_admin("user"));
        assert!(!settings.is_admin(""));
    }
}
