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

pub use engine::*;
pub use mission_runner::*;
pub use mission_generator::*;
pub use state::*;
pub use coroutines::*;
pub use events::*;
pub use behaviors::*;
pub use validation::*;
pub use views::*;
