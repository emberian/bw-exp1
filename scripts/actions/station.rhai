// Station-related action handlers
//
// These handlers demonstrate how to implement game actions in scripts
// using the new ScriptAction system.

// Called when the script is loaded to register action handlers
fn init() {
    // Register dock action with ship status requirement
    register_action_with_requirements("dock", "handle_dock", [
        "authenticated",
        #{ ship_status: ["idle"] }
    ]);

    // Register undock action
    register_action_with_requirements("undock", "handle_undock", [
        "authenticated",
        "must_be_docked"
    ]);

    // Register use_service action
    register_action_with_requirements("use_service", "handle_use_service", [
        "authenticated",
        "must_be_docked"
    ]);

    log_info("Station actions registered");
}

// Handle dock action
// ctx: { player_id, ship_id, sector_id, action }
// params: { station_id: string }
fn handle_dock(ctx, params) {
    let station_id = params["station_id"];
    if station_id == () {
        return #{ success: false, error: "station_id required" };
    }

    // Parse station_id (it comes as a string)
    let ship_id = ctx["ship_id"];

    // Query ship to get position
    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Query location (station)
    let location = query_location(station_id);
    if location == () {
        return #{ success: false, error: "Station not found" };
    }

    // Check if station is dockable
    let location_type = location["location_type"];
    if location_type != "Station" && location_type != "Outpost" && location_type != "Starbase" {
        return #{ success: false, error: "Cannot dock at this location type" };
    }

    // Check distance (must be within 100 units)
    let dist = distance_between(ship["position"], location["position"]);
    if dist > 100.0 {
        return #{
            success: false,
            error: "Too far from station (must be within 100 units, currently " + dist + ")"
        };
    }

    // Apply dock status via mutation
    modify_ship(ship_id, #{
        status: "docked",
        docked_at: station_id
    });

    // Send notification
    send_notification(ctx["player_id"], "Docked at " + location["name"]);

    // Emit event for other systems
    emit_event("player_docked", #{
        player_id: ctx["player_id"],
        station_id: station_id,
        station_name: location["name"]
    });

    #{
        success: true,
        data: #{
            station_name: location["name"],
            services: location["services"]
        }
    }
}

// Handle undock action
fn handle_undock(ctx, params) {
    let ship_id = ctx["ship_id"];
    let player_id = ctx["player_id"];

    // Modify ship status back to idle
    modify_ship(ship_id, #{
        status: "idle",
        docked_at: ()
    });

    send_notification(player_id, "Undocked successfully");

    emit_event("player_undocked", #{
        player_id: player_id
    });

    #{ success: true }
}

// Handle use_service action
// params: { service: string }
fn handle_use_service(ctx, params) {
    let service = params["service"];
    if service == () {
        return #{ success: false, error: "service required" };
    }

    let ship_id = ctx["ship_id"];
    let player_id = ctx["player_id"];

    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Get docked station from ship status
    let docked_at = ship["docked_at"];
    if docked_at == () {
        return #{ success: false, error: "Not docked at a station" };
    }

    let location = query_location(docked_at);
    if location == () {
        return #{ success: false, error: "Docked station not found" };
    }

    // Check if service is available
    let services = location["services"];
    let service_available = false;
    for s in services {
        if s.to_lower() == service.to_lower() {
            service_available = true;
            break;
        }
    }

    if !service_available {
        return #{ success: false, error: "Service '" + service + "' not available at this station" };
    }

    // Handle different services
    let result = switch service.to_lower() {
        "refuel" => handle_refuel(ship_id, player_id),
        "rearm" => handle_rearm(ship_id, player_id),
        "repair" => handle_repair(ship_id, player_id),
        "shoreleave" => handle_shore_leave(ship_id, player_id),
        _ => #{ success: false, error: "Unknown service: " + service }
    };

    result
}

// Service handlers
fn handle_refuel(ship_id, player_id) {
    let ship = query_ship(ship_id);
    let current_fuel = ship["fuel"];

    if current_fuel >= 100.0 {
        return #{ success: false, error: "Fuel tank already full" };
    }

    // Calculate cost (10 credits per fuel unit)
    let fuel_needed = 100.0 - current_fuel;
    let cost = (fuel_needed * 10.0).to_int();

    // Check player has credits
    let player = query_player(player_id);
    if player["credits"] < cost {
        return #{ success: false, error: "Insufficient credits (need " + cost + ")" };
    }

    // Apply changes
    modify_ship(ship_id, #{ fuel: 100.0 });
    spend_credits(player_id, cost);

    send_notification(player_id, "Refueled for " + cost + " credits");

    #{ success: true, data: #{ cost: cost, fuel: 100.0 } }
}

fn handle_rearm(ship_id, player_id) {
    let ship = query_ship(ship_id);
    let current_ammo = ship["ammunition"];

    if current_ammo >= 100.0 {
        return #{ success: false, error: "Ammunition already full" };
    }

    let ammo_needed = 100.0 - current_ammo;
    let cost = (ammo_needed * 5.0).to_int();

    let player = query_player(player_id);
    if player["credits"] < cost {
        return #{ success: false, error: "Insufficient credits (need " + cost + ")" };
    }

    modify_ship(ship_id, #{ ammunition: 100.0 });
    spend_credits(player_id, cost);

    send_notification(player_id, "Rearmed for " + cost + " credits");

    #{ success: true, data: #{ cost: cost, ammunition: 100.0 } }
}

fn handle_repair(ship_id, player_id) {
    let ship = query_ship(ship_id);
    let current_hull = ship["hull"];

    if current_hull >= 100.0 {
        return #{ success: false, error: "Hull already at full integrity" };
    }

    let repair_needed = 100.0 - current_hull;
    let cost = (repair_needed * 20.0).to_int();

    let player = query_player(player_id);
    if player["credits"] < cost {
        return #{ success: false, error: "Insufficient credits (need " + cost + ")" };
    }

    modify_ship(ship_id, #{ hull: 100.0 });
    spend_credits(player_id, cost);

    send_notification(player_id, "Repaired for " + cost + " credits");

    #{ success: true, data: #{ cost: cost, hull: 100.0 } }
}

fn handle_shore_leave(ship_id, player_id) {
    let ship = query_ship(ship_id);
    let current_morale = ship["morale"];

    if current_morale >= 100.0 {
        return #{ success: false, error: "Crew morale already excellent" };
    }

    let morale_boost = 100.0 - current_morale;
    let cost = (morale_boost * 2.0).to_int();

    let player = query_player(player_id);
    if player["credits"] < cost {
        return #{ success: false, error: "Insufficient credits (need " + cost + ")" };
    }

    modify_ship(ship_id, #{ morale: 100.0 });
    spend_credits(player_id, cost);

    send_notification(player_id, "Crew enjoyed shore leave (cost: " + cost + " credits)");

    #{ success: true, data: #{ cost: cost, morale: 100.0 } }
}
