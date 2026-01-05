//! Archetype Registry
//!
//! Loads and manages entity archetypes from Rhai definition scripts.
//!
//! # Loading Process
//!
//! 1. Run `scripts/definitions/ships.rhai` and call `all_ships()`
//! 2. Run `scripts/definitions/weapons.rhai` and call `all_weapons()`
//! 3. Parse results into typed archetypes
//!
//! Note: `base.rhai` provides shared helpers but requires Rhai's import
//! system to be configured. For now, definition files include their own
//! helpers or inline the values.
//!
//! # Hot Reload
//!
//! When a definition file changes, call `reload()` to update archetypes
//! without restarting the server.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use rhai::{Engine, Scope, AST, Dynamic};

use super::{ShipArchetype, WeaponArchetype};

/// Error type for archetype operations.
#[derive(Debug, Clone)]
pub enum ArchetypeError {
    /// Script file not found
    FileNotFound(String),
    /// Script compilation failed
    CompilationError(String),
    /// Script execution failed
    ExecutionError(String),
    /// Invalid archetype data
    InvalidArchetype(String),
    /// Archetype not found
    NotFound(String),
}

impl std::fmt::Display for ArchetypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "Definition file not found: {}", path),
            Self::CompilationError(msg) => write!(f, "Script compilation error: {}", msg),
            Self::ExecutionError(msg) => write!(f, "Script execution error: {}", msg),
            Self::InvalidArchetype(msg) => write!(f, "Invalid archetype: {}", msg),
            Self::NotFound(id) => write!(f, "Archetype not found: {}", id),
        }
    }
}

impl std::error::Error for ArchetypeError {}

/// Registry for all entity archetypes.
///
/// Thread-safe and supports hot-reload.
#[derive(Debug)]
pub struct ArchetypeRegistry {
    ships: RwLock<HashMap<String, Arc<ShipArchetype>>>,
    weapons: RwLock<HashMap<String, Arc<WeaponArchetype>>>,
    /// Cached compiled ASTs for faster reloading
    compiled_scripts: RwLock<HashMap<String, AST>>,
}

impl Default for ArchetypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchetypeRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            ships: RwLock::new(HashMap::new()),
            weapons: RwLock::new(HashMap::new()),
            compiled_scripts: RwLock::new(HashMap::new()),
        }
    }

    /// Load all archetypes from the definitions directory.
    ///
    /// # Arguments
    ///
    /// * `engine` - Rhai engine for script execution
    /// * `scripts_dir` - Path to the scripts directory
    pub fn load_from_directory(
        &self,
        engine: &Engine,
        scripts_dir: &Path,
    ) -> Result<LoadResult, ArchetypeError> {
        let definitions_dir = scripts_dir.join("definitions");

        let mut result = LoadResult::default();

        // Load ships
        let ships_path = definitions_dir.join("ships.rhai");
        if ships_path.exists() {
            match self.load_ships(engine, &ships_path) {
                Ok(count) => result.ships_loaded = count,
                Err(e) => {
                    tracing::warn!("Failed to load ships: {}", e);
                    result.errors.push(format!("ships.rhai: {}", e));
                }
            }
        } else {
            tracing::info!("No ships.rhai found, using built-in ship classes");
        }

        // Load weapons
        let weapons_path = definitions_dir.join("weapons.rhai");
        if weapons_path.exists() {
            match self.load_weapons(engine, &weapons_path) {
                Ok(count) => result.weapons_loaded = count,
                Err(e) => {
                    tracing::warn!("Failed to load weapons: {}", e);
                    result.errors.push(format!("weapons.rhai: {}", e));
                }
            }
        }

        Ok(result)
    }

    /// Load ship archetypes from a script file.
    fn load_ships(&self, engine: &Engine, path: &Path) -> Result<usize, ArchetypeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| ArchetypeError::FileNotFound(path.display().to_string()))?;

        let ast = engine.compile(&content)
            .map_err(|e| ArchetypeError::CompilationError(e.to_string()))?;

        // Store compiled AST for hot-reload
        self.compiled_scripts.write().insert(
            path.display().to_string(),
            ast.clone(),
        );

        // Create scope and run the script to define functions
        let mut scope = Scope::new();
        engine.run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| ArchetypeError::ExecutionError(e.to_string()))?;

        // Call all_ships() to get the array of ship definitions
        let ships_result: Dynamic = engine.call_fn(&mut scope, &ast, "all_ships", ())
            .map_err(|e| ArchetypeError::ExecutionError(format!("all_ships(): {}", e)))?;

        // Parse the array
        let ships_array = ships_result.into_array()
            .map_err(|_| ArchetypeError::InvalidArchetype(
                "all_ships() must return an array".into()
            ))?;

        let mut ships = self.ships.write();
        ships.clear();

        for ship_data in ships_array {
            if let Some(archetype) = ShipArchetype::from_dynamic(ship_data) {
                let id = archetype.id.clone();
                ships.insert(id, Arc::new(archetype));
            }
        }

        let count = ships.len();
        tracing::info!("Loaded {} ship archetypes from {}", count, path.display());

        Ok(count)
    }

    /// Load weapon archetypes from a script file.
    fn load_weapons(&self, engine: &Engine, path: &Path) -> Result<usize, ArchetypeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| ArchetypeError::FileNotFound(path.display().to_string()))?;

        let ast = engine.compile(&content)
            .map_err(|e| ArchetypeError::CompilationError(e.to_string()))?;

        self.compiled_scripts.write().insert(
            path.display().to_string(),
            ast.clone(),
        );

        let mut scope = Scope::new();
        engine.run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| ArchetypeError::ExecutionError(e.to_string()))?;

        let weapons_result: Dynamic = engine.call_fn(&mut scope, &ast, "all_weapons", ())
            .map_err(|e| ArchetypeError::ExecutionError(format!("all_weapons(): {}", e)))?;

        let weapons_array = weapons_result.into_array()
            .map_err(|_| ArchetypeError::InvalidArchetype(
                "all_weapons() must return an array".into()
            ))?;

        let mut weapons = self.weapons.write();
        weapons.clear();

        for weapon_data in weapons_array {
            if let Some(archetype) = WeaponArchetype::from_dynamic(weapon_data) {
                let id = archetype.id.clone();
                weapons.insert(id, Arc::new(archetype));
            }
        }

        let count = weapons.len();
        tracing::info!("Loaded {} weapon archetypes from {}", count, path.display());

        Ok(count)
    }

    /// Hot-reload a specific definition file.
    pub fn reload(
        &self,
        engine: &Engine,
        changed_path: &Path,
    ) -> Result<usize, ArchetypeError> {
        let filename = changed_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        match filename {
            "ships.rhai" => self.load_ships(engine, changed_path),
            "weapons.rhai" => self.load_weapons(engine, changed_path),
            _ => {
                tracing::debug!("Ignoring reload for non-definition file: {}", filename);
                Ok(0)
            }
        }
    }

    // =========================================================================
    // Ship accessors
    // =========================================================================

    /// Get a ship archetype by ID.
    pub fn get_ship(&self, id: &str) -> Option<Arc<ShipArchetype>> {
        self.ships.read().get(id).cloned()
    }

    /// Get all ship archetypes.
    pub fn all_ships(&self) -> Vec<Arc<ShipArchetype>> {
        self.ships.read().values().cloned().collect()
    }

    /// Get all player ship archetypes.
    pub fn player_ships(&self) -> Vec<Arc<ShipArchetype>> {
        self.ships.read()
            .values()
            .filter(|s| s.is_player_class)
            .cloned()
            .collect()
    }

    /// Get all hostile NPC ship archetypes.
    pub fn hostile_ships(&self) -> Vec<Arc<ShipArchetype>> {
        self.ships.read()
            .values()
            .filter(|s| s.is_hostile)
            .cloned()
            .collect()
    }

    /// Check if a ship archetype exists.
    pub fn has_ship(&self, id: &str) -> bool {
        self.ships.read().contains_key(id)
    }

    /// Get ship count.
    pub fn ship_count(&self) -> usize {
        self.ships.read().len()
    }

    // =========================================================================
    // Weapon accessors
    // =========================================================================

    /// Get a weapon archetype by ID.
    pub fn get_weapon(&self, id: &str) -> Option<Arc<WeaponArchetype>> {
        self.weapons.read().get(id).cloned()
    }

    /// Get all weapon archetypes.
    pub fn all_weapons(&self) -> Vec<Arc<WeaponArchetype>> {
        self.weapons.read().values().cloned().collect()
    }

    /// Check if a weapon archetype exists.
    pub fn has_weapon(&self, id: &str) -> bool {
        self.weapons.read().contains_key(id)
    }

    /// Get weapon count.
    pub fn weapon_count(&self) -> usize {
        self.weapons.read().len()
    }

    // =========================================================================
    // Bulk operations
    // =========================================================================

    /// Clear all archetypes.
    pub fn clear(&self) {
        self.ships.write().clear();
        self.weapons.write().clear();
        self.compiled_scripts.write().clear();
    }

    /// Register a ship archetype directly (for testing or fallback).
    pub fn register_ship(&self, archetype: ShipArchetype) {
        let id = archetype.id.clone();
        self.ships.write().insert(id, Arc::new(archetype));
    }

    /// Register a weapon archetype directly (for testing or fallback).
    pub fn register_weapon(&self, archetype: WeaponArchetype) {
        let id = archetype.id.clone();
        self.weapons.write().insert(id, Arc::new(archetype));
    }
}

/// Result of loading archetypes.
#[derive(Debug, Default)]
pub struct LoadResult {
    pub ships_loaded: usize,
    pub weapons_loaded: usize,
    pub errors: Vec<String>,
}

impl LoadResult {
    /// Check if any errors occurred.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Total archetypes loaded.
    pub fn total_loaded(&self) -> usize {
        self.ships_loaded + self.weapons_loaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = ArchetypeRegistry::new();
        assert_eq!(registry.ship_count(), 0);
        assert_eq!(registry.weapon_count(), 0);
    }

    #[test]
    fn test_register_ship() {
        let registry = ArchetypeRegistry::new();

        let ship = ShipArchetype {
            id: "test_ship".to_string(),
            name: "Test Ship".to_string(),
            description: None,
            tier: 1,
            stats: Default::default(),
            weapons: vec![],
            upgrade_slots: vec![],
            is_player_class: true,
            is_hostile: false,
            behaviors: vec![],
            unlock: None,
        };

        registry.register_ship(ship);

        assert!(registry.has_ship("test_ship"));
        assert_eq!(registry.ship_count(), 1);

        let loaded = registry.get_ship("test_ship").unwrap();
        assert_eq!(loaded.name, "Test Ship");
    }
}
