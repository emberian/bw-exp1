//! Script Action System
//!
//! Allows scripts to register handlers for client actions, enabling game logic
//! to be implemented in scripts rather than hardcoded Rust handlers.
//!
//! Actions are dispatched by name to the appropriate script handler.

mod registry;
mod dispatcher;

pub use registry::*;
pub use dispatcher::*;
