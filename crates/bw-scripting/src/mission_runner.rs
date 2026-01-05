//! Mission state machine runner
//!
//! Executes Rhai mission scripts and manages state transitions.
//! Each mission is a state machine with:
//! - States (start, choices, combat, resolution)
//! - Transitions triggered by player choices or events
//! - Outcomes affecting resources and reputation

use rhai::{Dynamic, Map};
use serde::{Deserialize, Serialize};

use crate::engine::{MissionContext, ScriptEngine, ScriptError};

/// Manages execution of mission scripts.
pub struct MissionRunner<'a> {
    engine: &'a ScriptEngine,
}

impl<'a> MissionRunner<'a> {
    pub fn new(engine: &'a ScriptEngine) -> Self {
        Self { engine }
    }

    /// Start a new mission, calling on_start in the script.
    pub fn start_mission(&self, ctx: &MissionContext) -> Result<MissionOutcome, ScriptError> {
        let script_name = &ctx.script_path;

        if !self.engine.has_script(script_name) {
            return Err(ScriptError::NotFound(script_name.clone()));
        }

        let ctx_dynamic = ctx.to_dynamic();
        let result = self.engine.call_function_dynamic(
            script_name,
            "on_start",
            (ctx_dynamic,),
        )?;

        MissionOutcome::from_dynamic(result)
            .ok_or_else(|| ScriptError::runtime(script_name, "Invalid mission result"))
    }

    /// Process a player choice, calling on_choice in the script.
    pub fn process_choice(
        &self,
        ctx: &MissionContext,
        choice_id: &str,
    ) -> Result<MissionOutcome, ScriptError> {
        let script_name = &ctx.script_path;

        if !self.engine.has_script(script_name) {
            return Err(ScriptError::NotFound(script_name.clone()));
        }

        let ctx_dynamic = ctx.to_dynamic();
        let result = self.engine.call_function_dynamic(
            script_name,
            "on_choice",
            (ctx_dynamic, choice_id.to_string()),
        )?;

        MissionOutcome::from_dynamic(result)
            .ok_or_else(|| ScriptError::runtime(script_name, "Invalid mission result"))
    }

    /// Process combat resolution, calling on_combat_resolved in the script.
    pub fn process_combat_result(
        &self,
        ctx: &MissionContext,
        player_won: bool,
        enemy_fled: bool,
    ) -> Result<MissionOutcome, ScriptError> {
        let script_name = &ctx.script_path;

        if !self.engine.has_script(script_name) {
            return Err(ScriptError::NotFound(script_name.clone()));
        }

        let ctx_dynamic = ctx.to_dynamic();

        // Build combat_result map as expected by scripts
        let mut combat_result = rhai::Map::new();
        combat_result.insert("player_won".into(), player_won.into());
        combat_result.insert("enemy_fled".into(), enemy_fled.into());
        let combat_result_dynamic = rhai::Dynamic::from(combat_result);

        let result = self.engine.call_function_dynamic(
            script_name,
            "on_combat_resolved",
            (ctx_dynamic, combat_result_dynamic),
        )?;

        MissionOutcome::from_dynamic(result)
            .ok_or_else(|| ScriptError::runtime(script_name, "Invalid mission result"))
    }

    /// Process a timed event (for missions with time pressure).
    pub fn process_tick(
        &self,
        ctx: &MissionContext,
        elapsed_seconds: f64,
    ) -> Result<Option<MissionOutcome>, ScriptError> {
        let script_name = &ctx.script_path;

        if !self.engine.has_script(script_name) {
            return Ok(None);
        }

        // Check if on_tick exists
        let ctx_dynamic = ctx.to_dynamic();
        match self.engine.call_function_dynamic(
            script_name,
            "on_tick",
            (ctx_dynamic, elapsed_seconds),
        ) {
            Ok(result) => {
                if result.is_unit() {
                    Ok(None)
                } else {
                    Ok(MissionOutcome::from_dynamic(result))
                }
            }
            Err(e) if e.is_function_not_found() => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Outcome from a mission script execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionOutcome {
    /// New state of the mission
    pub new_state: MissionState,

    /// Narrative text to display
    pub narrative: String,

    /// Choices available (if in choice state)
    pub choices: Vec<MissionChoice>,

    /// Resource changes to apply
    pub resource_changes: ResourceChanges,

    /// Combat to spawn (if any)
    pub spawn_combat: Option<CombatSpawn>,

    /// Whether mission is complete
    pub is_complete: bool,

    /// Whether mission was successful (if complete)
    pub success: bool,

    /// Updated mission data (persisted)
    pub data_updates: Map,
}

impl MissionOutcome {
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        // Parse state from either "state" or "new_state" field
        let new_state = map.get("new_state")
            .or_else(|| map.get("state"))
            .and_then(|v| v.clone().into_string().ok())
            .map(|s| MissionState::from_string(&s))
            .unwrap_or(MissionState::InProgress);

        let narrative = map.get("narrative")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();

        let choices = map.get("choices")
            .and_then(|v| v.clone().into_array().ok())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(MissionChoice::from_dynamic)
                    .collect()
            })
            .unwrap_or_default();

        // Parse resource changes either from "resources" map or individual fields
        let resource_changes = map.get("resources")
            .and_then(|v| ResourceChanges::from_dynamic(v.clone()))
            .unwrap_or_else(|| ResourceChanges::from_dynamic(Dynamic::from(map.clone())).unwrap_or_default());

        // Parse combat spawn either from "combat" map or "spawn_combat" boolean
        let spawn_combat = map.get("combat")
            .and_then(|v| CombatSpawn::from_dynamic(v.clone()))
            .or_else(|| {
                let spawns = map.get("spawn_combat")
                    .and_then(|v| v.as_bool().ok())
                    .unwrap_or(false);
                if spawns {
                    Some(CombatSpawn {
                        enemy_type: "unknown".to_string(),
                        enemy_count: 1,
                        difficulty: 1.0,
                        can_flee: true,
                        flee_penalty: None,
                    })
                } else {
                    None
                }
            });

        // Check completion - either explicit "complete" or state is a completion state
        let is_complete = map.get("complete")
            .and_then(|v| v.as_bool().ok())
            .unwrap_or_else(|| {
                matches!(new_state, MissionState::CompletedSuccess | MissionState::CompletedFailure)
                    || map.get("new_state")
                        .and_then(|v| v.clone().into_string().ok())
                        .map(|s| s == "complete")
                        .unwrap_or(false)
            });

        let success = map.get("success")
            .and_then(|v| v.as_bool().ok())
            .unwrap_or(false);

        let data_updates = map.get("data")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap_or_default();

        Some(Self {
            new_state,
            narrative,
            choices,
            resource_changes,
            spawn_combat,
            is_complete,
            success,
            data_updates,
        })
    }
}

/// Mission state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionState {
    /// Just started, showing initial narrative
    Started,
    /// Awaiting player choice
    AwaitingChoice,
    /// In combat
    InCombat,
    /// Processing (e.g., skill check)
    InProgress,
    /// Mission complete (success)
    CompletedSuccess,
    /// Mission complete (failure)
    CompletedFailure,
    /// Mission abandoned
    Abandoned,
    /// Custom state from script
    Custom(String),
}

impl MissionState {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "started" => Self::Started,
            "awaiting_choice" | "choice" => Self::AwaitingChoice,
            "in_combat" | "combat" => Self::InCombat,
            "in_progress" | "progress" => Self::InProgress,
            "completed_success" | "success" => Self::CompletedSuccess,
            "completed_failure" | "failure" => Self::CompletedFailure,
            "abandoned" => Self::Abandoned,
            other => Self::Custom(other.to_string()),
        }
    }

    /// Returns the string representation of this state.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Started => "started",
            Self::AwaitingChoice => "awaiting_choice",
            Self::InCombat => "in_combat",
            Self::InProgress => "in_progress",
            Self::CompletedSuccess => "completed_success",
            Self::CompletedFailure => "completed_failure",
            Self::Abandoned => "abandoned",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for MissionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A choice the player can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionChoice {
    pub id: String,
    pub text: String,
    pub requirements: Vec<ChoiceRequirement>,
    pub tooltip: Option<String>,
}

impl MissionChoice {
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = map.get("id")
            .and_then(|v| v.clone().into_string().ok())?;

        let text = map.get("text")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_else(|| id.clone());

        let requirements = map.get("requirements")
            .and_then(|v| v.clone().into_array().ok())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(ChoiceRequirement::from_dynamic)
                    .collect()
            })
            .unwrap_or_default();

        let tooltip = map.get("tooltip")
            .and_then(|v| v.clone().into_string().ok());

        Some(Self { id, text, requirements, tooltip })
    }

    /// Check if player meets requirements.
    pub fn is_available(&self, ctx: &MissionContext) -> bool {
        self.requirements.iter().all(|req| req.is_met(ctx))
    }
}

/// Requirement for a choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChoiceRequirement {
    MinExperience(i32),
    MinFame(i32),
    MinReputation(i32),
    MinHull(f32),
    MinAmmo(f32),
    MinFuel(f32),
    MinMorale(f32),
    HasItem(String),
    Custom(String, i64),
}

impl ChoiceRequirement {
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        if let Some(v) = map.get("min_experience") {
            return Some(Self::MinExperience(v.as_int().ok()? as i32));
        }
        if let Some(v) = map.get("min_fame") {
            return Some(Self::MinFame(v.as_int().ok()? as i32));
        }
        if let Some(v) = map.get("min_reputation") {
            return Some(Self::MinReputation(v.as_int().ok()? as i32));
        }
        if let Some(v) = map.get("min_hull") {
            return Some(Self::MinHull(v.as_float().ok()? as f32));
        }
        if let Some(v) = map.get("min_ammo") {
            return Some(Self::MinAmmo(v.as_float().ok()? as f32));
        }
        if let Some(v) = map.get("min_fuel") {
            return Some(Self::MinFuel(v.as_float().ok()? as f32));
        }
        if let Some(v) = map.get("min_morale") {
            return Some(Self::MinMorale(v.as_float().ok()? as f32));
        }
        if let Some(v) = map.get("has_item") {
            return Some(Self::HasItem(v.clone().into_string().ok()?));
        }

        // Custom requirement
        if let (Some(name), Some(val)) = (map.get("custom"), map.get("value")) {
            return Some(Self::Custom(
                name.clone().into_string().ok()?,
                val.as_int().ok()?,
            ));
        }

        None
    }

    pub fn is_met(&self, ctx: &MissionContext) -> bool {
        match self {
            Self::MinExperience(v) => ctx.crew_experience >= *v,
            Self::MinFame(v) => ctx.player_fame >= *v,
            Self::MinReputation(v) => ctx.player_reputation >= *v,
            Self::MinHull(v) => ctx.ship_hull >= *v,
            Self::MinAmmo(v) => ctx.ship_ammo >= *v,
            Self::MinFuel(v) => ctx.ship_fuel >= *v,
            Self::MinMorale(v) => ctx.crew_morale >= *v,
            Self::HasItem(item) => {
                tracing::warn!("Inventory system not implemented, assuming player has item: {}", item);
                true // Allow missions to proceed until inventory is implemented
            }
            Self::Custom(_, _) => true, // Custom logic in script
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::MinExperience(v) => format!("Requires {} experience", v),
            Self::MinFame(v) => format!("Requires {} fame", v),
            Self::MinReputation(v) => format!("Requires {} reputation", v),
            Self::MinHull(v) => format!("Requires {}% hull", v),
            Self::MinAmmo(v) => format!("Requires {}% ammunition", v),
            Self::MinFuel(v) => format!("Requires {}% fuel", v),
            Self::MinMorale(v) => format!("Requires {}% morale", v),
            Self::HasItem(item) => format!("Requires {}", item),
            Self::Custom(name, _) => format!("Requires: {}", name),
        }
    }
}

/// Resource changes from mission outcome.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceChanges {
    pub reputation: i32,
    pub fame: i32,
    pub ammunition: f32,
    pub fuel: f32,
    pub morale: f32,
    pub experience: i32,
    pub hull: f32,
    pub shields: f32,
}

impl ResourceChanges {
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        Some(Self {
            // Handle both "reputation" and "reputation_change" field names
            reputation: map.get("reputation_change")
                .or_else(|| map.get("reputation"))
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0) as i32,
            // Handle both "fame" and "fame_change" field names
            fame: map.get("fame_change")
                .or_else(|| map.get("fame"))
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0) as i32,
            ammunition: map.get("ammunition")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(0.0) as f32,
            fuel: map.get("fuel")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(0.0) as f32,
            morale: map.get("morale")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(0.0) as f32,
            experience: map.get("experience")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0) as i32,
            hull: map.get("hull")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(0.0) as f32,
            shields: map.get("shields")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(0.0) as f32,
        })
    }

    /// Check if any resources would go critical.
    pub fn would_cause_critical(&self, ctx: &MissionContext) -> Option<String> {
        if ctx.player_reputation + self.reputation <= 0 {
            return Some("Reputation would drop to zero (game over)".to_string());
        }
        if ctx.ship_ammo + self.ammunition <= 0.0 {
            return Some("Ammunition would be depleted".to_string());
        }
        if ctx.ship_fuel + self.fuel <= 0.0 {
            return Some("Fuel would be depleted".to_string());
        }
        None
    }
}

/// Combat spawn from mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatSpawn {
    pub enemy_type: String,
    pub enemy_count: u32,
    pub difficulty: f32,
    pub can_flee: bool,
    pub flee_penalty: Option<ResourceChanges>,
}

impl CombatSpawn {
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        Some(Self {
            enemy_type: map.get("enemy_type")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_else(|| "pirate".to_string()),
            enemy_count: map.get("enemy_count")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(1) as u32,
            difficulty: map.get("difficulty")
                .and_then(|v| v.as_float().ok())
                .unwrap_or(1.0) as f32,
            can_flee: map.get("can_flee")
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(true),
            flee_penalty: map.get("flee_penalty")
                .and_then(|v| ResourceChanges::from_dynamic(v.clone())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mission_state_parsing() {
        assert_eq!(MissionState::from_string("started"), MissionState::Started);
        assert_eq!(MissionState::from_string("choice"), MissionState::AwaitingChoice);
        assert_eq!(MissionState::from_string("combat"), MissionState::InCombat);
        assert_eq!(MissionState::from_string("success"), MissionState::CompletedSuccess);
        assert_eq!(MissionState::from_string("custom_state"), MissionState::Custom("custom_state".to_string()));
    }
}
