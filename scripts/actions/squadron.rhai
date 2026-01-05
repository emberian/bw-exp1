// Squadron action handlers
//
// Handles squadron creation, management, invitations, and leadership actions.

fn init() {
    register_action_with_requirements("create_squadron", "handle_create_squadron", [
        "authenticated"
    ]);

    register_action_with_requirements("leave_squadron", "handle_leave_squadron", [
        "authenticated"
    ]);

    register_action_with_requirements("invite_to_squadron", "handle_invite_to_squadron", [
        "authenticated"
    ]);

    register_action_with_requirements("accept_squadron_invite", "handle_accept_squadron_invite", [
        "authenticated"
    ]);

    register_action_with_requirements("decline_squadron_invite", "handle_decline_squadron_invite", [
        "authenticated"
    ]);

    // Squadron management actions (leader/officer only)
    register_action_with_requirements("squadron_action", "handle_squadron_action", [
        "authenticated"
    ]);

    log_info("Squadron actions registered");
}

// Create a new squadron
// params: { name: string, tag: string }
fn handle_create_squadron(ctx, params) {
    let name = params["name"];
    let tag = params["tag"];

    if name == () || name == "" {
        return #{ success: false, error: "Squadron name required" };
    }
    if tag == () || tag == "" {
        return #{ success: false, error: "Squadron tag required" };
    }

    // Validate name length
    if name.len() < 3 || name.len() > 32 {
        return #{ success: false, error: "Squadron name must be 3-32 characters" };
    }

    // Validate tag length
    if tag.len() < 2 || tag.len() > 5 {
        return #{ success: false, error: "Squadron tag must be 2-5 characters" };
    }

    let player_id = ctx["player_id"];
    let player = query_player(player_id);

    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    // Check player isn't already in a squadron
    if player["squadron_id"] != () {
        return #{ success: false, error: "Already in a squadron. Leave first." };
    }

    // Check minimum reputation (optional requirement)
    let min_rep = 50;
    if player["reputation"] < min_rep {
        return #{
            success: false,
            error: "Need " + min_rep + " reputation to create a squadron"
        };
    }

    // Check if tag is already taken
    if squadron_tag_exists(tag) {
        return #{ success: false, error: "Squadron tag already in use" };
    }

    // Create the squadron
    let squadron_id = create_squadron(name, tag, player_id);

    // Update player to be in squadron as leader
    modify_player(player_id, #{
        squadron_id: squadron_id,
        squadron_role: "leader"
    });

    send_notification(player_id, "Squadron [" + tag + "] " + name + " created!");

    emit_event("squadron_created", #{
        squadron_id: squadron_id,
        name: name,
        tag: tag,
        leader_id: player_id
    });

    #{
        success: true,
        data: #{
            squadron_id: squadron_id,
            name: name,
            tag: tag
        }
    }
}

// Leave current squadron
fn handle_leave_squadron(ctx, params) {
    let player_id = ctx["player_id"];
    let player = query_player(player_id);

    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    let squadron_id = player["squadron_id"];
    if squadron_id == () {
        return #{ success: false, error: "Not in a squadron" };
    }

    let squadron = query_squadron(squadron_id);
    if squadron == () {
        // Squadron doesn't exist, just clear player's membership
        modify_player(player_id, #{
            squadron_id: (),
            squadron_role: ()
        });
        return #{ success: true };
    }

    // Check if player is leader
    if squadron["leader_id"] == player_id {
        // Leader can't leave unless they transfer leadership or disband
        let member_count = squadron["member_count"];
        if member_count > 1 {
            return #{
                success: false,
                error: "Transfer leadership before leaving, or kick all members to disband"
            };
        }

        // Solo leader - disband squadron
        disband_squadron(squadron_id);
        modify_player(player_id, #{
            squadron_id: (),
            squadron_role: ()
        });

        send_notification(player_id, "Squadron disbanded");

        emit_event("squadron_disbanded", #{
            squadron_id: squadron_id,
            disbanded_by: player_id
        });

        return #{ success: true, data: #{ disbanded: true } };
    }

    // Regular member leaving
    remove_from_squadron(player_id, squadron_id);

    modify_player(player_id, #{
        squadron_id: (),
        squadron_role: ()
    });

    send_notification(player_id, "Left squadron [" + squadron["tag"] + "]");

    emit_event("squadron_member_left", #{
        squadron_id: squadron_id,
        player_id: player_id
    });

    #{ success: true }
}

// Invite a player to squadron
// params: { player_id: uuid }
fn handle_invite_to_squadron(ctx, params) {
    let target_id = params["player_id"];
    if target_id == () {
        return #{ success: false, error: "player_id required" };
    }

    let inviter_id = ctx["player_id"];
    let inviter = query_player(inviter_id);

    if inviter == () {
        return #{ success: false, error: "Player not found" };
    }

    let squadron_id = inviter["squadron_id"];
    if squadron_id == () {
        return #{ success: false, error: "Not in a squadron" };
    }

    // Check inviter has permission (leader or officer)
    let role = inviter["squadron_role"];
    if role != "leader" && role != "officer" {
        return #{ success: false, error: "Only leaders and officers can invite" };
    }

    // Check target exists
    let target = query_player(target_id);
    if target == () {
        return #{ success: false, error: "Target player not found" };
    }

    // Check target isn't already in a squadron
    if target["squadron_id"] != () {
        return #{ success: false, error: "Player is already in a squadron" };
    }

    let squadron = query_squadron(squadron_id);
    if squadron == () {
        return #{ success: false, error: "Squadron not found" };
    }

    // Create invitation
    let invite_id = create_squadron_invite(squadron_id, inviter_id, target_id);

    // Notify target
    send_squadron_invite(target_id, #{
        invite_id: invite_id,
        squadron_id: squadron_id,
        squadron_name: squadron["name"],
        squadron_tag: squadron["tag"],
        inviter_name: inviter["username"]
    });

    send_notification(inviter_id, "Invitation sent to " + target["username"]);

    #{
        success: true,
        data: #{
            invite_id: invite_id,
            target_name: target["username"]
        }
    }
}

// Accept squadron invitation
// params: { invite_id: uuid }
fn handle_accept_squadron_invite(ctx, params) {
    let invite_id = params["invite_id"];
    if invite_id == () {
        return #{ success: false, error: "invite_id required" };
    }

    let player_id = ctx["player_id"];
    let player = query_player(player_id);

    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    // Check player isn't already in a squadron
    if player["squadron_id"] != () {
        return #{ success: false, error: "Already in a squadron" };
    }

    // Get and validate invitation
    let invite = get_squadron_invite(invite_id);
    if invite == () {
        return #{ success: false, error: "Invitation not found or expired" };
    }

    if invite["target_id"] != player_id {
        return #{ success: false, error: "This invitation is not for you" };
    }

    let squadron_id = invite["squadron_id"];
    let squadron = query_squadron(squadron_id);
    if squadron == () {
        return #{ success: false, error: "Squadron no longer exists" };
    }

    // Add player to squadron
    add_to_squadron(player_id, squadron_id, "member");

    modify_player(player_id, #{
        squadron_id: squadron_id,
        squadron_role: "member"
    });

    // Delete invitation
    delete_squadron_invite(invite_id);

    send_notification(player_id, "Joined squadron [" + squadron["tag"] + "] " + squadron["name"]);

    // Notify squadron
    broadcast_to_squadron(squadron_id, player["username"] + " has joined the squadron!");

    emit_event("squadron_member_joined", #{
        squadron_id: squadron_id,
        player_id: player_id
    });

    #{
        success: true,
        data: #{
            squadron_id: squadron_id,
            squadron_name: squadron["name"],
            squadron_tag: squadron["tag"]
        }
    }
}

// Decline squadron invitation
// params: { invite_id: uuid }
fn handle_decline_squadron_invite(ctx, params) {
    let invite_id = params["invite_id"];
    if invite_id == () {
        return #{ success: false, error: "invite_id required" };
    }

    let player_id = ctx["player_id"];

    let invite = get_squadron_invite(invite_id);
    if invite == () {
        return #{ success: false, error: "Invitation not found" };
    }

    if invite["target_id"] != player_id {
        return #{ success: false, error: "This invitation is not for you" };
    }

    delete_squadron_invite(invite_id);

    #{ success: true }
}

// Handle various squadron management actions
// params: { action_type: string, ... }
fn handle_squadron_action(ctx, params) {
    let action_type = params["action_type"];
    if action_type == () {
        return #{ success: false, error: "action_type required" };
    }

    let player_id = ctx["player_id"];
    let player = query_player(player_id);

    if player == () {
        return #{ success: false, error: "Player not found" };
    }

    let squadron_id = player["squadron_id"];
    if squadron_id == () {
        return #{ success: false, error: "Not in a squadron" };
    }

    let squadron = query_squadron(squadron_id);
    if squadron == () {
        return #{ success: false, error: "Squadron not found" };
    }

    let role = player["squadron_role"];
    let is_leader = role == "leader";
    let is_officer = role == "officer";

    // Route to specific handler
    switch action_type {
        "promote_to_officer" => handle_promote_officer(player_id, squadron_id, params, is_leader),
        "demote_officer" => handle_demote_officer(player_id, squadron_id, params, is_leader),
        "kick_member" => handle_kick_member(player_id, squadron_id, params, is_leader, is_officer),
        "transfer_leadership" => handle_transfer_leadership(player_id, squadron_id, params, is_leader),
        "set_motto" => handle_set_motto(player_id, squadron_id, params, is_leader, is_officer),
        "enable_wargames" => handle_enable_wargames(player_id, squadron_id, params, is_leader),
        "enable_privateering" => handle_enable_privateering(player_id, squadron_id, params, is_leader),
        "declare_war" => handle_declare_war(player_id, squadron_id, params, is_leader),
        "make_peace" => handle_make_peace(player_id, squadron_id, params, is_leader),
        "form_alliance" => handle_form_alliance(player_id, squadron_id, params, is_leader),
        _ => #{ success: false, error: "Unknown squadron action: " + action_type }
    }
}

// Sub-handlers for squadron actions

fn handle_promote_officer(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can promote officers" };
    }

    let target_id = params["player_id"];
    if target_id == () {
        return #{ success: false, error: "player_id required" };
    }

    let target = query_player(target_id);
    if target == () || target["squadron_id"] != squadron_id {
        return #{ success: false, error: "Player not in your squadron" };
    }

    if target["squadron_role"] == "officer" {
        return #{ success: false, error: "Player is already an officer" };
    }

    modify_player(target_id, #{ squadron_role: "officer" });
    send_notification(target_id, "You have been promoted to officer!");
    broadcast_to_squadron(squadron_id, target["username"] + " has been promoted to officer");

    #{ success: true }
}

fn handle_demote_officer(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can demote officers" };
    }

    let target_id = params["player_id"];
    if target_id == () {
        return #{ success: false, error: "player_id required" };
    }

    let target = query_player(target_id);
    if target == () || target["squadron_id"] != squadron_id {
        return #{ success: false, error: "Player not in your squadron" };
    }

    if target["squadron_role"] != "officer" {
        return #{ success: false, error: "Player is not an officer" };
    }

    modify_player(target_id, #{ squadron_role: "member" });
    send_notification(target_id, "You have been demoted to member");

    #{ success: true }
}

fn handle_kick_member(player_id, squadron_id, params, is_leader, is_officer) {
    if !is_leader && !is_officer {
        return #{ success: false, error: "Only leaders and officers can kick members" };
    }

    let target_id = params["player_id"];
    if target_id == () {
        return #{ success: false, error: "player_id required" };
    }

    if target_id == player_id {
        return #{ success: false, error: "Cannot kick yourself" };
    }

    let target = query_player(target_id);
    if target == () || target["squadron_id"] != squadron_id {
        return #{ success: false, error: "Player not in your squadron" };
    }

    // Officers can't kick other officers or leader
    let target_role = target["squadron_role"];
    if !is_leader && (target_role == "leader" || target_role == "officer") {
        return #{ success: false, error: "Cannot kick leaders or officers" };
    }

    remove_from_squadron(target_id, squadron_id);
    modify_player(target_id, #{
        squadron_id: (),
        squadron_role: ()
    });

    send_notification(target_id, "You have been kicked from the squadron");
    broadcast_to_squadron(squadron_id, target["username"] + " has been kicked");

    #{ success: true }
}

fn handle_transfer_leadership(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can transfer leadership" };
    }

    let target_id = params["player_id"];
    if target_id == () {
        return #{ success: false, error: "player_id required" };
    }

    let target = query_player(target_id);
    if target == () || target["squadron_id"] != squadron_id {
        return #{ success: false, error: "Player not in your squadron" };
    }

    // Transfer leadership
    modify_player(target_id, #{ squadron_role: "leader" });
    modify_player(player_id, #{ squadron_role: "officer" });
    update_squadron_leader(squadron_id, target_id);

    send_notification(target_id, "You are now the squadron leader!");
    broadcast_to_squadron(squadron_id, target["username"] + " is now the squadron leader");

    #{ success: true }
}

fn handle_set_motto(player_id, squadron_id, params, is_leader, is_officer) {
    if !is_leader && !is_officer {
        return #{ success: false, error: "Only leaders and officers can set the motto" };
    }

    let motto = params["motto"];
    if motto == () {
        return #{ success: false, error: "motto required" };
    }

    if motto.len() > 100 {
        return #{ success: false, error: "Motto must be 100 characters or less" };
    }

    modify_squadron(squadron_id, #{ motto: motto });
    broadcast_to_squadron(squadron_id, "Squadron motto updated: " + motto);

    #{ success: true }
}

fn handle_enable_wargames(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can toggle wargames" };
    }

    let enabled = params["enabled"];
    if enabled == () {
        return #{ success: false, error: "enabled required" };
    }

    modify_squadron(squadron_id, #{ wargames_enabled: enabled });

    let msg = if enabled { "Wargames enabled - friendly fire allowed" } else { "Wargames disabled" };
    broadcast_to_squadron(squadron_id, msg);

    #{ success: true }
}

fn handle_enable_privateering(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can toggle privateering" };
    }

    let enabled = params["enabled"];
    if enabled == () {
        return #{ success: false, error: "enabled required" };
    }

    modify_squadron(squadron_id, #{ privateering_enabled: enabled });

    let msg = if enabled { "Privateering enabled" } else { "Privateering disabled" };
    broadcast_to_squadron(squadron_id, msg);

    #{ success: true }
}

fn handle_declare_war(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can declare war" };
    }

    let target_squadron_id = params["squadron_id"];
    if target_squadron_id == () {
        return #{ success: false, error: "squadron_id required" };
    }

    if target_squadron_id == squadron_id {
        return #{ success: false, error: "Cannot declare war on yourself" };
    }

    let target_squadron = query_squadron(target_squadron_id);
    if target_squadron == () {
        return #{ success: false, error: "Target squadron not found" };
    }

    // Check not already at war
    if are_squadrons_at_war(squadron_id, target_squadron_id) {
        return #{ success: false, error: "Already at war with this squadron" };
    }

    declare_war(squadron_id, target_squadron_id);

    let squadron = query_squadron(squadron_id);
    broadcast_to_squadron(squadron_id, "War declared on [" + target_squadron["tag"] + "]!");
    broadcast_to_squadron(target_squadron_id, "[" + squadron["tag"] + "] has declared war!");

    emit_event("war_declared", #{
        aggressor_id: squadron_id,
        defender_id: target_squadron_id
    });

    #{ success: true }
}

fn handle_make_peace(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can negotiate peace" };
    }

    let target_squadron_id = params["squadron_id"];
    if target_squadron_id == () {
        return #{ success: false, error: "squadron_id required" };
    }

    if !are_squadrons_at_war(squadron_id, target_squadron_id) {
        return #{ success: false, error: "Not at war with this squadron" };
    }

    // Create peace proposal (requires both sides to accept)
    create_peace_proposal(squadron_id, target_squadron_id);

    let target_squadron = query_squadron(target_squadron_id);
    broadcast_to_squadron(target_squadron_id, "Peace proposal received from another squadron");

    #{ success: true, data: #{ proposal_sent: true } }
}

fn handle_form_alliance(player_id, squadron_id, params, is_leader) {
    if !is_leader {
        return #{ success: false, error: "Only the leader can propose alliances" };
    }

    let target_squadron_id = params["squadron_id"];
    if target_squadron_id == () {
        return #{ success: false, error: "squadron_id required" };
    }

    if target_squadron_id == squadron_id {
        return #{ success: false, error: "Cannot ally with yourself" };
    }

    let target_squadron = query_squadron(target_squadron_id);
    if target_squadron == () {
        return #{ success: false, error: "Target squadron not found" };
    }

    // Check not already allied
    if are_squadrons_allied(squadron_id, target_squadron_id) {
        return #{ success: false, error: "Already allied with this squadron" };
    }

    // Check not at war
    if are_squadrons_at_war(squadron_id, target_squadron_id) {
        return #{ success: false, error: "Cannot ally while at war" };
    }

    // Create alliance proposal
    let proposal_id = create_alliance_proposal(squadron_id, target_squadron_id);

    let squadron = query_squadron(squadron_id);
    send_alliance_proposal(target_squadron_id, #{
        proposal_id: proposal_id,
        from_squadron_id: squadron_id,
        from_squadron_name: squadron["name"],
        from_squadron_tag: squadron["tag"]
    });

    #{ success: true, data: #{ proposal_id: proposal_id } }
}
