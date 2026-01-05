//! Network communication with game server

pub mod message_handler;
mod ws_client;

pub use ws_client::{NetworkClient, NetworkEvent};
