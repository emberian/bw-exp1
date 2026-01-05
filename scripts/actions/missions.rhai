// Mission action handlers
//
// Handles mission accept, abandon, and choice selection.

fn init() {
    register_action_with_requirements("accept_mission", "handle_accept_mission", [
        "authenticated"
    ]);

    register_action_with_requirements("abandon_mission", "handle_abandon_mission", [
        "authenticated"
    ]);

    register_action_with_requirements("mission_choice", "handle_mission_choice", [
        "authenticated"
    ]);

    log_info("Mission actions registered");
}

// Accept a mission
// params: { mission_id: uuid }
fn handle_accept_mission(ctx, params) {
    let mission_id = params["mission_id"];
    if mission_id == () {
        return #{ success: false, error: "mission_id required" };
    }

    let player_id = ctx["player_id"];
    let sector_id = ctx["sector_id"];

    // Query the mission
    let mission = query_mission(mission_id);
    if mission == () {
        return #{ success: false, error: "Mission not found" };
    }

    // Check mission is available
    if mission["status"] != "available" {
        return #{ success: false, error: "Mission is not available" };
    }

    // Check player meets requirements
    let player = query_player(player_id);
    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    // Check reputation requirement
    let min_rep = mission["min_reputation"];
    if min_rep != () && player["reputation"] < min_rep {
        return #{
            success: false,
            error: "Insufficient reputation (need " + min_rep + ", have " + player["reputation"] + ")"
        };
    }

    // Check if player already has max active missions
    let active_missions = player["active_missions"];
    if active_missions != () && active_missions.len() >= 3 {
        return #{ success: false, error: "Maximum active missions reached (3)" };
    }

    // Accept the mission
    modify_mission(mission_id, #{
        status: "active",
        accepted_by: player_id,
        accepted_at: current_tick()
    });

    // Add to player's active missions
    add_player_mission(player_id, mission_id);

    send_notification(player_id, "Mission accepted: " + mission["title"]);

    emit_event("mission_accepted", #{
        player_id: player_id,
        mission_id: mission_id,
        mission_title: mission["title"]
    });

    #{
        success: true,
        data: #{
            mission_id: mission_id,
            title: mission["title"],
            description: mission["description"]
        }
    }
}

// Abandon an active mission
// params: { mission_id: uuid }
fn handle_abandon_mission(ctx, params) {
    let mission_id = params["mission_id"];
    if mission_id == () {
        return #{ success: false, error: "mission_id required" };
    }

    let player_id = ctx["player_id"];

    // Query the mission
    let mission = query_mission(mission_id);
    if mission == () {
        return #{ success: false, error: "Mission not found" };
    }

    // Check mission is active and owned by player
    if mission["status"] != "active" {
        return #{ success: false, error: "Mission is not active" };
    }

    if mission["accepted_by"] != player_id {
        return #{ success: false, error: "This is not your mission" };
    }

    // Calculate reputation penalty
    let rep_penalty = mission["reputation_reward"] / 2;
    if rep_penalty < 5 {
        rep_penalty = 5;
    }

    // Abandon the mission
    modify_mission(mission_id, #{
        status: "available",
        accepted_by: (),
        accepted_at: ()
    });

    // Remove from player's active missions
    remove_player_mission(player_id, mission_id);

    // Apply reputation penalty
    modify_player(player_id, #{
        reputation_delta: -rep_penalty
    });

    send_notification(player_id, "Mission abandoned. Lost " + rep_penalty + " reputation.");

    emit_event("mission_abandoned", #{
        player_id: player_id,
        mission_id: mission_id,
        reputation_lost: rep_penalty
    });

    #{
        success: true,
        data: #{
            reputation_lost: rep_penalty
        }
    }
}

// Make a choice in a mission dialog
// params: { mission_id: uuid, choice_id: string }
fn handle_mission_choice(ctx, params) {
    let mission_id = params["mission_id"];
    let choice_id = params["choice_id"];

    if mission_id == () {
        return #{ success: false, error: "mission_id required" };
    }
    if choice_id == () {
        return #{ success: false, error: "choice_id required" };
    }

    let player_id = ctx["player_id"];

    // Query the mission
    let mission = query_mission(mission_id);
    if mission == () {
        return #{ success: false, error: "Mission not found" };
    }

    // Verify player owns this mission
    if mission["accepted_by"] != player_id {
        return #{ success: false, error: "This is not your mission" };
    }

    // Get pending choices
    let pending_choice = mission["pending_choice"];
    if pending_choice == () {
        return #{ success: false, error: "No choice pending for this mission" };
    }

    // Validate choice_id exists in available choices
    let choices = pending_choice["choices"];
    let valid_choice = ();
    for choice in choices {
        if choice["id"] == choice_id {
            valid_choice = choice;
            break;
        }
    }

    if valid_choice == () {
        return #{ success: false, error: "Invalid choice: " + choice_id };
    }

    // Check if choice is available
    if valid_choice["is_available"] == false {
        return #{ success: false, error: "That choice is not available: " + valid_choice["requirement_text"] };
    }

    // Record the choice
    record_mission_choice(mission_id, choice_id);

    // Clear the pending choice
    modify_mission(mission_id, #{
        pending_choice: ()
    });

    emit_event("mission_choice_made", #{
        player_id: player_id,
        mission_id: mission_id,
        choice_id: choice_id
    });

    send_notification(player_id, "Choice recorded: " + valid_choice["text"]);

    #{
        success: true,
        data: #{
            choice_id: choice_id,
            choice_text: valid_choice["text"]
        }
    }
}
