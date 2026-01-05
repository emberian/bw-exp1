//! Core data models for Blackwing
//!
//! All game entities are defined here with full serialization support.

mod ship;
mod player;
mod sector;
mod mission;
mod faction;
mod squadron;
mod resources;

pub use ship::*;
pub use player::*;
pub use sector::*;
pub use mission::*;
pub use faction::*;
pub use squadron::*;
pub use resources::*;
