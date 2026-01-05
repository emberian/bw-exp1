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
pub mod state;
pub mod coroutines;
pub mod events;
pub mod behaviors;
pub mod validation;
pub mod views;
pub mod transaction;
pub mod archetypes;
pub mod ai;
pub mod persistence;
pub mod errors;
pub mod context;

pub use engine::*;
pub use mission_runner::*;
pub use mission_generator::*;
pub use state::*;
pub use coroutines::*;
pub use events::*;
pub use behaviors::*;
pub use validation::*;
pub use views::{ShipView, PlayerView, SectorView, LocationView};
pub use transaction::TransactionContext;
pub use archetypes::{ArchetypeRegistry, ShipArchetype, WeaponArchetype, ShipStats, ShipWeapon};
pub use ai::{BtNode, BtStatus, BtNodeState, DecoratorKind, UtilityOption, BehaviorTreeRunner, register_ai_bindings};
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
};

// Re-export derive macros
pub use bw_scripting_macros::{RhaiSerialize, RhaiDeserialize};
