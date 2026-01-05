//! State access layer for scripts
//!
//! Provides safe, controlled access to game state from Rhai scripts.
//! Scripts can query state (read-only snapshots) and queue mutations
//! that are applied atomically after script execution completes.

mod snapshots;
mod mutations;
mod accessor;

pub use snapshots::*;
pub use mutations::*;
pub use accessor::*;
