//! Rhai scripting engine setup
//!
//! Central engine with all bindings registered.

use parking_lot::RwLock;
use rhai::{Engine, Scope, AST, Dynamic};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

use crate::bindings;

/// Scripting engine errors.
#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("Script not found: {0}")]
    NotFound(String),

    #[error("Compilation error: {0}")]
    CompileError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("No result returned from script")]
    NoResult,
}

/// The main scripting engine.
pub struct ScriptEngine {
    engine: Engine,
    scripts: RwLock<HashMap<String, AST>>,
    scripts_dir: String,
}

impl ScriptEngine {
    /// Create a new script engine with all bindings.
    pub fn new(scripts_dir: impl Into<String>) -> Self {
        let mut engine = Engine::new();

        // Safety limits for untrusted scripts
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(64, 64);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(1_000);
        engine.set_max_map_size(500);

        // Register all API bindings
        bindings::register_all(&mut engine);

        Self {
            engine,
            scripts: RwLock::new(HashMap::new()),
            scripts_dir: scripts_dir.into(),
        }
    }

    /// Load a script from file.
    pub fn load_script(&self, name: &str) -> Result<(), ScriptError> {
        let path = std::path::PathBuf::from(format!("{}/{}", self.scripts_dir, name));
        let ast = self.engine.compile_file(path)
            .map_err(|e| ScriptError::CompileError(e.to_string()))?;

        self.scripts.write().insert(name.to_string(), ast);
        Ok(())
    }

    /// Load all scripts from the scripts directory.
    pub fn load_all_scripts(&self) -> Result<(), ScriptError> {
        let path = Path::new(&self.scripts_dir);
        self.load_scripts_recursive(path, "")?;
        Ok(())
    }

    fn load_scripts_recursive(&self, dir: &Path, prefix: &str) -> Result<(), ScriptError> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let new_prefix = if prefix.is_empty() {
                    entry.file_name().to_string_lossy().to_string()
                } else {
                    format!("{}/{}", prefix, entry.file_name().to_string_lossy())
                };
                self.load_scripts_recursive(&path, &new_prefix)?;
            } else if path.extension().is_some_and(|ext| ext == "rhai") {
                let name = if prefix.is_empty() {
                    entry.file_name().to_string_lossy().to_string()
                } else {
                    format!("{}/{}", prefix, entry.file_name().to_string_lossy())
                };
                tracing::info!("Loading script: {}", name);
                self.load_script(&name)?;
            }
        }

        Ok(())
    }

    /// Check if a script is loaded.
    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.read().contains_key(name)
    }

    /// Get list of loaded scripts.
    pub fn loaded_scripts(&self) -> Vec<String> {
        self.scripts.read().keys().cloned().collect()
    }

    /// Run a script function with arguments.
    pub fn call_function<T: Clone + Send + Sync + 'static>(
        &self,
        script_name: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T, ScriptError> {
        let scripts = self.scripts.read();
        let ast = scripts.get(script_name)
            .ok_or_else(|| ScriptError::NotFound(script_name.to_string()))?;

        self.engine
            .call_fn::<T>(&mut Scope::new(), ast, function, args)
            .map_err(|e| ScriptError::RuntimeError(e.to_string()))
    }

    /// Run a script function returning Dynamic.
    pub fn call_function_dynamic(
        &self,
        script_name: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<Dynamic, ScriptError> {
        let scripts = self.scripts.read();
        let ast = scripts.get(script_name)
            .ok_or_else(|| ScriptError::NotFound(script_name.to_string()))?;

        self.engine
            .call_fn::<Dynamic>(&mut Scope::new(), ast, function, args)
            .map_err(|e| ScriptError::RuntimeError(e.to_string()))
    }

    /// Run a script with a scope.
    pub fn run_with_scope(
        &self,
        script_name: &str,
        scope: &mut Scope,
    ) -> Result<Dynamic, ScriptError> {
        let scripts = self.scripts.read();
        let ast = scripts.get(script_name)
            .ok_or_else(|| ScriptError::NotFound(script_name.to_string()))?;

        self.engine
            .run_ast_with_scope(scope, ast)
            .map_err(|e| ScriptError::RuntimeError(e.to_string()))?;

        // Return the result variable if set
        Ok(scope.get_value::<Dynamic>("result").unwrap_or(Dynamic::UNIT))
    }

    /// Evaluate a script expression.
    pub fn eval<T: Clone + Send + Sync + 'static>(&self, script: &str) -> Result<T, ScriptError> {
        self.engine
            .eval::<T>(script)
            .map_err(|e| ScriptError::RuntimeError(e.to_string()))
    }

    /// Get direct access to the Rhai engine for advanced usage.
    pub fn rhai_engine(&self) -> &Engine {
        &self.engine
    }
}

/// Context passed to mission scripts.
#[derive(Clone)]
pub struct MissionContext {
    pub mission_id: Uuid,
    pub mission_type: String,
    /// The script path for this mission (e.g., "missions/random/pirate_attack.rhai")
    pub script_path: String,
    pub current_state: String,
    pub data: rhai::Map,
    pub player_id: Uuid,
    pub player_reputation: i32,
    pub player_fame: i32,
    pub ship_id: Uuid,
    pub ship_hull: f32,
    pub ship_ammo: f32,
    pub ship_fuel: f32,
    pub crew_morale: f32,
    pub crew_experience: i32,
    pub sector_id: Uuid,
}

impl MissionContext {
    /// Convert to Rhai Dynamic map.
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = rhai::Map::new();
        map.insert("mission_id".into(), self.mission_id.to_string().into());
        map.insert("mission_type".into(), self.mission_type.clone().into());
        map.insert("script_path".into(), self.script_path.clone().into());
        map.insert("current_state".into(), self.current_state.clone().into());
        map.insert("data".into(), Dynamic::from(self.data.clone()));
        map.insert("player_id".into(), self.player_id.to_string().into());
        map.insert("player_reputation".into(), (self.player_reputation as i64).into());
        map.insert("player_fame".into(), (self.player_fame as i64).into());
        map.insert("ship_id".into(), self.ship_id.to_string().into());
        map.insert("ship_hull".into(), (self.ship_hull as f64).into());
        map.insert("ship_ammo".into(), (self.ship_ammo as f64).into());
        map.insert("ship_fuel".into(), (self.ship_fuel as f64).into());
        map.insert("crew_morale".into(), (self.crew_morale as f64).into());
        map.insert("crew_experience".into(), (self.crew_experience as i64).into());
        map.insert("sector_id".into(), self.sector_id.to_string().into());
        Dynamic::from(map)
    }
}

/// Result from mission script execution.
#[derive(Debug, Clone)]
pub struct MissionResult {
    pub success: bool,
    pub new_state: String,
    pub reputation_change: i32,
    pub fame_change: i32,
    pub narrative: String,
    pub choices: Option<Vec<MissionChoice>>,
    pub spawn_combat: bool,
    pub data_updates: rhai::Map,
}

/// A mission choice option.
#[derive(Debug, Clone)]
pub struct MissionChoice {
    pub id: String,
    pub text: String,
    pub requirements: Vec<ChoiceRequirement>,
}

/// Requirement for a choice.
#[derive(Debug, Clone)]
pub enum ChoiceRequirement {
    MinExperience(i32),
    MinFame(i32),
    MinReputation(i32),
    MinHull(f32),
    MinAmmo(f32),
}

impl MissionResult {
    /// Parse from Rhai Dynamic.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<rhai::Map>()?;

        Some(Self {
            success: map.get("success")
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false),
            new_state: map.get("new_state")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_else(|| "complete".to_string()),
            reputation_change: map.get("reputation_change")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0) as i32,
            fame_change: map.get("fame_change")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0) as i32,
            narrative: map.get("narrative")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default(),
            choices: None, // Parse separately if present
            spawn_combat: map.get("spawn_combat")
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false),
            data_updates: map.get("data")
                .and_then(|v| v.clone().try_cast::<rhai::Map>())
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ScriptEngine::new("scripts");
        assert!(engine.loaded_scripts().is_empty());
    }

    #[test]
    fn test_eval() {
        let engine = ScriptEngine::new("scripts");
        let result: i64 = engine.eval("40 + 2").unwrap();
        assert_eq!(result, 42);
    }
}
