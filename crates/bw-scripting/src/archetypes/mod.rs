//! Entity Archetypes - Rhai-defined game data
//!
//! Archetypes define the base stats and properties of game entities.
//! Unlike hardcoded Rust enums, archetypes can be:
//! - Hot-reloaded at runtime
//! - Extended by admins without Rust changes
//! - Composed using Rhai imports and splatting
//!
//! ```rhai
//! // scripts/definitions/ships.rhai
//! fn patrol_corvette() {
//!     #{
//!         id: "patrol_corvette",
//!         name: "Patrol Corvette",
//!         stats: #{ attack: 30.0, defense: 20.0, speed: 80.0, ... },
//!         weapons: [...],
//!         is_player_class: true,
//!     }
//! }
//! ```

mod registry;
mod ship_archetype;
mod weapon_archetype;

pub use registry::*;
pub use ship_archetype::*;
pub use weapon_archetype::*;
