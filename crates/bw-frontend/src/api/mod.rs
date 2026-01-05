//! API client
//!
//! WebSocket and HTTP communication with the game server.

pub mod ws;

pub use ws::{ConnectionState, WsService};
