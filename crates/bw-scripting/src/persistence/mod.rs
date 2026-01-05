//! Script Persistence System
//!
//! Provides persistent storage for script state, enabling features like:
//! - Nemesis system (remembering player-enemy history)
//! - Quest progress across sessions
//! - Dynamic world state
//! - Checkpoints and save states
//!
//! # Example
//!
//! ```rhai
//! // Save some state
//! save_state("nemesis:player123", #{ name: "Dread Pirate", defeats: 3 });
//!
//! // Load it later
//! let history = load_state("nemesis:player123");
//! if history != () {
//!     print(`${history.name} has been defeated ${history.defeats} times`);
//! }
//!
//! // Create a checkpoint
//! checkpoint("before_boss_fight");
//! ```

mod store;
mod bindings;

pub use store::{ScriptStateStore, InMemoryStore, FileStore, ScriptState, PersistenceError};
pub use bindings::register as register_persistence_bindings;
