// Social action handlers
//
// Handles hailing, alliance responses, and other social interactions.

fn init() {
    register_action_with_requirements("hail", "handle_hail", [
        "authenticated"
    ]);

    register_action_with_requirements("accept_alliance", "handle_accept_alliance", [
        "authenticated"
    ]);

    register_action_with_requirements("decline_alliance", "handle_decline_alliance", [
        "authenticated"
    ]);

    log_info("Social actions registered");
}

// Hail another ship (quick "yo" greeting)
// params: { target_id: uuid }
fn handle_hail(ctx, params) {
    let target_id = params["target_id"];
    if target_id == () {
        return #{ success: false, error: "target_id required" };
    }

    let player_id = ctx["player_id"];
    let ship_id = ctx["ship_id"];
    let sector_id = ctx["sector_id"];

    // Can't hail self
    if target_id == ship_id {
        return #{ success: false, error: "Cannot hail yourself" };
    }

    let player = query_player(player_id);
    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    // Query target ship
    let target_ship = query_ship(target_id);
    if target_ship == () {
        return #{ success: false, error: "Target ship not found" };
    }

    // Check target is in same sector
    if target_ship["sector_id"] != sector_id {
        return #{ success: false, error: "Target is not in this sector" };
    }

    // Check target is a player ship
    if !target_ship["is_player"] {
        // NPC ships might respond differently
        return handle_hail_npc(player_id, target_ship);
    }

    // Get target player
    let target_owner_id = target_ship["owner_id"];
    if target_owner_id == () {
        return #{ success: false, error: "Target has no owner" };
    }

    // Send hail notification to target
    send_hail(target_owner_id, #{
        from_id: player_id,
        from_name: player["username"]
    });

    send_notification(player_id, "Hailed " + target_ship["name"]);

    emit_event("player_hailed", #{
        from_player_id: player_id,
        to_player_id: target_owner_id
    });

    #{ success: true }
}

// Handle hailing an NPC
fn handle_hail_npc(player_id, npc_ship) {
    let npc_name = npc_ship["name"];
    let ship_class = npc_ship["ship_class"];

    // Different NPC types respond differently
    let response = switch ship_class {
        "Pirate" | "Raider" => "The pirate vessel ignores your hail... or are they plotting something?",
        "Patrol" | "Military" => "Patrol acknowledges. Fly safe, civilian.",
        "Freighter" | "Transport" => "Freighter " + npc_name + " responds: 'Safe travels, friend.'",
        "Miner" => "Mining vessel responds: 'Busy extracting. Good luck out there.'",
        _ => npc_name + " acknowledges your hail."
    };

    send_notification(player_id, response);

    // Some NPCs might have special behavior on hail
    emit_event("npc_hailed", #{
        player_id: player_id,
        npc_ship_id: npc_ship["id"],
        npc_class: ship_class
    });

    #{ success: true, data: #{ response: response } }
}

// Accept alliance proposal
// params: { proposal_id: uuid }
fn handle_accept_alliance(ctx, params) {
    let proposal_id = params["proposal_id"];
    if proposal_id == () {
        return #{ success: false, error: "proposal_id required" };
    }

    let player_id = ctx["player_id"];
    let player = query_player(player_id);

    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    // Must be in a squadron
    let squadron_id = player["squadron_id"];
    if squadron_id == () {
        return #{ success: false, error: "Not in a squadron" };
    }

    // Must be leader to accept alliance
    if player["squadron_role"] != "leader" {
        return #{ success: false, error: "Only the squadron leader can accept alliances" };
    }

    // Get proposal
    let proposal = get_alliance_proposal(proposal_id);
    if proposal == () {
        return #{ success: false, error: "Alliance proposal not found or expired" };
    }

    // Verify this proposal is for our squadron
    if proposal["to_squadron_id"] != squadron_id {
        return #{ success: false, error: "This proposal is not for your squadron" };
    }

    let from_squadron_id = proposal["from_squadron_id"];
    let from_squadron = query_squadron(from_squadron_id);
    let to_squadron = query_squadron(squadron_id);

    if from_squadron == () {
        delete_alliance_proposal(proposal_id);
        return #{ success: false, error: "Proposing squadron no longer exists" };
    }

    // Create alliance
    create_alliance(from_squadron_id, squadron_id);

    // Delete proposal
    delete_alliance_proposal(proposal_id);

    // Notify both squadrons
    broadcast_to_squadron(from_squadron_id, "Alliance formed with [" + to_squadron["tag"] + "]!");
    broadcast_to_squadron(squadron_id, "Alliance formed with [" + from_squadron["tag"] + "]!");

    emit_event("alliance_formed", #{
        squadron_1: from_squadron_id,
        squadron_2: squadron_id
    });

    #{
        success: true,
        data: #{
            allied_with_id: from_squadron_id,
            allied_with_name: from_squadron["name"],
            allied_with_tag: from_squadron["tag"]
        }
    }
}

// Decline alliance proposal
// params: { proposal_id: uuid }
fn handle_decline_alliance(ctx, params) {
    let proposal_id = params["proposal_id"];
    if proposal_id == () {
        return #{ success: false, error: "proposal_id required" };
    }

    let player_id = ctx["player_id"];
    let player = query_player(player_id);

    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    // Must be in a squadron
    let squadron_id = player["squadron_id"];
    if squadron_id == () {
        return #{ success: false, error: "Not in a squadron" };
    }

    // Must be leader to decline
    if player["squadron_role"] != "leader" {
        return #{ success: false, error: "Only the squadron leader can decline alliances" };
    }

    // Get proposal
    let proposal = get_alliance_proposal(proposal_id);
    if proposal == () {
        return #{ success: false, error: "Alliance proposal not found" };
    }

    // Verify this proposal is for our squadron
    if proposal["to_squadron_id"] != squadron_id {
        return #{ success: false, error: "This proposal is not for your squadron" };
    }

    let from_squadron_id = proposal["from_squadron_id"];

    // Delete proposal
    delete_alliance_proposal(proposal_id);

    // Optionally notify the proposing squadron
    broadcast_to_squadron(from_squadron_id, "Alliance proposal was declined");

    #{ success: true }
}
