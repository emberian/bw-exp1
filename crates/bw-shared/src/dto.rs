//! Data Transfer Objects
//!
//! DTOs are simplified versions of models for network transfer.
//! They contain only the data clients need to see.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Player DTO (what clients see about players).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDto {
    pub id: Uuid,
    pub username: String,
    pub reputation: i32,
    pub fame: i32,
    pub faction_tag: String,
    pub squadron_tag: Option<String>,
    pub is_online: bool,
    /// Whether this player has admin/GM privileges
    #[serde(default)]
    pub is_admin: bool,
}

/// Ship DTO (what clients see about ships).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub ship_class: String,
    pub position: PositionDto,
    pub hull_percent: f32,
    pub shield_percent: f32,
    pub status: String,
    pub faction_tag: Option<String>,
    pub is_player: bool,
    pub is_hostile: bool,
}

/// Ship update DTO (delta update for existing ship).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipUpdateDto {
    pub id: Uuid,
    pub position: Option<PositionDto>,
    pub hull_percent: Option<f32>,
    pub shield_percent: Option<f32>,
    pub status: Option<String>,
}

/// Position DTO.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PositionDto {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Sector DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorDto {
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
    pub controlling_faction: Option<String>,
    pub locations: Vec<LocationDto>,
    pub adjacent_sectors: Vec<AdjacentSectorDto>,
}

/// Location DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationDto {
    pub id: Uuid,
    pub name: String,
    pub location_type: String,
    pub position: PositionDto,
    pub faction_tag: Option<String>,
    pub services: Vec<String>,
}

/// Adjacent sector info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacentSectorDto {
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
}

/// Mission DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDto {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub mission_type: String,
    pub status: String,
    pub priority: String,
    pub reputation_reward: i32,
    pub fame_reward: i32,
    pub is_high_profile: bool,
    pub expires_in_seconds: Option<u32>,
    pub progress: f32,
    pub can_accept: bool,
}

/// Mission update DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionUpdateDto {
    pub id: Uuid,
    pub status: Option<String>,
    pub progress: Option<f32>,
}

/// Choice DTO for mission choices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceDto {
    pub id: String,
    pub text: String,
    pub is_available: bool,
    pub requirement_text: Option<String>,
}

/// Game event DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEventDto {
    pub event_type: String,
    pub message: String,
    pub actor_name: Option<String>,
    pub target_name: Option<String>,
}

/// Combat event DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEventDto {
    pub event_type: String,
    pub attacker_name: String,
    pub target_name: String,
    pub damage: Option<f32>,
    pub hit: bool,
    pub message: String,
}

/// Squadron DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadronDto {
    pub id: Uuid,
    pub name: String,
    pub tag: String,
    pub motto: Option<String>,
    pub leader_name: String,
    pub member_count: u32,
    pub reputation_bonus: f32,
    pub fame_bonus: f32,
    pub is_at_war: bool,
}

/// Faction DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionDto {
    pub id: Uuid,
    pub name: String,
    pub tag: String,
    pub description: String,
    pub philosophy: String,
    pub color: String,
    pub standing: i32,
    pub rank: String,
}

// =============================================================================
// Performance Metrics DTOs
// =============================================================================

/// Timing metrics for a single tick phase.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhaseTimingDto {
    /// Phase name (e.g., "coroutines", "behaviors", "global")
    pub name: String,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Whether this phase was skipped (e.g., conditional phases like npc_spawn)
    #[serde(default)]
    pub skipped: bool,
}

/// Per-sector timing breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorTimingDto {
    pub sector_id: Uuid,
    pub sector_name: String,
    /// Total time spent in this sector (microseconds)
    pub total_us: u64,
    /// Sub-phase breakdown within sector
    pub phases: Vec<PhaseTimingDto>,
}

/// Complete tick metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickMetricsDto {
    /// Tick number
    pub tick: u64,
    /// Total tick duration (microseconds)
    pub total_us: u64,
    /// Budget (100ms = 100,000 us)
    pub budget_us: u64,
    /// Whether tick exceeded budget
    pub over_budget: bool,
    /// Top-level phase timings
    pub phases: Vec<PhaseTimingDto>,
    /// Per-sector breakdown
    pub sectors: Vec<SectorTimingDto>,
}

/// Rolling window of tick metrics for historical view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickMetricsHistoryDto {
    /// Last N tick metrics (typically 100)
    pub ticks: Vec<TickMetricsDto>,
    /// Average tick duration over window (microseconds)
    pub avg_duration_us: u64,
    /// 95th percentile tick duration
    pub p95_duration_us: u64,
    /// Maximum tick duration in window
    pub max_duration_us: u64,
    /// Number of ticks over budget in window
    pub over_budget_count: u32,
}

// =============================================================================
// Playtest DTOs
// =============================================================================

/// Configuration for forking state into a playtest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkConfigDto {
    /// Sectors to include (empty = all sectors)
    #[serde(default)]
    pub sectors: Vec<Uuid>,
    /// Whether to include all ships in selected sectors
    #[serde(default = "default_true")]
    pub include_ships: bool,
    /// Specific ships to include (overrides include_ships if non-empty)
    #[serde(default)]
    pub specific_ships: Vec<Uuid>,
    /// Whether to include missions
    #[serde(default = "default_true")]
    pub include_missions: bool,
    /// Whether to include NPC ships
    #[serde(default = "default_true")]
    pub include_npcs: bool,
    /// Whether to include other players' ships (except invited players)
    #[serde(default)]
    pub include_other_players: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ForkConfigDto {
    fn default() -> Self {
        Self {
            sectors: vec![],
            include_ships: true,
            specific_ships: vec![],
            include_missions: true,
            include_npcs: true,
            include_other_players: false,
        }
    }
}

/// Configuration for promoting changes from playtest to live.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromoteConfigDto {
    /// Ship modifications to promote (by ship_id)
    #[serde(default)]
    pub ships: Vec<Uuid>,
    /// Player modifications to promote (by player_id)
    #[serde(default)]
    pub players: Vec<Uuid>,
    /// Whether to spawn new entities created in playtest
    #[serde(default)]
    pub spawn_new_entities: bool,
    /// Whether to apply entity deletions
    #[serde(default)]
    pub apply_deletions: bool,
}

/// Summary of a playtest for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytestSummaryDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub owner_name: String,
    pub participant_count: usize,
    pub created_at_tick: u64,
    pub current_tick: u64,
    pub paused: bool,
    pub time_scale: f32,
}

/// Full details of a playtest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytestDetailDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub owner_name: String,
    pub participants: Vec<PlaytestParticipantDto>,
    pub created_at_tick: u64,
    pub current_tick: u64,
    pub paused: bool,
    pub time_scale: f32,
    pub sector_count: usize,
    pub ship_count: usize,
    pub player_count: usize,
}

/// A participant in a playtest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytestParticipantDto {
    pub player_id: Uuid,
    pub player_name: String,
    pub is_gm: bool,
    pub ship_id: Uuid,
    pub sector_id: Uuid,
    pub is_connected: bool,
}

// =============================================================================
// Debug DTOs
// =============================================================================

/// Debug session target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DebugTargetDto {
    /// Debug within a specific playtest
    Playtest(Uuid),
    /// Debug on the live server
    Live,
}

/// Breakpoint DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointDto {
    pub id: Uuid,
    pub script: String,
    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
    pub hit_count: u64,
    pub enabled: bool,
}

/// Function breakpoint DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionBreakpointDto {
    pub id: Uuid,
    pub function_name: String,
    pub break_on_entry: bool,
    pub break_on_exit: bool,
    pub enabled: bool,
}

/// Stack frame DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrameDto {
    pub index: usize,
    pub function_name: String,
    pub source: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Variable DTO for inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDto {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub expandable: bool,
    pub path: String,
}

/// Entity context during debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityContextDto {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub entity_name: String,
    pub sector_id: Option<Uuid>,
}

/// Reason execution was paused.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PauseReasonDto {
    /// Hit a line breakpoint
    Breakpoint { breakpoint_id: Uuid },
    /// Hit a function entry breakpoint
    FunctionEntry { function_name: String },
    /// Hit a function exit breakpoint
    FunctionExit { function_name: String },
    /// Step completed
    Step,
    /// Manual pause requested
    Pause,
    /// Exception was thrown
    Exception { message: String },
}

/// Debug session info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSessionDto {
    pub session_id: Uuid,
    pub target: DebugTargetDto,
    pub is_paused: bool,
    pub breakpoint_count: usize,
}

// =============================================================================
// Schema Introspection DTOs
// =============================================================================

/// Summary of an archetype schema for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeSchemaDto {
    /// Type identifier (e.g., "ship", "weapon", "effect")
    pub archetype_type: String,
    /// Rust struct name (e.g., "ShipArchetype")
    pub name: String,
    /// Total number of fields
    pub field_count: usize,
    /// Number of required fields
    pub required_count: usize,
}

/// Detailed field information from an archetype schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchemaDto {
    /// Field name
    pub name: String,
    /// Rhai/script type (e.g., "String", "Number", "Bool", "Array", "Map")
    pub field_type: String,
    /// Rust type (e.g., "f32", "Vec<String>", "Option<Uuid>")
    pub rust_type: String,
    /// Whether this field is required
    pub required: bool,
    /// Optional description/documentation
    pub description: Option<String>,
}

/// Action script parameter schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSchemaDto {
    /// Description of the action
    pub description: String,
    /// Parameter definitions
    pub params: std::collections::HashMap<String, ParamSchemaDto>,
}

/// Parameter schema for action scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSchemaDto {
    /// Parameter type (e.g., "uuid", "string", "int", "float", "bool", "array")
    pub param_type: String,
    /// Whether this parameter is required
    pub required: bool,
}

// =============================================================================
// Validation DTOs
// =============================================================================

/// A single validation issue (error, warning, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssueDto {
    /// Severity level
    pub severity: ValidationSeverity,
    /// Human-readable message
    pub message: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// End line (for multi-line issues)
    pub end_line: Option<usize>,
    /// End column
    pub end_column: Option<usize>,
    /// Error code for documentation lookup
    pub code: Option<String>,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Validation severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

// =============================================================================
// State Introspection DTOs
// =============================================================================

/// A single state change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeDto {
    /// Entity that changed
    pub entity_id: Uuid,
    /// Type of change
    pub change_type: StateChangeType,
    /// Field path that changed (e.g., "hull_integrity", "cargo.items[0]")
    pub field_path: Option<String>,
    /// Previous value (if applicable)
    pub old_value: Option<serde_json::Value>,
    /// New value (if applicable)
    pub new_value: Option<serde_json::Value>,
}

/// Types of state changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChangeType {
    Created,
    Updated,
    Deleted,
}

/// A watch expression definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchDto {
    /// Unique watch ID
    pub id: Uuid,
    /// Expression to evaluate (e.g., "ships[uuid].hull")
    pub expression: String,
    /// User-friendly name
    pub name: Option<String>,
    /// Last evaluated value
    pub last_value: Option<serde_json::Value>,
    /// Number of history entries stored
    pub history_length: usize,
}

/// A single watch value update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchValueDto {
    /// Watch ID
    pub watch_id: Uuid,
    /// Tick when evaluated
    pub tick: u64,
    /// Evaluated value
    pub value: serde_json::Value,
    /// Error message if evaluation failed
    pub error: Option<String>,
}

// =============================================================================
// Export DTOs
// =============================================================================

/// Configuration for creating an export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfigDto {
    /// Sectors to include (empty = all)
    #[serde(default)]
    pub sectors: Vec<Uuid>,
    /// Include player entities
    #[serde(default = "default_true")]
    pub include_players: bool,
    /// Include NPC ships
    #[serde(default = "default_true")]
    pub include_npcs: bool,
    /// Include missions
    #[serde(default = "default_true")]
    pub include_missions: bool,
    /// Include script files
    #[serde(default = "default_true")]
    pub include_scripts: bool,
    /// Include archetype definitions
    #[serde(default = "default_true")]
    pub include_definitions: bool,
    /// Include SQLite state snapshot
    #[serde(default)]
    pub include_sqlite: bool,
    /// Export format
    #[serde(default)]
    pub format: ExportFormat,
}

impl Default for ExportConfigDto {
    fn default() -> Self {
        Self {
            sectors: vec![],
            include_players: true,
            include_npcs: true,
            include_missions: true,
            include_scripts: true,
            include_definitions: true,
            include_sqlite: false,
            format: ExportFormat::Cbor,
        }
    }
}

/// Export file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExportFormat {
    /// CBOR serialized entities
    #[default]
    Cbor,
    /// SQLite database snapshot
    Sqlite,
    /// Both CBOR and SQLite
    Both,
}

/// Summary of an export for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSummaryDto {
    /// Export ID
    pub id: Uuid,
    /// User-provided name
    pub name: String,
    /// Creation timestamp (Unix epoch ms)
    pub created_at: u64,
    /// File size in bytes
    pub size_bytes: u64,
    /// Export format used
    pub format: ExportFormat,
    /// Current status
    pub status: ExportStatus,
    /// Source (Live or Playtest ID)
    pub source: ExportSourceDto,
}

/// Export status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportStatus {
    InProgress,
    Completed,
    Failed,
}

/// Source of the export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportSourceDto {
    /// Exported from live server
    Live,
    /// Exported from a playtest
    Playtest(Uuid),
}

/// Export progress update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgressDto {
    /// Export ID
    pub export_id: Uuid,
    /// Current phase description
    pub phase: String,
    /// Progress percentage (0-100)
    pub percent: u8,
}
