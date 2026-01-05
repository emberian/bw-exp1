//! BLACKWING Core - Game data models
//!
//! This crate contains all core data structures and domain events
//! for the Blackwing space patrol game.
//!
//! Game systems (combat, movement, reputation) are in bw-game.

pub mod hash_helpers;
pub mod models;
pub mod events;

pub use models::*;
