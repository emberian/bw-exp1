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

pub use engine::*;
pub use mission_runner::*;
pub use mission_generator::*;
