//! BLACKWING Scripting Engine
//!
//! Rhai-based scripting for:
//! - Missions (state machines, choices, outcomes)
//! - Combat (damage formulas, accuracy)
//! - AI behaviors (NPC decision making)
//! - Economy (trade calculations)
//!
//! All scripts can be modded by users.

pub mod engine;
pub mod bindings;
pub mod loader;
pub mod mission_runner;
pub mod mission_generator;
pub mod coroutines;
pub mod events;
pub mod handlers;
pub mod actions;
pub mod effects;
pub mod behaviors;
pub mod validation;
pub mod views;
pub mod transaction;
pub mod persistence;
pub mod errors;
pub mod context;
pub mod debug;
pub mod schema;

// Core scripting exports
pub use engine::*;
pub use mission_runner::*;
pub use mission_generator::*;
pub use coroutines::*;
pub use events::*;
pub use handlers::{
    HandlerContext, HandlerFn, Handler, HandlerType, HandlerResult,
    HandlerRegistry, HandlerDispatcher,
};
pub use actions::*;
pub use effects::{
    EffectContext, EffectDispatcher, EffectRegistry,
    create_effect_registry, create_effect_dispatcher, apply_effect,
};
pub use behaviors::*;
pub use validation::*;
pub use views::{ShipView, PlayerView, SectorView, LocationView};
pub use transaction::TransactionContext;
pub use persistence::{
    ScriptStateStore, InMemoryStore, FileStore, ScriptState, PersistenceError,
    register_persistence_bindings,
};
pub use errors::{
    ScriptError, ScriptErrorContext,
    push_error, take_errors, last_error, has_errors, clear_errors,
    set_current_script, set_current_tick, get_current_script, get_current_tick
};
pub use context::{
    ScriptExecutionContext, ExecutionGuard, ExecutionError,
    with_context, with_context_mut, is_executing,
    current_script_path, current_owner_entity, current_sector, current_tick,
    with_effect_dispatcher,
};
pub use debug::{
    DebugController, DebugSession, DebugTarget, DebugCommand,
    Breakpoint, FunctionBreakpoint, PausedState, StackFrame, Variable,
    EntityContext, PauseReason, StepMode, EvaluationResult,
    register_debugger,
};

// Re-export derive macros
pub use bw_scripting_macros::{RhaiSerialize, RhaiDeserialize, RhaiSchema};

// Re-export schema types
pub use schema::{
    ArchetypeSchema, FieldSchema, RhaiSchema as RhaiSchemaTrait,
    ActionSchema, ActionSchemaRegistry, ParamSchema,
    DefFieldType, DefFieldSchema, DefinitionSchema, DefinitionSchemaRegistry,
    SHIP_SCHEMA, WEAPON_SCHEMA, EFFECT_SCHEMA, CARGO_SCHEMA, ABILITY_SCHEMA, FACTION_SCHEMA,
};

// Minimal re-exports for internal macros (used via $crate::state in macros.rs)
pub mod state {
    pub use bw_game::state::{ShipChanges, PlayerChanges};
}
