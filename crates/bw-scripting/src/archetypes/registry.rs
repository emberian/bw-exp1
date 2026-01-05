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

use super::{ShipArchetype, WeaponArchetype, EffectArchetype, AbilityArchetype, CargoArchetype, FactionArchetype};

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
    effects: RwLock<HashMap<String, Arc<EffectArchetype>>>,
    abilities: RwLock<HashMap<String, Arc<AbilityArchetype>>>,
    cargo: RwLock<HashMap<String, Arc<CargoArchetype>>>,
    factions: RwLock<HashMap<String, Arc<FactionArchetype>>>,
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
            effects: RwLock::new(HashMap::new()),
            abilities: RwLock::new(HashMap::new()),
            cargo: RwLock::new(HashMap::new()),
            factions: RwLock::new(HashMap::new()),
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

        // Load effects
        let effects_path = definitions_dir.join("effects.rhai");
        if effects_path.exists() {
            match self.load_effects(engine, &effects_path) {
                Ok(count) => result.effects_loaded = count,
                Err(e) => {
                    tracing::warn!("Failed to load effects: {}", e);
                    result.errors.push(format!("effects.rhai: {}", e));
                }
            }
        }

        // Load abilities
        let abilities_path = definitions_dir.join("abilities.rhai");
        if abilities_path.exists() {
            match self.load_abilities(engine, &abilities_path) {
                Ok(count) => result.abilities_loaded = count,
                Err(e) => {
                    tracing::warn!("Failed to load abilities: {}", e);
                    result.errors.push(format!("abilities.rhai: {}", e));
                }
            }
        }

        // Load cargo
        let cargo_path = definitions_dir.join("cargo.rhai");
        if cargo_path.exists() {
            match self.load_cargo(engine, &cargo_path) {
                Ok(count) => result.cargo_loaded = count,
                Err(e) => {
                    tracing::warn!("Failed to load cargo: {}", e);
                    result.errors.push(format!("cargo.rhai: {}", e));
                }
            }
        }

        // Load factions
        let factions_path = definitions_dir.join("factions.rhai");
        if factions_path.exists() {
            match self.load_factions(engine, &factions_path) {
                Ok(count) => result.factions_loaded = count,
                Err(e) => {
                    tracing::warn!("Failed to load factions: {}", e);
                    result.errors.push(format!("factions.rhai: {}", e));
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

    /// Load effect archetypes from a script file.
    fn load_effects(&self, engine: &Engine, path: &Path) -> Result<usize, ArchetypeError> {
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

        let effects_result: Dynamic = engine.call_fn(&mut scope, &ast, "all_effects", ())
            .map_err(|e| ArchetypeError::ExecutionError(format!("all_effects(): {}", e)))?;

        let effects_array = effects_result.into_array()
            .map_err(|_| ArchetypeError::InvalidArchetype(
                "all_effects() must return an array".into()
            ))?;

        let mut effects = self.effects.write();
        effects.clear();

        for effect_data in effects_array {
            if let Some(archetype) = EffectArchetype::from_dynamic(effect_data) {
                let id = archetype.id.clone();
                effects.insert(id, Arc::new(archetype));
            }
        }

        let count = effects.len();
        tracing::info!("Loaded {} effect archetypes from {}", count, path.display());

        Ok(count)
    }

    /// Load ability archetypes from a script file.
    fn load_abilities(&self, engine: &Engine, path: &Path) -> Result<usize, ArchetypeError> {
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

        let abilities_result: Dynamic = engine.call_fn(&mut scope, &ast, "all_abilities", ())
            .map_err(|e| ArchetypeError::ExecutionError(format!("all_abilities(): {}", e)))?;

        let abilities_array = abilities_result.into_array()
            .map_err(|_| ArchetypeError::InvalidArchetype(
                "all_abilities() must return an array".into()
            ))?;

        let mut abilities = self.abilities.write();
        abilities.clear();

        for ability_data in abilities_array {
            if let Some(archetype) = AbilityArchetype::from_dynamic(ability_data) {
                let id = archetype.id.clone();
                abilities.insert(id, Arc::new(archetype));
            }
        }

        let count = abilities.len();
        tracing::info!("Loaded {} ability archetypes from {}", count, path.display());

        Ok(count)
    }

    /// Load cargo archetypes from a script file.
    fn load_cargo(&self, engine: &Engine, path: &Path) -> Result<usize, ArchetypeError> {
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

        let cargo_result: Dynamic = engine.call_fn(&mut scope, &ast, "all_cargo", ())
            .map_err(|e| ArchetypeError::ExecutionError(format!("all_cargo(): {}", e)))?;

        let cargo_array = cargo_result.into_array()
            .map_err(|_| ArchetypeError::InvalidArchetype(
                "all_cargo() must return an array".into()
            ))?;

        let mut cargo = self.cargo.write();
        cargo.clear();

        for cargo_data in cargo_array {
            if let Some(archetype) = CargoArchetype::from_dynamic(cargo_data) {
                let id = archetype.id.clone();
                cargo.insert(id, Arc::new(archetype));
            }
        }

        let count = cargo.len();
        tracing::info!("Loaded {} cargo archetypes from {}", count, path.display());

        Ok(count)
    }

    /// Load faction archetypes from a script file.
    fn load_factions(&self, engine: &Engine, path: &Path) -> Result<usize, ArchetypeError> {
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

        let factions_result: Dynamic = engine.call_fn(&mut scope, &ast, "all_factions", ())
            .map_err(|e| ArchetypeError::ExecutionError(format!("all_factions(): {}", e)))?;

        let factions_array = factions_result.into_array()
            .map_err(|_| ArchetypeError::InvalidArchetype(
                "all_factions() must return an array".into()
            ))?;

        let mut factions = self.factions.write();
        factions.clear();

        for faction_data in factions_array {
            if let Some(archetype) = FactionArchetype::from_dynamic(faction_data) {
                let id = archetype.id.clone();
                factions.insert(id, Arc::new(archetype));
            }
        }

        let count = factions.len();
        tracing::info!("Loaded {} faction archetypes from {}", count, path.display());

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
            "effects.rhai" => self.load_effects(engine, changed_path),
            "abilities.rhai" => self.load_abilities(engine, changed_path),
            "cargo.rhai" => self.load_cargo(engine, changed_path),
            "factions.rhai" => self.load_factions(engine, changed_path),
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
    // Effect accessors
    // =========================================================================

    /// Get an effect archetype by ID.
    pub fn get_effect(&self, id: &str) -> Option<Arc<EffectArchetype>> {
        self.effects.read().get(id).cloned()
    }

    /// Get all effect archetypes.
    pub fn all_effects(&self) -> Vec<Arc<EffectArchetype>> {
        self.effects.read().values().cloned().collect()
    }

    /// Check if an effect archetype exists.
    pub fn has_effect(&self, id: &str) -> bool {
        self.effects.read().contains_key(id)
    }

    /// Get effect count.
    pub fn effect_count(&self) -> usize {
        self.effects.read().len()
    }

    // =========================================================================
    // Ability accessors
    // =========================================================================

    /// Get an ability archetype by ID.
    pub fn get_ability(&self, id: &str) -> Option<Arc<AbilityArchetype>> {
        self.abilities.read().get(id).cloned()
    }

    /// Get all ability archetypes.
    pub fn all_abilities(&self) -> Vec<Arc<AbilityArchetype>> {
        self.abilities.read().values().cloned().collect()
    }

    /// Get active (non-passive) abilities.
    pub fn active_abilities(&self) -> Vec<Arc<AbilityArchetype>> {
        self.abilities.read()
            .values()
            .filter(|a| !a.is_passive)
            .cloned()
            .collect()
    }

    /// Get passive abilities.
    pub fn passive_abilities(&self) -> Vec<Arc<AbilityArchetype>> {
        self.abilities.read()
            .values()
            .filter(|a| a.is_passive)
            .cloned()
            .collect()
    }

    /// Check if an ability archetype exists.
    pub fn has_ability(&self, id: &str) -> bool {
        self.abilities.read().contains_key(id)
    }

    /// Get ability count.
    pub fn ability_count(&self) -> usize {
        self.abilities.read().len()
    }

    // =========================================================================
    // Cargo accessors
    // =========================================================================

    /// Get a cargo archetype by ID.
    pub fn get_cargo(&self, id: &str) -> Option<Arc<CargoArchetype>> {
        self.cargo.read().get(id).cloned()
    }

    /// Get all cargo archetypes.
    pub fn all_cargo(&self) -> Vec<Arc<CargoArchetype>> {
        self.cargo.read().values().cloned().collect()
    }

    /// Get legal cargo only.
    pub fn legal_cargo(&self) -> Vec<Arc<CargoArchetype>> {
        self.cargo.read()
            .values()
            .filter(|c| c.legal)
            .cloned()
            .collect()
    }

    /// Get contraband cargo only.
    pub fn contraband_cargo(&self) -> Vec<Arc<CargoArchetype>> {
        self.cargo.read()
            .values()
            .filter(|c| c.is_contraband())
            .cloned()
            .collect()
    }

    /// Check if a cargo archetype exists.
    pub fn has_cargo(&self, id: &str) -> bool {
        self.cargo.read().contains_key(id)
    }

    /// Get cargo count.
    pub fn cargo_count(&self) -> usize {
        self.cargo.read().len()
    }

    // =========================================================================
    // Faction accessors
    // =========================================================================

    /// Get a faction archetype by ID.
    pub fn get_faction(&self, id: &str) -> Option<Arc<FactionArchetype>> {
        self.factions.read().get(id).cloned()
    }

    /// Get all faction archetypes.
    pub fn all_factions(&self) -> Vec<Arc<FactionArchetype>> {
        self.factions.read().values().cloned().collect()
    }

    /// Get playable factions only.
    pub fn playable_factions(&self) -> Vec<Arc<FactionArchetype>> {
        self.factions.read()
            .values()
            .filter(|f| f.is_playable)
            .cloned()
            .collect()
    }

    /// Get hostile factions only.
    pub fn hostile_factions(&self) -> Vec<Arc<FactionArchetype>> {
        self.factions.read()
            .values()
            .filter(|f| f.is_hostile)
            .cloned()
            .collect()
    }

    /// Check if a faction archetype exists.
    pub fn has_faction(&self, id: &str) -> bool {
        self.factions.read().contains_key(id)
    }

    /// Get faction count.
    pub fn faction_count(&self) -> usize {
        self.factions.read().len()
    }

    /// Get the relation between two factions.
    ///
    /// Returns 0 (neutral) if either faction doesn't exist or no relation is defined.
    pub fn get_faction_relation(&self, faction_a: &str, faction_b: &str) -> i32 {
        if let Some(faction) = self.get_faction(faction_a) {
            faction.get_relation(faction_b)
        } else {
            0
        }
    }

    // =========================================================================
    // Bulk operations
    // =========================================================================

    /// Clear all archetypes.
    pub fn clear(&self) {
        self.ships.write().clear();
        self.weapons.write().clear();
        self.effects.write().clear();
        self.abilities.write().clear();
        self.cargo.write().clear();
        self.factions.write().clear();
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

    /// Register an effect archetype directly (for testing or fallback).
    pub fn register_effect(&self, archetype: EffectArchetype) {
        let id = archetype.id.clone();
        self.effects.write().insert(id, Arc::new(archetype));
    }

    /// Register an ability archetype directly (for testing or fallback).
    pub fn register_ability(&self, archetype: AbilityArchetype) {
        let id = archetype.id.clone();
        self.abilities.write().insert(id, Arc::new(archetype));
    }

    /// Register a cargo archetype directly (for testing or fallback).
    pub fn register_cargo(&self, archetype: CargoArchetype) {
        let id = archetype.id.clone();
        self.cargo.write().insert(id, Arc::new(archetype));
    }

    /// Register a faction archetype directly (for testing or fallback).
    pub fn register_faction(&self, archetype: FactionArchetype) {
        let id = archetype.id.clone();
        self.factions.write().insert(id, Arc::new(archetype));
    }
}

/// Result of loading archetypes.
#[derive(Debug, Default)]
pub struct LoadResult {
    pub ships_loaded: usize,
    pub weapons_loaded: usize,
    pub effects_loaded: usize,
    pub abilities_loaded: usize,
    pub cargo_loaded: usize,
    pub factions_loaded: usize,
    pub errors: Vec<String>,
}

impl LoadResult {
    /// Check if any errors occurred.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Total archetypes loaded.
    pub fn total_loaded(&self) -> usize {
        self.ships_loaded + self.weapons_loaded + self.effects_loaded
            + self.abilities_loaded + self.cargo_loaded + self.factions_loaded
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
