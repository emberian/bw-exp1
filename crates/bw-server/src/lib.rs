//! BLACKWING Game Server
//!
//! Axum-based game server with:
//! - HTTP API for authentication and static data
//! - WebSocket for real-time game state
//! - Authoritative game loop running at 10 TPS

pub mod auth;
pub mod config;
pub mod persistence;
pub mod routes;
pub mod scripting;
pub mod simulation;
pub mod state;
pub mod ws;

pub use config::{ServerConfig, ConfigManager, init_config, config, try_config};
pub use persistence::Database;
pub use state::*;
