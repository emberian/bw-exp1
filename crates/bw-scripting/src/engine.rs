//! Rhai scripting engine setup
//!
//! Central engine with all bindings registered.

use parking_lot::RwLock;
use rhai::{Engine, Scope, AST, Dynamic, EvalAltResult, Position};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

use crate::bindings;

/// Detailed error location in a script.
#[derive(Debug, Clone)]
pub struct ScriptLocation {
    pub script: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ScriptLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}:{}", self.script, self.line, self.column)
        } else {
            write!(f, "{}", self.script)
        }
    }
}

/// Scripting engine errors with enhanced context.
#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("Script not found: {0}")]
    NotFound(String),

    #[error("{location}: compilation error: {message}")]
    CompileError {
        location: ScriptLocation,
        message: String,
    },

    #[error("{location}: {message}")]
    RuntimeError {
        location: ScriptLocation,
        message: String,
        /// Call stack at the point of error (function names)
        stack_trace: Vec<String>,
    },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("No result returned from script")]
    NoResult,

    #[error("Validation error in {script}: {message}")]
    ValidationError {
        script: String,
        message: String,
        missing_functions: Vec<String>,
    },
}

impl ScriptError {
    /// Create a compile error from a Rhai parse error.
    pub fn from_parse_error(script: &str, err: rhai::ParseError) -> Self {
        let pos = err.position();
        Self::CompileError {
            location: ScriptLocation {
                script: script.to_string(),
                line: pos.line().unwrap_or(0),
                column: pos.position().unwrap_or(0),
            },
            message: err.to_string(),
        }
    }

    /// Create a compile error from a Rhai file compilation error (Box<EvalAltResult>).
    pub fn from_compile_file_error(script: &str, err: Box<EvalAltResult>) -> Self {
        // compile_file errors are typically IO or parse errors
        let position = err.position();
        Self::CompileError {
            location: ScriptLocation {
                script: script.to_string(),
                line: position.line().unwrap_or(0),
                column: position.position().unwrap_or(0),
            },
            message: err.to_string(),
        }
    }

    /// Create a runtime error from a Rhai evaluation error.
    pub fn from_eval_error(script: &str, err: Box<EvalAltResult>) -> Self {
        let (message, position, stack) = Self::extract_error_info(&err);
        Self::RuntimeError {
            location: ScriptLocation {
                script: script.to_string(),
                line: position.line().unwrap_or(0),
                column: position.position().unwrap_or(0),
            },
            message,
            stack_trace: stack,
        }
    }

    /// Extract detailed information from a Rhai error.
    fn extract_error_info(err: &EvalAltResult) -> (String, Position, Vec<String>) {
        let mut stack = Vec::new();
        let mut current = err;
        let mut position = Position::NONE;

        // Walk the error chain to build stack trace
        loop {
            match current {
                EvalAltResult::ErrorInFunctionCall(name, _, inner, pos) => {
                    stack.push(name.clone());
                    if position.is_none() {
                        position = *pos;
                    }
                    current = inner;
                }
                EvalAltResult::ErrorInModule(path, inner, pos) => {
                    stack.push(format!("module:{}", path));
                    if position.is_none() {
                        position = *pos;
                    }
                    current = inner;
                }
                _ => {
                    if position.is_none() {
                        position = current.position();
                    }
                    break;
                }
            }
        }

        // Get the root error message
        let message = match current {
            EvalAltResult::ErrorFunctionNotFound(name, _) => {
                format!("function not found: {}", name)
            }
            EvalAltResult::ErrorVariableNotFound(name, _) => {
                format!("variable not found: {}", name)
            }
            EvalAltResult::ErrorIndexNotFound(idx, _) => {
                format!("index not found: {}", idx)
            }
            EvalAltResult::ErrorMismatchDataType(expected, actual, _) => {
                format!("type mismatch: expected {}, got {}", expected, actual)
            }
            EvalAltResult::ErrorArithmetic(msg, _) => {
                format!("arithmetic error: {}", msg)
            }
            EvalAltResult::ErrorRuntime(msg, _) => {
                format!("runtime error: {}", msg)
            }
            other => other.to_string(),
        };

        (message, position, stack)
    }

    /// Get the stack trace as a formatted string.
    pub fn stack_trace_string(&self) -> Option<String> {
        match self {
            Self::RuntimeError { stack_trace, .. } if !stack_trace.is_empty() => {
                Some(stack_trace.iter()
                    .enumerate()
                    .map(|(i, name)| format!("  {}: {}", i, name))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            _ => None,
        }
    }

    /// Get a detailed error message including stack trace.
    pub fn detailed_message(&self) -> String {
        match self {
            Self::RuntimeError { location, message, stack_trace } => {
                let mut result = format!("{}: {}", location, message);
                if !stack_trace.is_empty() {
                    result.push_str("\n  Call stack:\n");
                    for (i, name) in stack_trace.iter().enumerate() {
                        result.push_str(&format!("    {}: {}\n", i, name));
                    }
                }
                result
            }
            other => other.to_string(),
        }
    }

    /// Create a simple runtime error (without location info).
    /// Use this for errors generated outside of actual script execution.
    pub fn runtime(script: &str, message: impl Into<String>) -> Self {
        Self::RuntimeError {
            location: ScriptLocation {
                script: script.to_string(),
                line: 0,
                column: 0,
            },
            message: message.into(),
            stack_trace: Vec::new(),
        }
    }

    /// Check if this error is a "function not found" error.
    pub fn is_function_not_found(&self) -> bool {
        match self {
            Self::RuntimeError { message, .. } => message.contains("function not found"),
            _ => false,
        }
    }
}

// =============================================================================
// Script Profiling
// =============================================================================

/// Execution statistics for a script.
#[derive(Debug, Clone, Default)]
pub struct ScriptStats {
    /// Total number of calls
    pub call_count: u64,
    /// Total execution time across all calls
    pub total_time: Duration,
    /// Minimum execution time
    pub min_time: Duration,
    /// Maximum execution time
    pub max_time: Duration,
    /// Last execution time
    pub last_time: Duration,
}

impl ScriptStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self {
            call_count: 0,
            total_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            last_time: Duration::ZERO,
        }
    }

    /// Record an execution.
    pub fn record(&mut self, duration: Duration) {
        self.call_count += 1;
        self.total_time += duration;
        self.last_time = duration;

        if duration < self.min_time {
            self.min_time = duration;
        }
        if duration > self.max_time {
            self.max_time = duration;
        }
    }

    /// Get average execution time.
    pub fn avg_time(&self) -> Duration {
        if self.call_count == 0 {
            Duration::ZERO
        } else {
            self.total_time / self.call_count as u32
        }
    }

    /// Reset stats.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Profiling data for the script engine.
#[derive(Debug, Default)]
pub struct ScriptProfiler {
    /// Stats per script
    stats: RwLock<HashMap<String, ScriptStats>>,
    /// Whether profiling is enabled
    enabled: bool,
}

impl ScriptProfiler {
    /// Create a new profiler (disabled by default).
    pub fn new() -> Self {
        Self {
            stats: RwLock::new(HashMap::new()),
            enabled: false,
        }
    }

    /// Create an enabled profiler.
    pub fn enabled() -> Self {
        Self {
            stats: RwLock::new(HashMap::new()),
            enabled: true,
        }
    }

    /// Enable profiling.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable profiling.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if profiling is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record an execution (if enabled).
    pub fn record(&self, script: &str, function: &str, duration: Duration) {
        if !self.enabled {
            return;
        }

        let key = format!("{}::{}", script, function);
        let mut stats = self.stats.write();
        stats.entry(key).or_insert_with(ScriptStats::new).record(duration);
    }

    /// Get stats for a specific script/function.
    pub fn get_stats(&self, script: &str, function: &str) -> Option<ScriptStats> {
        let key = format!("{}::{}", script, function);
        self.stats.read().get(&key).cloned()
    }

    /// Get all stats.
    pub fn all_stats(&self) -> HashMap<String, ScriptStats> {
        self.stats.read().clone()
    }

    /// Get stats sorted by total time (descending).
    pub fn top_by_total_time(&self, limit: usize) -> Vec<(String, ScriptStats)> {
        let stats = self.stats.read();
        let mut entries: Vec<_> = stats.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
        entries.truncate(limit);
        entries
    }

    /// Get stats sorted by average time (descending).
    pub fn top_by_avg_time(&self, limit: usize) -> Vec<(String, ScriptStats)> {
        let stats = self.stats.read();
        let mut entries: Vec<_> = stats.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| b.1.avg_time().cmp(&a.1.avg_time()));
        entries.truncate(limit);
        entries
    }

    /// Get stats sorted by call count (descending).
    pub fn top_by_call_count(&self, limit: usize) -> Vec<(String, ScriptStats)> {
        let stats = self.stats.read();
        let mut entries: Vec<_> = stats.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));
        entries.truncate(limit);
        entries
    }

    /// Reset all stats.
    pub fn reset(&self) {
        self.stats.write().clear();
    }

    /// Format a stats report.
    pub fn format_report(&self, limit: usize) -> String {
        let top = self.top_by_total_time(limit);
        if top.is_empty() {
            return "No script profiling data collected.".to_string();
        }

        let mut report = String::from("Script Profiling Report (by total time):\n");
        report.push_str(&format!("{:<50} {:>8} {:>12} {:>12} {:>12}\n",
            "Script::Function", "Calls", "Total", "Avg", "Max"));
        report.push_str(&"-".repeat(96));
        report.push('\n');

        for (name, stats) in top {
            report.push_str(&format!("{:<50} {:>8} {:>12.2?} {:>12.2?} {:>12.2?}\n",
                if name.len() > 50 { &name[..50] } else { &name },
                stats.call_count,
                stats.total_time,
                stats.avg_time(),
                stats.max_time));
        }

        report
    }
}

// =============================================================================
// Script Engine
// =============================================================================

/// The main scripting engine.
pub struct ScriptEngine {
    engine: Engine,
    scripts: RwLock<HashMap<String, AST>>,
    scripts_dir: String,
    modules_dir: Option<String>,
    profiler: ScriptProfiler,
}

impl ScriptEngine {
    /// Create a new script engine with all bindings.
    pub fn new(scripts_dir: impl Into<String>) -> Self {
        Self::with_modules(scripts_dir, None::<String>)
    }

    /// Create a new script engine with module support.
    ///
    /// Scripts can import shared modules using:
    /// ```rhai
    /// import "utils" as utils;
    /// import "combat/helpers" as combat;
    /// ```
    pub fn with_modules(
        scripts_dir: impl Into<String>,
        modules_dir: Option<impl Into<String>>,
    ) -> Self {
        let scripts_dir = scripts_dir.into();
        let modules_dir = modules_dir.map(|d| d.into());

        let mut engine = Engine::new();

        // Safety limits for untrusted scripts
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(64, 64);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(1_000);
        engine.set_max_map_size(500);
        engine.set_max_modules(50); // Limit imported modules

        // Set up module resolver
        let resolver = Self::create_module_resolver(&scripts_dir, modules_dir.as_deref());
        engine.set_module_resolver(resolver);

        // Register all API bindings
        bindings::register_all(&mut engine);

        Self {
            engine,
            scripts: RwLock::new(HashMap::new()),
            scripts_dir,
            modules_dir,
            profiler: ScriptProfiler::new(),
        }
    }

    /// Enable profiling.
    pub fn enable_profiling(&mut self) {
        self.profiler.enable();
    }

    /// Disable profiling.
    pub fn disable_profiling(&mut self) {
        self.profiler.disable();
    }

    /// Get profiling stats.
    pub fn profiler(&self) -> &ScriptProfiler {
        &self.profiler
    }

    /// Create a module resolver that looks in both modules dir and scripts dir.
    fn create_module_resolver(
        scripts_dir: &str,
        modules_dir: Option<&str>,
    ) -> rhai::module_resolvers::FileModuleResolver {
        // Primary search path is the modules directory (or scripts/modules if not specified)
        let primary_path = modules_dir
            .map(|d| d.to_string())
            .unwrap_or_else(|| format!("{}/modules", scripts_dir));

        let mut resolver = rhai::module_resolvers::FileModuleResolver::new_with_path(primary_path);

        // Set file extension
        resolver.set_extension("rhai");

        resolver
    }

    /// Get the modules directory path.
    pub fn modules_dir(&self) -> String {
        self.modules_dir.clone()
            .unwrap_or_else(|| format!("{}/modules", self.scripts_dir))
    }

    /// Load a script from file.
    pub fn load_script(&self, name: &str) -> Result<(), ScriptError> {
        let path = std::path::PathBuf::from(format!("{}/{}", self.scripts_dir, name));
        let ast = self.engine.compile_file(path)
            .map_err(|e| ScriptError::from_compile_file_error(name, e))?;

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

    /// Validate a loaded script against a contract.
    pub fn validate_script(
        &self,
        name: &str,
        contract: &crate::validation::ScriptContract,
    ) -> Result<crate::validation::ValidationResult, ScriptError> {
        let scripts = self.scripts.read();
        let ast = scripts.get(name)
            .ok_or_else(|| ScriptError::NotFound(name.to_string()))?;

        Ok(crate::validation::validate_script(&self.engine, ast, contract, name))
    }

    /// Validate a script using an inferred contract based on its path.
    pub fn validate_script_auto(&self, name: &str) -> Result<crate::validation::ValidationResult, ScriptError> {
        let contract = crate::validation::infer_contract(name);
        self.validate_script(name, &contract)
    }

    /// Load a script and validate it, returning error if validation fails.
    pub fn load_and_validate(
        &self,
        name: &str,
        contract: &crate::validation::ScriptContract,
    ) -> Result<(), ScriptError> {
        self.load_script(name)?;
        let result = self.validate_script(name, contract)?;

        if !result.is_valid() {
            // Remove the invalid script
            self.scripts.write().remove(name);

            return Err(ScriptError::ValidationError {
                script: name.to_string(),
                message: result.error_message().unwrap_or_default(),
                missing_functions: result.missing_required.iter().map(|f| f.name.clone()).collect(),
            });
        }

        // Log warnings if any
        for warning in &result.warnings {
            tracing::warn!(script = name, "{}", warning);
        }

        Ok(())
    }

    /// Validate all loaded scripts, returning a summary of results.
    pub fn validate_all(&self) -> Vec<crate::validation::ValidationResult> {
        let script_names: Vec<String> = self.scripts.read().keys().cloned().collect();
        let mut results = Vec::new();

        for name in script_names {
            let contract = crate::validation::infer_contract(&name);
            if let Ok(result) = self.validate_script(&name, &contract) {
                if !result.is_valid() || !result.warnings.is_empty() {
                    results.push(result);
                }
            }
        }

        results
    }

    /// Reload a script from disk (for hot-reload).
    /// Returns list of warnings on success.
    pub fn reload_script(&self, name: &str) -> Result<Vec<String>, ScriptError> {
        // First load the new version
        let path = std::path::PathBuf::from(format!("{}/{}", self.scripts_dir, name));
        let ast = self.engine.compile_file(path)
            .map_err(|e| ScriptError::from_compile_file_error(name, e))?;

        // Validate before replacing
        let contract = crate::validation::infer_contract(name);
        let result = crate::validation::validate_script(&self.engine, &ast, &contract, name);

        if !result.is_valid() {
            return Err(ScriptError::ValidationError {
                script: name.to_string(),
                message: result.error_message().unwrap_or_default(),
                missing_functions: result.missing_required.iter().map(|f| f.name.clone()).collect(),
            });
        }

        // Replace the old script
        self.scripts.write().insert(name.to_string(), ast);
        tracing::info!(script = name, "Script reloaded successfully");

        Ok(result.warnings)
    }

    /// Reload archetype definitions (ships, weapons).
    /// Returns (ships_loaded, weapons_loaded, errors) tuple.
    pub fn reload_definitions(&self) -> Result<(usize, usize, Vec<String>), ScriptError> {
        // Reload the archetype definitions
        // This would reload from scripts/definitions/*.rhai
        let definitions_dir = format!("{}/definitions", self.scripts_dir);
        let path = Path::new(&definitions_dir);

        if !path.exists() {
            return Err(ScriptError::NotFound("definitions directory".to_string()));
        }

        let mut ships_loaded = 0;
        let mut weapons_loaded = 0;
        let mut errors = Vec::new();

        // Load all .rhai files in definitions directory
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.extension().is_some_and(|ext| ext == "rhai") {
                let name = format!("definitions/{}", entry.file_name().to_string_lossy());

                match self.load_script(&name) {
                    Ok(()) => {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        if file_name.contains("ship") {
                            ships_loaded += 1;
                        } else if file_name.contains("weapon") {
                            weapons_loaded += 1;
                        } else {
                            ships_loaded += 1; // Default to counting as ship definitions
                        }
                        tracing::info!(script = %name, "Reloaded definition script");
                    }
                    Err(e) => {
                        errors.push(format!("{}: {}", name, e));
                    }
                }
            }
        }

        tracing::info!(
            ships = ships_loaded,
            weapons = weapons_loaded,
            errors = errors.len(),
            "Reloaded archetype definitions"
        );

        Ok((ships_loaded, weapons_loaded, errors))
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

        let start = Instant::now();
        let result = self.engine
            .call_fn::<T>(&mut Scope::new(), ast, function, args)
            .map_err(|e| ScriptError::from_eval_error(script_name, e));
        self.profiler.record(script_name, function, start.elapsed());

        result
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

        let start = Instant::now();
        let result = self.engine
            .call_fn::<Dynamic>(&mut Scope::new(), ast, function, args)
            .map_err(|e| ScriptError::from_eval_error(script_name, e));
        self.profiler.record(script_name, function, start.elapsed());

        result
    }

    /// Run a script function with a custom engine (e.g., debug-enabled).
    ///
    /// This allows executing scripts with a specially configured engine,
    /// such as one with debugging callbacks registered.
    pub fn call_function_dynamic_with_engine(
        &self,
        custom_engine: &Engine,
        script_name: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<Dynamic, ScriptError> {
        let scripts = self.scripts.read();
        let ast = scripts.get(script_name)
            .ok_or_else(|| ScriptError::NotFound(script_name.to_string()))?;

        let start = Instant::now();
        let result = custom_engine
            .call_fn::<Dynamic>(&mut Scope::new(), ast, function, args)
            .map_err(|e| ScriptError::from_eval_error(script_name, e));
        self.profiler.record(script_name, function, start.elapsed());

        result
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

        let start = Instant::now();
        let result = self.engine
            .run_ast_with_scope(scope, ast)
            .map_err(|e| ScriptError::from_eval_error(script_name, e));
        self.profiler.record(script_name, "<run>", start.elapsed());

        result?;

        // Return the result variable if set
        Ok(scope.get_value::<Dynamic>("result").unwrap_or(Dynamic::UNIT))
    }

    /// Evaluate a script expression.
    pub fn eval<T: Clone + Send + Sync + 'static>(&self, script: &str) -> Result<T, ScriptError> {
        let start = Instant::now();
        let result = self.engine
            .eval::<T>(script)
            .map_err(|e| ScriptError::from_eval_error("<eval>", e));
        self.profiler.record("<eval>", "<inline>", start.elapsed());

        result
    }

    /// Get direct access to the Rhai engine for advanced usage.
    pub fn rhai_engine(&self) -> &Engine {
        &self.engine
    }

    /// Create a new Rhai engine with all bindings registered.
    ///
    /// This creates a fresh engine configured identically to the internal one,
    /// useful for scenarios like debugging where a separate engine is needed.
    pub fn create_engine_with_bindings(&self) -> Engine {
        let mut engine = Engine::new();

        // Safety limits for untrusted scripts
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(64, 64);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(1_000);
        engine.set_max_map_size(500);
        engine.set_max_modules(50);

        // Set up module resolver
        let resolver = Self::create_module_resolver(&self.scripts_dir, self.modules_dir.as_deref());
        engine.set_module_resolver(resolver);

        // Register all API bindings
        bindings::register_all(&mut engine);

        engine
    }

    /// Get the scripts directory.
    pub fn scripts_dir(&self) -> &str {
        &self.scripts_dir
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

// MissionChoice and ChoiceRequirement are defined in mission_runner module
pub use crate::mission_runner::{MissionChoice, ChoiceRequirement};

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
