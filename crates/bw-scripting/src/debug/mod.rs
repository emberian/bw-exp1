//! Script Debugging System
//!
//! Provides debugging infrastructure for Rhai scripts including:
//! - Breakpoints (line, function, conditional)
//! - Step-through execution (into, over, out)
//! - Variable/scope inspection
//! - Call stack visualization
//!
//! Integrates with Rhai's built-in debugger API and the GM Editor.
//!
//! # Example
//!
//! ```rust,ignore
//! use bw_scripting::debug::{DebugController, DebugTarget};
//!
//! let controller = DebugController::new();
//!
//! // Start a debug session targeting live server
//! let (session_id, pause_rx) = controller.start_session(DebugTarget::Live);
//!
//! // Set a breakpoint
//! controller.set_breakpoint(session_id, Breakpoint::at_line("ai/pirate.rhai", 42));
//!
//! // When breakpoint hits, pause_rx receives PausedState
//! // Send commands to control execution
//! controller.send_command(session_id, DebugCommand::Continue);
//! ```

mod types;
mod controller;
mod rhai_integration;

pub use types::*;
pub use controller::DebugController;
pub use rhai_integration::register_debugger;
