//! Facade objects for fluent script API
//!
//! **DEPRECATED**: These view types are deprecated in favor of direct snapshot access.
//! Use `query_ship()`, `query_player()`, etc. to get snapshots, and `modify_ship()`,
//! etc. for mutations. This provides a simpler, more explicit API.
//!
//! Old pattern (deprecated):
//! ```rhai
//! let ship = get_ship(ship_id);
//! let hull = ship.hull;           // Uses ShipView getter
//! ship.damage(10);                // Uses ShipView method
//! ```
//!
//! New pattern (preferred):
//! ```rhai
//! let ship = query_ship(ship_id);
//! let hull = ship["hull"];                    // Direct map access
//! damage_ship(ship_id, 10.0);                 // Standalone function
//! modify_ship(ship_id, #{ hull: hull - 10 }); // Explicit mutation
//! ```
//!
//! Views are thin wrappers around entity IDs that use the StateAccessor
//! to read snapshots and queue mutations. All mutations are safe - they
//! validate inputs and go through the permission system.

#[macro_use]
mod macros;

mod ship_view;
mod player_view;
mod sector_view;
mod location_view;

pub use ship_view::*;
pub use player_view::*;
pub use sector_view::*;
pub use location_view::*;

use rhai::Engine;

/// Register all view types with the Rhai engine.
pub fn register(engine: &mut Engine) {
    ship_view::register(engine);
    player_view::register(engine);
    sector_view::register(engine);
    location_view::register(engine);
}
