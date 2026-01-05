//! Cross-script dependency analysis.
//!
//! Provides dependency graph building, cycle detection, and missing action
//! detection for cross-script validation (E950/W950).

use std::collections::{HashMap, HashSet};

/// Information about a script's action registrations and calls.
#[derive(Debug, Clone, Default)]
pub struct ScriptActionInfo {
    /// Actions this script registers (action_name -> handler_name)
    pub provides: HashMap<String, String>,
    /// Actions this script calls (action_name -> positions where called)
    pub requires: HashSet<String>,
    /// Script path
    pub path: String,
}

/// Graph of script dependencies for cycle detection and missing action checking.
#[derive(Debug, Default)]
pub struct ScriptDependencyGraph {
    /// Map of script path -> action info
    pub scripts: HashMap<String, ScriptActionInfo>,
    /// All actions registered across all scripts (action_name -> script_path)
    pub action_providers: HashMap<String, String>,
}

impl ScriptDependencyGraph {
    /// Create a new empty dependency graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a script's info to the graph.
    pub fn add_script(&mut self, path: &str, info: ScriptActionInfo) {
        // Register all actions this script provides
        for action_name in info.provides.keys() {
            self.action_providers.insert(action_name.clone(), path.to_string());
        }
        self.scripts.insert(path.to_string(), info);
    }

    /// Find actions that are called but not registered by any script.
    pub fn find_missing_actions(&self) -> Vec<(String, String)> {
        // (action_name, script_that_requires_it)
        let mut missing = Vec::new();

        for (script_path, info) in &self.scripts {
            for action_name in &info.requires {
                // Skip if this action is provided by any script
                if !self.action_providers.contains_key(action_name) {
                    // Also skip common engine-provided actions
                    if !is_builtin_action(action_name) {
                        missing.push((action_name.clone(), script_path.clone()));
                    }
                }
            }
        }

        missing
    }

    /// Detect circular dependencies between scripts.
    /// Returns list of (script_a, script_b) pairs that form cycles.
    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        // Build dependency edges: script A depends on script B if A requires an action that B provides
        let mut deps: HashMap<&str, HashSet<&str>> = HashMap::new();

        for (script_path, info) in &self.scripts {
            let mut script_deps = HashSet::new();
            for action_name in &info.requires {
                if let Some(provider_path) = self.action_providers.get(action_name) {
                    if provider_path != script_path {
                        script_deps.insert(provider_path.as_str());
                    }
                }
            }
            if !script_deps.is_empty() {
                deps.insert(script_path.as_str(), script_deps);
            }
        }

        // Find cycles using DFS
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        fn dfs<'a>(
            node: &'a str,
            deps: &HashMap<&'a str, HashSet<&'a str>>,
            visited: &mut HashSet<&'a str>,
            rec_stack: &mut HashSet<&'a str>,
            path: &mut Vec<&'a str>,
            cycles: &mut Vec<Vec<String>>,
        ) {
            visited.insert(node);
            rec_stack.insert(node);
            path.push(node);

            if let Some(neighbors) = deps.get(node) {
                for &neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        dfs(neighbor, deps, visited, rec_stack, path, cycles);
                    } else if rec_stack.contains(neighbor) {
                        // Found a cycle - extract the cycle from path
                        let cycle_start = path.iter().position(|&n| n == neighbor).unwrap();
                        let cycle: Vec<String> = path[cycle_start..].iter().map(|s| s.to_string()).collect();
                        if cycle.len() > 1 {
                            cycles.push(cycle);
                        }
                    }
                }
            }

            path.pop();
            rec_stack.remove(node);
        }

        for script_path in self.scripts.keys() {
            if !visited.contains(script_path.as_str()) {
                dfs(script_path.as_str(), &deps, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }
}

/// Check if an action name is a built-in engine action (not from scripts).
fn is_builtin_action(name: &str) -> bool {
    // Common actions that might be called but not defined in scripts
    // (because they're handled by the engine or registered elsewhere)
    matches!(name,
        "login" | "logout" | "register" |
        "chat" | "whisper" |
        "ping" | "pong" |
        "heartbeat" |
        // Add more as needed
        _  if name.starts_with("__") // Internal actions
    )
}
