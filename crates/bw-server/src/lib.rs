//! BLACKWING Game Server
//!
//! Axum-based game server with:
//! - HTTP API for authentication and static data
//! - WebSocket for real-time game state
//! - Authoritative game loop running at 10 TPS

pub mod state;
pub mod routes;
pub mod ws;
pub mod simulation;
pub mod persistence;

pub use state::*;
