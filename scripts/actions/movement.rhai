// Movement action handlers
//
// Handles ship movement commands: position, location, sector, and stop.

fn init() {
    register_action_with_requirements("move_to_position", "handle_move_to_position", [
        "authenticated",
        #{ ship_status: ["idle", "moving"] }
    ]);

    register_action_with_requirements("move_to_location", "handle_move_to_location", [
        "authenticated",
        #{ ship_status: ["idle", "moving"] }
    ]);

    register_action_with_requirements("move_to_sector", "handle_move_to_sector", [
        "authenticated",
        #{ ship_status: ["idle"] }
    ]);

    register_action_with_requirements("stop_movement", "handle_stop_movement", [
        "authenticated"
    ]);

    log_info("Movement actions registered");
}

// Handle move to absolute position
// params: { x: f64, y: f64, z: f64 }
fn handle_move_to_position(ctx, params) {
    let x = params["x"];
    let y = params["y"];
    let z = params["z"];

    if x == () || y == () {
        return #{ success: false, error: "x and y coordinates required" };
    }

    // Default z to 0 if not provided
    if z == () {
        z = 0.0;
    }

    let ship_id = ctx["ship_id"];
    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Check ship is not docked
    if ship["status"] == "docked" {
        return #{ success: false, error: "Cannot move while docked" };
    }

    // Check ship is not in combat
    if ship["status"] == "in_combat" {
        return #{ success: false, error: "Cannot move while in combat" };
    }

    // Set ship to InTransit status with destination
    // Use the dedicated move_ship_to function which properly sets InTransit status
    if !move_ship_to(ship_id, x, y, z) {
        return #{ success: false, error: "Failed to start movement" };
    }

    #{
        success: true,
        data: #{ destination: #{ x: x, y: y, z: z } }
    }
}

// Handle move to a named location
// params: { location_id: uuid }
fn handle_move_to_location(ctx, params) {
    let location_id = params["location_id"];
    if location_id == () {
        return #{ success: false, error: "location_id required" };
    }

    let ship_id = ctx["ship_id"];
    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Check ship status
    if ship["status"] == "docked" {
        return #{ success: false, error: "Cannot move while docked" };
    }
    if ship["status"] == "in_combat" {
        return #{ success: false, error: "Cannot move while in combat" };
    }

    // Query the location
    let location = query_location(location_id);
    if location == () {
        return #{ success: false, error: "Location not found" };
    }

    // Check location is in same sector
    let sector_id = ctx["sector_id"];
    if location["sector_id"] != () && location["sector_id"] != sector_id {
        return #{ success: false, error: "Location is in a different sector" };
    }

    let pos = location["position"];

    // Set ship to InTransit status using dedicated function
    if !move_ship_to(ship_id, pos["x"], pos["y"], pos["z"]) {
        return #{ success: false, error: "Failed to start movement" };
    }

    send_notification(ctx["player_id"], "Moving to " + location["name"]);

    #{
        success: true,
        data: #{
            location_name: location["name"],
            destination: pos
        }
    }
}

// Handle sector jump
// params: { sector_id: uuid }
fn handle_move_to_sector(ctx, params) {
    let target_sector_id = params["sector_id"];
    if target_sector_id == () {
        return #{ success: false, error: "sector_id required" };
    }

    let ship_id = ctx["ship_id"];
    let player_id = ctx["player_id"];
    let current_sector_id = ctx["sector_id"];

    // Can't jump to same sector
    if target_sector_id == current_sector_id {
        return #{ success: false, error: "Already in this sector" };
    }

    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Check ship status
    if ship["status"] == "docked" {
        return #{ success: false, error: "Undock before traveling to another sector" };
    }
    if ship["status"] == "in_combat" {
        return #{ success: false, error: "Cannot travel while in combat" };
    }
    if ship["status"] == "in_transit" {
        return #{ success: false, error: "Already traveling" };
    }

    // Query current sector for jumpgate
    let sector = query_sector(current_sector_id);
    if sector == () {
        return #{ success: false, error: "Current sector not found" };
    }

    // Check if there's a jumpgate in this sector
    let has_jumpgate = false;
    let jumpgate_pos = ();
    for loc in sector["locations"] {
        if loc["location_type"] == "Jumpgate" {
            has_jumpgate = true;
            jumpgate_pos = loc["position"];
            break;
        }
    }

    if !has_jumpgate {
        return #{ success: false, error: "No jumpgate in this sector" };
    }

    // Check if target sector is adjacent
    let adjacent = sector["adjacent_sectors"];
    let is_adjacent = false;
    for adj_id in adjacent {
        if adj_id == target_sector_id {
            is_adjacent = true;
            break;
        }
    }

    if !is_adjacent {
        return #{ success: false, error: "Target sector is not accessible from this jumpgate" };
    }

    // Check ship is near jumpgate (within 200 units)
    let ship_pos = ship["position"];
    let dist = distance_between(ship_pos, jumpgate_pos);
    if dist > 200.0 {
        return #{
            success: false,
            error: "Must be near jumpgate to travel (within 200 units, currently " + dist.to_int() + ")"
        };
    }

    // Query target sector for name
    let target_sector = query_sector(target_sector_id);
    let target_name = if target_sector != () { target_sector["name"] } else { "Unknown" };

    // Set ship to in_transit status
    modify_ship(ship_id, #{
        status: "in_transit",
        destination_sector: target_sector_id
    });

    send_notification(player_id, "Jumping to " + target_name + "...");

    emit_event("sector_jump_initiated", #{
        player_id: player_id,
        from_sector: current_sector_id,
        to_sector: target_sector_id
    });

    #{
        success: true,
        data: #{
            target_sector_id: target_sector_id,
            target_sector_name: target_name
        }
    }
}

// Handle stop movement
fn handle_stop_movement(ctx, _params) {
    let ship_id = ctx["ship_id"];
    let ship = query_ship(ship_id);

    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    let status = ship["status"];

    // Can only stop if moving (in_transit status)
    if status != "in_transit" {
        return #{ success: false, error: "Ship is not moving" };
    }

    // Use modify_ship with "idle" status to stop movement
    modify_ship(ship_id, #{
        status: "idle"
    });

    #{ success: true }
}
