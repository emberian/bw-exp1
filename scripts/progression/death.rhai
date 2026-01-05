// Death Script
//
// Handles player death consequences based on game mode.
// Called when a player's ship is destroyed.

// Handle player death event
//
// Arguments:
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//   - killer_id: String (killer's ship UUID, or () for environmental)
//   - death_context: Map { sector_id, position, cause }
//
// Returns: Map { respawned: bool, penalties: Map, message: String }
fn on_player_death(player_id, ship_id, killer_id, death_context) {
    // Get player data
    let player = query_player(player_id);
    if player == () {
        return #{
            respawned: false,
            error: "Player not found"
        };
    }

    // Get game mode
    let game_mode = player["game_mode"];
    if game_mode == () || game_mode == "" {
        game_mode = "standard";
    }

    // Load penalties from data file
    let penalties = get_data("death_penalties", game_mode);
    if penalties == () {
        penalties = get_default_penalties(game_mode);
    }

    // Log the death
    log_death(player_id, ship_id, killer_id, death_context);

    // Apply penalties based on game mode
    if game_mode == "standard" {
        return handle_standard_death(player_id, ship_id, player, penalties, death_context);
    } else if game_mode == "hardcore" {
        return handle_hardcore_death(player_id, ship_id, player, penalties, death_context);
    } else if game_mode == "ironman" {
        return handle_ironman_death(player_id, ship_id, player, penalties, death_context);
    }

    // Unknown mode - use standard
    handle_standard_death(player_id, ship_id, player, penalties, death_context)
}

// Handle death in standard mode (soft death - ship recovered)
fn handle_standard_death(player_id, ship_id, player, penalties, context) {
    let applied_penalties = #{};

    // Apply reputation loss
    let rep_loss = penalties["reputation_loss"];
    if rep_loss == () { rep_loss = 50; }
    if rep_loss > 0 {
        modify_player(player_id, #{ reputation_delta: -rep_loss });
        applied_penalties["reputation_lost"] = rep_loss;
    }

    // Apply fame loss (percentage)
    let fame_loss_pct = penalties["fame_loss_percent"];
    if fame_loss_pct == () { fame_loss_pct = 50; }
    let current_fame = player["fame"];
    if current_fame == () { current_fame = 0; }

    let fame_loss = (current_fame * fame_loss_pct) / 100;
    if fame_loss > 0 {
        modify_player(player_id, #{ fame_delta: -fame_loss });
        applied_penalties["fame_lost"] = fame_loss;
    }

    // Apply crew experience loss
    let ship = query_ship(ship_id);
    if ship != () {
        let xp_loss_pct = penalties["crew_xp_loss_percent"];
        if xp_loss_pct == () { xp_loss_pct = 50; }

        let current_xp = ship["experience"];
        if current_xp == () { current_xp = 0; }

        let xp_loss = (current_xp * xp_loss_pct) / 100;
        let new_xp = current_xp - xp_loss;
        if new_xp < 0 { new_xp = 0; }

        modify_ship(ship_id, #{ experience: new_xp });
        applied_penalties["experience_lost"] = xp_loss;
    }

    // Find respawn station
    let respawn_at = penalties["respawn_at"];
    if respawn_at == () { respawn_at = "nearest_friendly_station"; }

    let respawn_station = find_respawn_station(player_id, context["sector_id"], respawn_at);

    // Respawn ship with partial hull
    let respawn_hull = penalties["respawn_hull_percent"];
    if respawn_hull == () { respawn_hull = 50; }

    modify_ship(ship_id, #{
        hull: respawn_hull,
        shields: 0.0,
        status: "docked"
    });

    // Send death notification
    let station_name = "unknown station";
    if respawn_station != () {
        station_name = respawn_station["name"];
        if station_name == () { station_name = "nearby station"; }
    }

    send_error(player_id, "Your ship was destroyed! You wake in the medical bay of " + station_name);

    // Send penalties summary
    let penalty_msg = "Penalties: -" + rep_loss + " reputation";
    if fame_loss > 0 {
        penalty_msg += ", -" + fame_loss + " fame";
    }
    send_notification(player_id, penalty_msg);

    #{
        respawned: true,
        ship_lost: false,
        respawn_station: respawn_station,
        penalties: applied_penalties,
        message: "Ship recovered. " + penalty_msg
    }
}

// Handle death in hardcore mode (ship lost, must use backup or buy new)
fn handle_hardcore_death(player_id, ship_id, player, penalties, context) {
    let applied_penalties = #{};

    // Full reputation loss
    let rep_loss = penalties["reputation_loss"];
    if rep_loss == () { rep_loss = 100; }
    if rep_loss > 0 {
        modify_player(player_id, #{ reputation_delta: -rep_loss });
        applied_penalties["reputation_lost"] = rep_loss;
    }

    // Full fame loss
    let fame_loss_pct = penalties["fame_loss_percent"];
    if fame_loss_pct == () { fame_loss_pct = 100; }
    let current_fame = player["fame"];
    if current_fame == () { current_fame = 0; }

    let fame_loss = (current_fame * fame_loss_pct) / 100;
    if fame_loss > 0 {
        modify_player(player_id, #{ fame_delta: -fame_loss });
        applied_penalties["fame_lost"] = fame_loss;
    }

    // Destroy the ship
    destroy_ship(ship_id);
    applied_penalties["ship_destroyed"] = true;

    // Check for backup ships
    let owned_ships = player["owned_ships"];
    if owned_ships == () { owned_ships = []; }

    if owned_ships.len() > 0 {
        // Activate backup ship
        let backup_ship_id = owned_ships[0];
        // TODO: set_active_ship API needed

        send_error(player_id, "Your ship was destroyed! Activating backup vessel.");
        send_notification(player_id, "Lost: " + rep_loss + " reputation, " + fame_loss + " fame");

        return #{
            respawned: true,
            ship_lost: true,
            backup_activated: true,
            backup_ship_id: backup_ship_id,
            penalties: applied_penalties,
            message: "Ship destroyed. Backup vessel activated."
        };
    }

    // No backup - must purchase new ship
    let respawn_station = find_shipyard(player_id, context["sector_id"]);

    send_error(player_id, "Your ship was destroyed! You have no backup vessels.");
    send_warning(player_id, "Visit a shipyard to purchase a new ship.");

    #{
        respawned: false,
        ship_lost: true,
        must_purchase_ship: true,
        respawn_station: respawn_station,
        penalties: applied_penalties,
        message: "Ship destroyed. No backup available. Purchase required."
    }
}

// Handle death in ironman mode (permadeath - character deleted)
fn handle_ironman_death(player_id, ship_id, player, penalties, context) {
    // Log final stats
    let final_stats = #{
        reputation: player["reputation"],
        fame: player["fame"],
        credits: get_credits(player_id),
        game_mode: "ironman"
    };

    // Destroy the ship
    destroy_ship(ship_id);

    // Send final message
    send_error(player_id, "PERMADEATH: Your journey has ended.");
    send_notification(player_id, "Final stats - Fame: " + final_stats["fame"] + ", Reputation: " + final_stats["reputation"]);

    // TODO: Mark player as dead/inactive in database
    // TODO: Show leaderboard position

    #{
        respawned: false,
        permadeath: true,
        ship_lost: true,
        final_stats: final_stats,
        message: "Permadeath. Character journey ended."
    }
}

// Find appropriate respawn station
fn find_respawn_station(player_id, sector_id, respawn_type) {
    if respawn_type == "nearest_friendly_station" {
        // Try to find a friendly station in current sector
        let sector = query_sector(sector_id);
        if sector != () {
            let locations = sector["locations"];
            if locations != () {
                for loc in locations {
                    let loc_type = loc["location_type"];
                    if loc_type == "station" || loc_type == "outpost" {
                        return loc;
                    }
                }
            }
        }
    }

    // Default: return any station we can find
    ()
}

// Find a shipyard for hardcore respawn
fn find_shipyard(player_id, sector_id) {
    let sector = query_sector(sector_id);
    if sector != () {
        let locations = sector["locations"];
        if locations != () {
            for loc in locations {
                let services = loc["services"];
                if services != () {
                    for service in services {
                        if service == "shipyard" {
                            return loc;
                        }
                    }
                }
            }
        }
    }
    ()
}

// Get default penalties if data file not found
fn get_default_penalties(game_mode) {
    if game_mode == "standard" {
        return #{
            reputation_loss: 50,
            fame_loss_percent: 50,
            crew_xp_loss_percent: 50,
            respawn_hull_percent: 50,
            respawn_at: "nearest_friendly_station"
        };
    } else if game_mode == "hardcore" {
        return #{
            reputation_loss: 100,
            fame_loss_percent: 100,
            crew_lost: true,
            ship_lost: true,
            respawn_at: "shipyard",
            must_purchase_ship: true
        };
    } else if game_mode == "ironman" {
        return #{
            permadeath: true
        };
    }

    // Unknown mode - use standard
    #{
        reputation_loss: 50,
        fame_loss_percent: 50,
        respawn_hull_percent: 50
    }
}

// Log death for analytics/history
fn log_death(player_id, ship_id, killer_id, context) {
    let log_data = #{
        player_id: player_id,
        ship_id: ship_id,
        killer_id: killer_id,
        sector_id: context["sector_id"],
        cause: context["cause"],
        timestamp: 0  // TODO: get current tick
    };

    // Emit event for tracking
    emit_event("player_death", log_data);

    log_info("Player death: " + player_id + " killed by " + if killer_id != () { killer_id } else { "environment" });
}

// Calculate insurance payout (future feature)
fn calculate_insurance(player_id, ship_id) {
    let ship = query_ship(ship_id);
    if ship == () { return 0; }

    let ship_class = ship["ship_class"];
    let ship_def = get_ship_def(ship_class);
    if ship_def == () { return 0; }

    // 25% of ship value
    let ship_value = ship_def["purchase_credits"];
    if ship_value == () { return 0; }

    ship_value / 4
}

// Send death screen data to client
fn prepare_death_screen(player_id, result) {
    let screen_data = #{
        title: if result["permadeath"] { "JOURNEY'S END" } else { "SHIP DESTROYED" },
        message: result["message"],
        penalties: result["penalties"],
        respawned: result["respawned"],
        actions: []
    };

    if result["respawned"] {
        screen_data["actions"].push(#{
            id: "continue",
            text: "Continue",
            enabled: true
        });
    } else if result["must_purchase_ship"] {
        screen_data["actions"].push(#{
            id: "go_shipyard",
            text: "Go to Shipyard",
            enabled: true
        });
    } else if result["permadeath"] {
        screen_data["actions"].push(#{
            id: "new_character",
            text: "Create New Character",
            enabled: true
        });
        screen_data["actions"].push(#{
            id: "view_leaderboard",
            text: "View Leaderboard",
            enabled: true
        });
    }

    screen_data
}
