//! Movement system
//!
//! Handles ship movement within and between sectors.

use crate::models::{Position, Sector, Ship, ShipStatus};

/// Calculate fuel cost for in-sector movement.
pub fn calculate_in_sector_fuel_cost(ship: &Ship, distance: f64, sector: &Sector) -> f32 {
    let base_cost = sector.in_sector_fuel_cost(distance);
    let class_stats = ship.ship_class.base_stats();
    base_cost * (class_stats.fuel_per_sector / 5.0) // Normalize to corvette
}

/// Calculate fuel cost for inter-sector movement.
pub fn calculate_inter_sector_fuel_cost(ship: &Ship, from: &Sector, to: &Sector) -> f32 {
    let base_cost = from.inter_sector_fuel_cost();
    let to_modifier = to.fuel_cost_modifier;
    let class_stats = ship.ship_class.base_stats();

    base_cost * (class_stats.fuel_per_sector / 5.0) * ((1.0 + to_modifier) / 2.0)
}

/// Move a ship towards a destination.
pub fn move_ship_towards(ship: &mut Ship, destination: Position, speed: f64, delta_time: f64) -> bool {
    let distance_to_move = speed * delta_time;
    let current_distance = ship.position.distance_to(&destination);

    if current_distance <= distance_to_move {
        ship.position = destination;
        true // Arrived
    } else {
        ship.position.move_towards(&destination, distance_to_move);
        false // Still moving
    }
}

/// Check if a ship can move to a sector.
pub fn can_move_to_sector(ship: &Ship, current_sector: &Sector, target_sector_id: uuid::Uuid) -> bool {
    // Must have the sectors be adjacent
    if !current_sector.adjacent_sectors.contains(&target_sector_id) {
        return false;
    }

    // Must be able to move
    ship.can_move()
}

/// Update ship movement state.
pub fn update_ship_movement(ship: &mut Ship, destination: Position, target_id: Option<uuid::Uuid>) {
    ship.status = ShipStatus::InTransit {
        destination,
        target_id,
    };
}

/// Clear ship movement state.
pub fn clear_ship_movement(ship: &mut Ship) {
    ship.status = ShipStatus::Idle;
}
