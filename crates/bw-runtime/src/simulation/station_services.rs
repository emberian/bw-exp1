//! Station Services System
//!
//! Handles docking, undocking, and station service usage.

use uuid::Uuid;

use bw_core::models::{LocationType, Ship, ShipStatus, StationService};
use bw_game::systems::spend_reputation;
use bw_shared::ServerMessage;

use crate::GameState;

/// Result of a station service action.
#[derive(Debug)]
pub struct ServiceResult {
    pub success: bool,
    pub message: String,
    pub cost_reputation: i32,
    pub cost_credits: i32,
    pub resource_update: Option<ServerMessage>,
}

/// Attempt to dock at a station.
pub fn dock_at_station(
    state: &GameState,
    player_id: Uuid,
    station_id: Uuid,
) -> Result<String, String> {
    // Get player session
    let session = state
        .players
        .get(&player_id)
        .ok_or("Player not found")?;
    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session);

    // Get ship
    let mut ship = state
        .ships
        .get_mut(&ship_id)
        .ok_or("Ship not found")?;

    // Check if ship can dock
    if !ship.can_move() {
        return Err("Ship cannot dock in current state".to_string());
    }

    if matches!(ship.status, ShipStatus::InCombat { .. }) {
        return Err("Cannot dock while in combat".to_string());
    }

    // Get sector
    let sector = state
        .sectors
        .get(&sector_id)
        .ok_or("Sector not found")?;

    // Find station
    let station = sector
        .sector
        .locations
        .iter()
        .find(|loc| loc.id == station_id)
        .ok_or("Station not found")?;

    // Check if station is dockable
    if !matches!(
        station.location_type,
        LocationType::CivilianStation
            | LocationType::NavalStation
            | LocationType::FreePort
    ) {
        return Err("Cannot dock at this location".to_string());
    }

    // Check distance
    let distance = ship.position.distance_to(&station.position);
    if distance > 50.0 {
        return Err(format!(
            "Too far from station (distance: {:.0}, need < 50)",
            distance
        ));
    }

    // Dock (persistence is automatic via dirty tracking)
    ship.status = ShipStatus::Docked {
        station_id,
    };
    ship.position = station.position;

    let station_name = station.name.clone();
    Ok(format!("Docked at {}", station_name))
}

/// Undock from current station.
pub fn undock(state: &GameState, player_id: Uuid) -> Result<String, String> {
    // Get player session
    let session = state
        .players
        .get(&player_id)
        .ok_or("Player not found")?;
    let ship_id = session.ship_id;
    drop(session);

    // Get ship
    let mut ship = state
        .ships
        .get_mut(&ship_id)
        .ok_or("Ship not found")?;

    // Check if docked
    if !matches!(ship.status, ShipStatus::Docked { .. }) {
        return Err("Not currently docked".to_string());
    }

    // Undock - move slightly away from station (persistence is automatic)
    ship.status = ShipStatus::Idle;
    ship.position.x += 10.0;
    ship.position.y += 10.0;

    Ok("Undocked successfully".to_string())
}

/// Use a station service.
pub fn use_service(
    state: &GameState,
    player_id: Uuid,
    service: StationService,
) -> ServiceResult {
    // Get player session
    let session = match state.players.get(&player_id) {
        Some(s) => s,
        None => {
            return ServiceResult {
                success: false,
                message: "Player not found".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    };
    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session);

    // Get ship
    let mut ship = match state.ships.get_mut(&ship_id) {
        Some(s) => s,
        None => {
            return ServiceResult {
                success: false,
                message: "Ship not found".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    };

    // Check if docked
    let station_id = match ship.status {
        ShipStatus::Docked { station_id } => station_id,
        _ => {
            return ServiceResult {
                success: false,
                message: "Must be docked to use services".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    };

    // Get sector to verify station has service
    let sector = match state.sectors.get(&sector_id) {
        Some(s) => s,
        None => {
            return ServiceResult {
                success: false,
                message: "Sector not found".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    };

    let station = match sector
        .sector
        .locations
        .iter()
        .find(|loc| loc.id == station_id)
    {
        Some(s) => s,
        None => {
            return ServiceResult {
                success: false,
                message: "Station not found".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    };

    // Check if station has service
    if !station.services.contains(&service) {
        return ServiceResult {
            success: false,
            message: format!("Station does not offer {:?} service", service),
            cost_reputation: 0,
            cost_credits: 0,
            resource_update: None,
        };
    }

    // Get player reference for reputation checks
    let mut player = match state.player_data.get_mut(&player_id) {
        Some(p) => p,
        None => {
            return ServiceResult {
                success: false,
                message: "Player not found".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    };

    // Process service
    match service {
        StationService::Refuel => {
            let fuel_needed = 100.0 - ship.resources.fuel;
            if fuel_needed < 1.0 {
                return ServiceResult {
                    success: false,
                    message: "Fuel tanks already full".to_string(),
                    cost_reputation: 0,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            // Resupply at non-naval stations costs reputation
            let rep_cost = if !matches!(station.location_type, LocationType::NavalStation) {
                5 // 5 reputation to resupply at civilian stations
            } else {
                0 // Free at naval stations
            };

            if rep_cost > 0 && !spend_reputation(&mut player.resources, rep_cost) {
                return ServiceResult {
                    success: false,
                    message: format!("Insufficient reputation (need {} rep)", rep_cost),
                    cost_reputation: rep_cost,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            ship.resources.fuel = 100.0;
            // Persistence is automatic via TrackedDashMap dirty tracking

            ServiceResult {
                success: true,
                message: format!("Refueled {:.1}% fuel", fuel_needed),
                cost_reputation: rep_cost,
                cost_credits: (fuel_needed * 10.0) as i32, // 10 credits per %
                resource_update: Some(build_resource_update(&ship, &player)),
            }
        }

        StationService::Rearm => {
            let ammo_needed = 100.0 - ship.resources.ammunition;
            if ammo_needed < 1.0 {
                return ServiceResult {
                    success: false,
                    message: "Ammunition stores already full".to_string(),
                    cost_reputation: 0,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            // Resupply at non-naval stations costs reputation
            let rep_cost = if !matches!(station.location_type, LocationType::NavalStation) {
                5 // 5 reputation to resupply at civilian stations
            } else {
                0 // Free at naval stations
            };

            if rep_cost > 0 && !spend_reputation(&mut player.resources, rep_cost) {
                return ServiceResult {
                    success: false,
                    message: format!("Insufficient reputation (need {} rep)", rep_cost),
                    cost_reputation: rep_cost,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            ship.resources.ammunition = 100.0;
            // Persistence is automatic via TrackedDashMap dirty tracking

            ServiceResult {
                success: true,
                message: format!("Rearmed {:.1}% ammunition", ammo_needed),
                cost_reputation: rep_cost,
                cost_credits: (ammo_needed * 15.0) as i32, // 15 credits per %
                resource_update: Some(build_resource_update(&ship, &player)),
            }
        }

        StationService::Repair => {
            let hull_needed = 100.0 - ship.hull_integrity;
            let shield_needed = ship.ship_class.base_stats().shield_capacity - ship.shield_strength;

            if hull_needed < 1.0 && shield_needed < 1.0 {
                return ServiceResult {
                    success: false,
                    message: "Ship already at full integrity".to_string(),
                    cost_reputation: 0,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            ship.hull_integrity = 100.0;
            ship.shield_strength = ship.ship_class.base_stats().shield_capacity;

            // If was disabled, restore to idle
            if matches!(ship.status, ShipStatus::Disabled) {
                ship.status = ShipStatus::Docked { station_id };
            }
            // Persistence is automatic via TrackedDashMap dirty tracking

            ServiceResult {
                success: true,
                message: format!(
                    "Repaired {:.1}% hull and {:.1}% shields",
                    hull_needed, shield_needed
                ),
                cost_reputation: 0,
                cost_credits: (hull_needed * 50.0 + shield_needed * 20.0) as i32,
                resource_update: None,
            }
        }

        StationService::ShoreLeave => {
            let morale_boost = 100.0 - ship.crew.morale;
            if ship.crew.morale >= 90.0 {
                return ServiceResult {
                    success: false,
                    message: "Crew morale already high".to_string(),
                    cost_reputation: 0,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            // Shore leave always costs reputation (requesting leave from admiralty)
            let rep_cost = 10;

            if !spend_reputation(&mut player.resources, rep_cost) {
                return ServiceResult {
                    success: false,
                    message: format!("Insufficient reputation for shore leave (need {} rep)", rep_cost),
                    cost_reputation: rep_cost,
                    cost_credits: 0,
                    resource_update: None,
                };
            }

            ship.crew.morale = 100.0;
            // Persistence is automatic via TrackedDashMap dirty tracking

            ServiceResult {
                success: true,
                message: format!("Shore leave boosted crew morale by {:.1}%", morale_boost),
                cost_reputation: rep_cost,
                cost_credits: (morale_boost * 5.0) as i32,
                resource_update: Some(build_resource_update(&ship, &player)),
            }
        }

        StationService::Trade => {
            // Trading is handled separately via economy system
            ServiceResult {
                success: true,
                message: "Trading interface opened".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }

        StationService::ShipUpgrade => {
            // Ship upgrades handled separately
            ServiceResult {
                success: true,
                message: "Upgrade interface opened".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }

        StationService::MissionBoard => {
            // Mission board handled separately
            ServiceResult {
                success: true,
                message: "Mission board opened".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }

        StationService::BlackMarket => {
            // Black market handled separately
            ServiceResult {
                success: true,
                message: "Black market accessed. Trade carefully.".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }

        StationService::ShipPurchase => {
            // Ship purchase handled separately
            ServiceResult {
                success: true,
                message: "Ship dealer opened.".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }

        StationService::JumpgateAccess => {
            // Jumpgate handled by movement system
            ServiceResult {
                success: true,
                message: "Jumpgate access granted.".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }

        StationService::Information => {
            // Information access
            ServiceResult {
                success: true,
                message: "Archives accessed.".to_string(),
                cost_reputation: 0,
                cost_credits: 0,
                resource_update: None,
            }
        }
    }
}

/// Build a resource update message from ship and player state.
fn build_resource_update(ship: &Ship, player: &bw_core::models::Player) -> ServerMessage {
    ServerMessage::ResourceUpdate {
        reputation: player.resources.reputation,
        fame: player.resources.fame,
        ammunition: ship.resources.ammunition,
        fuel: ship.resources.fuel,
        morale: ship.crew.morale,
        experience: ship.crew.experience,
    }
}
