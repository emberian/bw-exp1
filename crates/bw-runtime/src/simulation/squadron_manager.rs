//! Squadron Management System
//!
//! Handles squadron CRUD, membership, sector control, building, and PvP rules.

use uuid::Uuid;
use tracing::{debug, info, instrument, warn};

use bw_core::models::{Squadron, SquadronBuildingType, SquadronRank, Location, LocationType, StationService, Ship, ShipClass};
use bw_game::systems::spend_reputation;
use bw_shared::dto::SquadronDto;

use crate::{GameState, SquadronInvite, AllianceProposal, ContestedSector};

/// Result of a squadron operation.
#[derive(Debug)]
pub struct SquadronResult {
    pub success: bool,
    pub message: String,
    pub squadron: Option<SquadronDto>,
}

/// Create a new squadron.
#[instrument(skip(state), fields(player_id = %player_id, name = %name, tag = %tag))]
pub fn create_squadron(
    state: &GameState,
    player_id: Uuid,
    name: String,
    tag: String,
) -> SquadronResult {
    // Validate tag format (2-5 uppercase chars)
    if tag.len() < 2 || tag.len() > 5 || !tag.chars().all(|c| c.is_ascii_uppercase()) {
        return SquadronResult {
            success: false,
            message: "Tag must be 2-5 uppercase letters".to_string(),
            squadron: None,
        };
    }

    // Validate name length
    if name.len() < 3 || name.len() > 32 {
        return SquadronResult {
            success: false,
            message: "Name must be 3-32 characters".to_string(),
            squadron: None,
        };
    }

    // Check if player exists
    let mut player = match state.player_data.get_mut(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    // Check if player already in a squadron
    if player.squadron_id.is_some() {
        return SquadronResult {
            success: false,
            message: "You are already in a squadron. Leave it first.".to_string(),
            squadron: None,
        };
    }

    // Check if tag is unique
    for squadron in state.squadrons.iter() {
        if squadron.tag.eq_ignore_ascii_case(&tag) {
            return SquadronResult {
                success: false,
                message: "Tag is already in use".to_string(),
                squadron: None,
            };
        }
    }

    // Cost to create squadron: 50 reputation
    const CREATION_COST: i32 = 50;
    if !spend_reputation(&mut player.resources, CREATION_COST) {
        return SquadronResult {
            success: false,
            message: format!(
                "Insufficient reputation. Creating a squadron costs {} rep.",
                CREATION_COST
            ),
            squadron: None,
        };
    }

    // Create squadron
    let squadron = Squadron::new(name.clone(), tag.clone(), player_id);
    let squadron_id = squadron.id;

    // Get player username for DTO
    let leader_name = player.username.clone();

    // Update player
    player.squadron_id = Some(squadron_id);
    player.squadron_rank = Some(SquadronRank::Leader);
    drop(player);

    // Build DTO before inserting (no officers yet on creation)
    let dto = build_squadron_dto(&squadron, &leader_name, vec![]);

    // Store squadron
    state.squadrons.insert(squadron_id, squadron);

    info!(squadron_id = %squadron_id, "Squadron created");

    SquadronResult {
        success: true,
        message: format!("Squadron '{}' [{}] created successfully!", name, tag),
        squadron: Some(dto),
    }
}

/// Result of an invitation that includes invite details for notification.
#[derive(Debug)]
pub struct InviteResult {
    pub success: bool,
    pub message: String,
    pub invite: Option<SquadronInvite>,
}

/// Invite a player to the squadron (creates pending invite).
#[instrument(skip(state), fields(inviter_id = %inviter_id, invitee_id = %invitee_id))]
pub fn invite_to_squadron(
    state: &GameState,
    inviter_id: Uuid,
    invitee_id: Uuid,
) -> InviteResult {
    // Get inviter's squadron
    let inviter = match state.player_data.get(&inviter_id) {
        Some(p) => p,
        None => {
            return InviteResult {
                success: false,
                message: "Player not found".to_string(),
                invite: None,
            }
        }
    };

    let squadron_id = match inviter.squadron_id {
        Some(id) => id,
        None => {
            return InviteResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                invite: None,
            }
        }
    };

    let inviter_name = inviter.username.clone();
    drop(inviter);

    // Get squadron
    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return InviteResult {
                success: false,
                message: "Squadron not found".to_string(),
                invite: None,
            }
        }
    };

    // Check if inviter can invite (leader, officer, or members_can_invite)
    let can_invite = squadron.can_manage(inviter_id) || squadron.settings.members_can_invite;
    if !can_invite {
        return InviteResult {
            success: false,
            message: "You don't have permission to invite".to_string(),
            invite: None,
        };
    }

    let squadron_name = squadron.name.clone();
    drop(squadron);

    // Check invitee
    let invitee = match state.player_data.get(&invitee_id) {
        Some(p) => p,
        None => {
            return InviteResult {
                success: false,
                message: "Target player not found".to_string(),
                invite: None,
            }
        }
    };

    if invitee.squadron_id.is_some() {
        return InviteResult {
            success: false,
            message: "Target player is already in a squadron".to_string(),
            invite: None,
        };
    }

    // Check if player already has a pending invite from this squadron
    if state.pending_squadron_invites.contains_key(&invitee_id) {
        return InviteResult {
            success: false,
            message: "Player already has a pending invitation".to_string(),
            invite: None,
        };
    }

    let invitee_name = invitee.username.clone();
    drop(invitee);

    // Create pending invite
    let invite = SquadronInvite {
        id: Uuid::new_v4(),
        squadron_id,
        squadron_name: squadron_name.clone(),
        inviter_id,
        inviter_name: inviter_name.clone(),
        invitee_id,
        created_at: state.get_tick(),
    };

    let invite_clone = invite.clone();
    state.pending_squadron_invites.insert(invitee_id, invite);

    info!(
        invitee_name = %invitee_name,
        squadron_name = %squadron_name,
        "Player invited to squadron"
    );

    InviteResult {
        success: true,
        message: format!("Invitation sent to {}", invitee_name),
        invite: Some(invite_clone),
    }
}

/// Accept a pending squadron invite.
pub fn accept_squadron_invite(
    state: &GameState,
    player_id: Uuid,
    invite_id: Uuid,
) -> SquadronResult {
    // Get and remove invite
    let invite = match state.pending_squadron_invites.remove(&player_id) {
        Some((_, invite)) if invite.id == invite_id => invite,
        _ => {
            return SquadronResult {
                success: false,
                message: "Invitation not found or expired".to_string(),
                squadron: None,
            }
        }
    };

    // Verify player still can join
    let mut player = match state.player_data.get_mut(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    if player.squadron_id.is_some() {
        return SquadronResult {
            success: false,
            message: "You are already in a squadron".to_string(),
            squadron: None,
        };
    }

    // Update player
    player.squadron_id = Some(invite.squadron_id);
    player.squadron_rank = Some(SquadronRank::Member);
    let player_name = player.username.clone();
    drop(player);

    // Add to squadron
    let mut squadron = match state.squadrons.get_mut(&invite.squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron no longer exists".to_string(),
                squadron: None,
            }
        }
    };

    squadron.add_member(player_id);
    let squadron_name = squadron.name.clone();
    drop(squadron);

    info!(player_name = %player_name, squadron_name = %squadron_name, "Player joined squadron");

    SquadronResult {
        success: true,
        message: format!("Welcome to {}!", squadron_name),
        squadron: None,
    }
}

/// Decline a pending squadron invite.
pub fn decline_squadron_invite(
    state: &GameState,
    player_id: Uuid,
    invite_id: Uuid,
) -> SquadronResult {
    // Get and remove invite
    let invite = match state.pending_squadron_invites.remove(&player_id) {
        Some((_, invite)) if invite.id == invite_id => invite,
        _ => {
            return SquadronResult {
                success: false,
                message: "Invitation not found or expired".to_string(),
                squadron: None,
            }
        }
    };

    debug!(squadron_name = %invite.squadron_name, "Player declined squadron invitation");

    SquadronResult {
        success: true,
        message: "Invitation declined".to_string(),
        squadron: None,
    }
}

/// Leave current squadron.
pub fn leave_squadron(state: &GameState, player_id: Uuid) -> SquadronResult {
    // Get player
    let mut player = match state.player_data.get_mut(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                squadron: None,
            }
        }
    };

    // Check if player is leader
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            // Squadron doesn't exist, just clear player's squadron_id
            player.squadron_id = None;
            player.squadron_rank = None;
            return SquadronResult {
                success: true,
                message: "Left squadron (squadron no longer exists)".to_string(),
                squadron: None,
            };
        }
    };

    if squadron.is_leader(player_id) {
        if squadron.member_count() == 1 {
            // Last member, disband squadron
            let squadron_name = squadron.name.clone();
            drop(squadron);
            state.squadrons.remove(&squadron_id);

            player.squadron_id = None;
            player.squadron_rank = None;

            info!(squadron_name = %squadron_name, "Squadron disbanded");

            return SquadronResult {
                success: true,
                message: "Squadron disbanded (you were the last member)".to_string(),
                squadron: None,
            };
        } else {
            return SquadronResult {
                success: false,
                message: "You must transfer leadership before leaving".to_string(),
                squadron: None,
            };
        }
    }

    // Remove from squadron
    squadron.remove_member(player_id);
    drop(squadron);

    player.squadron_id = None;
    player.squadron_rank = None;

    SquadronResult {
        success: true,
        message: "You have left the squadron".to_string(),
        squadron: None,
    }
}

/// Process a squadron action.
pub fn process_squadron_action(
    state: &GameState,
    player_id: Uuid,
    action: bw_shared::SquadronAction,
) -> SquadronResult {
    use bw_shared::SquadronAction;

    // Get player's squadron
    let player = match state.player_data.get(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                squadron: None,
            }
        }
    };

    drop(player);

    match action {
        SquadronAction::PromoteToOfficer { player_id: target_id } => {
            promote_to_officer(state, player_id, squadron_id, target_id)
        }
        SquadronAction::DemoteOfficer { player_id: target_id } => {
            demote_officer(state, player_id, squadron_id, target_id)
        }
        SquadronAction::KickMember { player_id: target_id } => {
            kick_member(state, player_id, squadron_id, target_id)
        }
        SquadronAction::TransferLeadership { player_id: target_id } => {
            transfer_leadership(state, player_id, squadron_id, target_id)
        }
        SquadronAction::SetMotto { motto } => {
            set_motto(state, player_id, squadron_id, motto)
        }
        SquadronAction::EnableWargames { enabled } => {
            toggle_wargames(state, player_id, squadron_id, enabled)
        }
        SquadronAction::EnablePrivateering { enabled } => {
            toggle_privateering(state, player_id, squadron_id, enabled)
        }
        SquadronAction::DeclareWar { squadron_id: target_squadron_id } => {
            declare_war(state, player_id, squadron_id, target_squadron_id)
        }
        SquadronAction::MakePeace { squadron_id: target_squadron_id } => {
            make_peace(state, player_id, squadron_id, target_squadron_id)
        }
        SquadronAction::FormAlliance { squadron_id: target_squadron_id } => {
            form_alliance(state, player_id, squadron_id, target_squadron_id)
        }
    }
}

fn promote_to_officer(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_id: Uuid,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can promote officers".to_string(),
            squadron: None,
        };
    }

    if !squadron.is_member(target_id) {
        return SquadronResult {
            success: false,
            message: "Target is not a squadron member".to_string(),
            squadron: None,
        };
    }

    squadron.promote_to_officer(target_id);
    drop(squadron);

    // Update player rank
    if let Some(mut player) = state.player_data.get_mut(&target_id) {
        player.squadron_rank = Some(SquadronRank::Officer);
    }

    SquadronResult {
        success: true,
        message: "Member promoted to officer".to_string(),
        squadron: None,
    }
}

fn demote_officer(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_id: Uuid,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can demote officers".to_string(),
            squadron: None,
        };
    }

    squadron.demote_officer(target_id);
    drop(squadron);

    // Update player rank
    if let Some(mut player) = state.player_data.get_mut(&target_id) {
        player.squadron_rank = Some(SquadronRank::Member);
    }

    SquadronResult {
        success: true,
        message: "Officer demoted to member".to_string(),
        squadron: None,
    }
}

fn kick_member(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_id: Uuid,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    // Only leader can kick officers, officers can kick members
    if squadron.is_leader(target_id) {
        return SquadronResult {
            success: false,
            message: "Cannot kick the squadron leader".to_string(),
            squadron: None,
        };
    }

    let can_kick = if squadron.is_officer(target_id) {
        squadron.is_leader(actor_id)
    } else {
        squadron.can_manage(actor_id)
    };

    if !can_kick {
        return SquadronResult {
            success: false,
            message: "You don't have permission to kick this member".to_string(),
            squadron: None,
        };
    }

    squadron.remove_member(target_id);
    drop(squadron);

    // Update player
    if let Some(mut player) = state.player_data.get_mut(&target_id) {
        player.squadron_id = None;
        player.squadron_rank = None;
    }

    SquadronResult {
        success: true,
        message: "Member kicked from squadron".to_string(),
        squadron: None,
    }
}

fn transfer_leadership(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_id: Uuid,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can transfer leadership".to_string(),
            squadron: None,
        };
    }

    if !squadron.is_member(target_id) {
        return SquadronResult {
            success: false,
            message: "Target is not a squadron member".to_string(),
            squadron: None,
        };
    }

    squadron.transfer_leadership(target_id);
    drop(squadron);

    // Update player ranks
    if let Some(mut player) = state.player_data.get_mut(&actor_id) {
        player.squadron_rank = Some(SquadronRank::Officer);
    }
    if let Some(mut player) = state.player_data.get_mut(&target_id) {
        player.squadron_rank = Some(SquadronRank::Leader);
    }

    SquadronResult {
        success: true,
        message: "Leadership transferred successfully".to_string(),
        squadron: None,
    }
}

fn set_motto(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    motto: String,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.can_manage(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only officers and leader can set the motto".to_string(),
            squadron: None,
        };
    }

    if motto.len() > 128 {
        return SquadronResult {
            success: false,
            message: "Motto must be 128 characters or less".to_string(),
            squadron: None,
        };
    }

    squadron.motto = if motto.is_empty() { None } else { Some(motto) };

    SquadronResult {
        success: true,
        message: "Squadron motto updated".to_string(),
        squadron: None,
    }
}

fn toggle_wargames(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    enabled: bool,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can toggle war games".to_string(),
            squadron: None,
        };
    }

    squadron.wargames_enabled = enabled;

    let status = if enabled { "enabled" } else { "disabled" };
    SquadronResult {
        success: true,
        message: format!("War games {}", status),
        squadron: None,
    }
}

fn toggle_privateering(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    enabled: bool,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can toggle privateering".to_string(),
            squadron: None,
        };
    }

    squadron.privateering_enabled = enabled;

    let status = if enabled { "enabled" } else { "disabled" };
    SquadronResult {
        success: true,
        message: format!("Privateering {}. Your squadron is now open to attacks from other squadrons.", status),
        squadron: None,
    }
}

fn declare_war(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_squadron_id: Uuid,
) -> SquadronResult {
    if squadron_id == target_squadron_id {
        return SquadronResult {
            success: false,
            message: "Cannot declare war on your own squadron".to_string(),
            squadron: None,
        };
    }

    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can declare war".to_string(),
            squadron: None,
        };
    }

    // Check target squadron exists
    if !state.squadrons.contains_key(&target_squadron_id) {
        return SquadronResult {
            success: false,
            message: "Target squadron not found".to_string(),
            squadron: None,
        };
    }

    squadron.declare_war(target_squadron_id);
    drop(squadron);

    // The target squadron can choose to reciprocate
    info!(
        target_squadron_id = %target_squadron_id,
        "Squadron declared war"
    );

    SquadronResult {
        success: true,
        message: "War declared! Your members can now engage their members.".to_string(),
        squadron: None,
    }
}

fn make_peace(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_squadron_id: Uuid,
) -> SquadronResult {
    let mut squadron = match state.squadrons.get_mut(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can make peace".to_string(),
            squadron: None,
        };
    }

    if !squadron.is_hostile_with(target_squadron_id) {
        return SquadronResult {
            success: false,
            message: "You are not at war with that squadron".to_string(),
            squadron: None,
        };
    }

    squadron.make_peace(target_squadron_id);

    info!(target_squadron_id = %target_squadron_id, "Squadron made peace");

    SquadronResult {
        success: true,
        message: "Peace declared. You can no longer engage their members.".to_string(),
        squadron: None,
    }
}

/// Result of an alliance proposal.
#[derive(Debug)]
pub struct AllianceProposalResult {
    pub success: bool,
    pub message: String,
    pub proposal: Option<AllianceProposal>,
    pub target_leader_id: Option<Uuid>,
}

fn form_alliance(
    state: &GameState,
    actor_id: Uuid,
    squadron_id: Uuid,
    target_squadron_id: Uuid,
) -> SquadronResult {
    if squadron_id == target_squadron_id {
        return SquadronResult {
            success: false,
            message: "Cannot form alliance with yourself".to_string(),
            squadron: None,
        };
    }

    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(actor_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can form alliances".to_string(),
            squadron: None,
        };
    }

    let from_squadron_name = squadron.name.clone();
    drop(squadron);

    let target = match state.squadrons.get(&target_squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Target squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    let target_name = target.name.clone();
    let _target_leader_id = target.leader_id;
    drop(target);

    // Check if already allied
    if let Some(s) = state.squadrons.get(&squadron_id)
        && s.allied_squadrons.contains(&target_squadron_id) {
            return SquadronResult {
                success: false,
                message: "Already allied with this squadron".to_string(),
                squadron: None,
            };
        }

    // Check if there's already a pending proposal
    if state.pending_alliances.contains_key(&target_squadron_id) {
        return SquadronResult {
            success: false,
            message: "Target squadron already has a pending alliance proposal".to_string(),
            squadron: None,
        };
    }

    // Create pending proposal
    let proposal = AllianceProposal {
        id: Uuid::new_v4(),
        from_squadron_id: squadron_id,
        from_squadron_name: from_squadron_name.clone(),
        to_squadron_id: target_squadron_id,
        created_at: state.get_tick(),
    };

    state.pending_alliances.insert(target_squadron_id, proposal);

    info!(from = %from_squadron_name, to = %target_name, "Squadron proposed alliance");

    SquadronResult {
        success: true,
        message: format!("Alliance proposal sent to {}!", target_name),
        squadron: None,
    }
}

/// Accept a pending alliance proposal.
pub fn accept_alliance(
    state: &GameState,
    player_id: Uuid,
    proposal_id: Uuid,
) -> SquadronResult {
    // Get player's squadron
    let player = match state.player_data.get(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                squadron: None,
            }
        }
    };
    drop(player);

    // Check if player is leader
    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(player_id) {
        return SquadronResult {
            success: false,
            message: "Only the squadron leader can accept alliances".to_string(),
            squadron: None,
        };
    }
    drop(squadron);

    // Get and remove proposal
    let proposal = match state.pending_alliances.remove(&squadron_id) {
        Some((_, proposal)) if proposal.id == proposal_id => proposal,
        _ => {
            return SquadronResult {
                success: false,
                message: "Alliance proposal not found or expired".to_string(),
                squadron: None,
            }
        }
    };

    // Form the alliance (both ways)
    if let Some(mut from_squadron) = state.squadrons.get_mut(&proposal.from_squadron_id) {
        from_squadron.form_alliance(squadron_id);
    }

    if let Some(mut to_squadron) = state.squadrons.get_mut(&squadron_id) {
        to_squadron.form_alliance(proposal.from_squadron_id);
    }

    info!(
        from_squadron_id = %proposal.from_squadron_id,
        to_squadron_id = %squadron_id,
        "Alliance formed"
    );

    SquadronResult {
        success: true,
        message: format!("Alliance formed with {}!", proposal.from_squadron_name),
        squadron: None,
    }
}

/// Decline a pending alliance proposal.
pub fn decline_alliance(
    state: &GameState,
    player_id: Uuid,
    proposal_id: Uuid,
) -> SquadronResult {
    // Get player's squadron
    let player = match state.player_data.get(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                squadron: None,
            }
        }
    };
    drop(player);

    // Check if player is leader
    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(player_id) {
        return SquadronResult {
            success: false,
            message: "Only the squadron leader can decline alliances".to_string(),
            squadron: None,
        };
    }
    drop(squadron);

    // Get and remove proposal
    let proposal = match state.pending_alliances.remove(&squadron_id) {
        Some((_, proposal)) if proposal.id == proposal_id => proposal,
        _ => {
            return SquadronResult {
                success: false,
                message: "Alliance proposal not found or expired".to_string(),
                squadron: None,
            }
        }
    };

    debug!(from_squadron = %proposal.from_squadron_name, "Alliance proposal declined");

    SquadronResult {
        success: true,
        message: "Alliance proposal declined".to_string(),
        squadron: None,
    }
}

// === Sector Control ===

/// Attempt to claim control of a sector.
pub fn claim_sector_control(
    state: &GameState,
    player_id: Uuid,
    sector_id: Uuid,
) -> SquadronResult {
    // Get player's squadron
    let player = match state.player_data.get(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You must be in a squadron to claim sectors".to_string(),
                squadron: None,
            }
        }
    };

    drop(player);

    // Check if player is leader or officer
    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.can_manage(player_id) {
        return SquadronResult {
            success: false,
            message: "Only officers can claim sectors".to_string(),
            squadron: None,
        };
    }

    drop(squadron);

    // Get sector
    let mut sector = match state.sectors.get_mut(&sector_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Sector not found".to_string(),
                squadron: None,
            }
        }
    };

    // Check if already controlled
    if let Some(controller_id) = sector.sector.controlling_squadron {
        if controller_id == squadron_id {
            return SquadronResult {
                success: false,
                message: "Your squadron already controls this sector".to_string(),
                squadron: None,
            };
        }

        // Need to contest - check if squadrons are at war
        let attacker_squadron = state.squadrons.get(&squadron_id);
        let is_hostile = attacker_squadron
            .map(|s| s.is_hostile_with(controller_id))
            .unwrap_or(false);

        if !is_hostile {
            return SquadronResult {
                success: false,
                message: "Sector is controlled by another squadron. Declare war to contest.".to_string(),
                squadron: None,
            };
        }

        drop(sector);

        // Check if already contested
        if let Some(mut contest) = state.contested_sectors.get_mut(&sector_id) {
            // Already contested - add influence based on presence
            let attacker_ships = count_squadron_ships_in_sector(state, squadron_id, sector_id);
            contest.attacker_influence += attacker_ships;

            // Check if contest resolved
            if contest.attacker_influence >= 100 {
                // Attacker wins!
                let defender_id = contest.defending_squadron_id;
                drop(contest);

                // Transfer control
                if let Some(mut sector) = state.sectors.get_mut(&sector_id) {
                    sector.sector.controlling_squadron = Some(squadron_id);
                }

                // Update squadron stats
                if let Some(mut defender) = state.squadrons.get_mut(&defender_id) {
                    defender.patrol_sectors.retain(|&id| id != sector_id);
                    defender.stats.sectors_controlled = defender.patrol_sectors.len() as i32;
                }

                if let Some(mut attacker) = state.squadrons.get_mut(&squadron_id)
                    && !attacker.patrol_sectors.contains(&sector_id) {
                        attacker.patrol_sectors.push(sector_id);
                        attacker.stats.sectors_controlled = attacker.patrol_sectors.len() as i32;
                    }

                state.contested_sectors.remove(&sector_id);

                info!(squadron_id = %squadron_id, sector_id = %sector_id, from = %defender_id, "Squadron captured sector");

                return SquadronResult {
                    success: true,
                    message: "Victory! Your squadron has captured this sector!".to_string(),
                    squadron: None,
                };
            }

            return SquadronResult {
                success: true,
                message: format!(
                    "Contest continues! Your influence: {}/100. Keep fighting for control!",
                    contest.attacker_influence
                ),
                squadron: None,
            };
        }

        // Start new contest
        let contest = ContestedSector {
            sector_id,
            defending_squadron_id: controller_id,
            attacking_squadron_id: squadron_id,
            defender_influence: 50, // Defender starts with advantage
            attacker_influence: 10, // Attacker starts with initial claim
            started_at: state.get_tick(),
        };

        state.contested_sectors.insert(sector_id, contest);

        let sector_name = state.sectors.get(&sector_id)
            .map(|s| s.sector.name.clone())
            .unwrap_or_default();

        info!(
            squadron_id = %squadron_id,
            sector_name = %sector_name,
            defender = %controller_id,
            "Sector contest initiated"
        );

        return SquadronResult {
            success: true,
            message: format!(
                "Contest initiated for sector '{}'! Fight for control to claim it.",
                sector_name
            ),
            squadron: None,
        };
    }

    // Claim uncontrolled sector
    sector.sector.controlling_squadron = Some(squadron_id);
    let sector_name = sector.sector.name.clone();
    drop(sector);

    // Update squadron
    if let Some(mut squadron) = state.squadrons.get_mut(&squadron_id)
        && !squadron.patrol_sectors.contains(&sector_id) {
            squadron.patrol_sectors.push(sector_id);
            squadron.stats.sectors_controlled = squadron.patrol_sectors.len() as i32;
        }

    info!(squadron_id = %squadron_id, sector_name = %sector_name, "Squadron claimed sector");

    SquadronResult {
        success: true,
        message: format!("Your squadron now controls sector '{}'!", sector_name),
        squadron: None,
    }
}

/// Release control of a sector.
pub fn release_sector_control(
    state: &GameState,
    player_id: Uuid,
    sector_id: Uuid,
) -> SquadronResult {
    // Get player's squadron
    let player = match state.player_data.get(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                squadron: None,
            }
        }
    };

    drop(player);

    // Check if leader
    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.is_leader(player_id) {
        return SquadronResult {
            success: false,
            message: "Only the leader can release sectors".to_string(),
            squadron: None,
        };
    }

    drop(squadron);

    // Get sector
    let mut sector = match state.sectors.get_mut(&sector_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Sector not found".to_string(),
                squadron: None,
            }
        }
    };

    // Check if squadron controls it
    if sector.sector.controlling_squadron != Some(squadron_id) {
        return SquadronResult {
            success: false,
            message: "Your squadron does not control this sector".to_string(),
            squadron: None,
        };
    }

    sector.sector.controlling_squadron = None;
    let sector_name = sector.sector.name.clone();
    drop(sector);

    // Update squadron
    if let Some(mut squadron) = state.squadrons.get_mut(&squadron_id) {
        squadron.patrol_sectors.retain(|&id| id != sector_id);
        squadron.stats.sectors_controlled = squadron.patrol_sectors.len() as i32;
    }

    info!(squadron_id = %squadron_id, sector_name = %sector_name, "Squadron released sector");

    SquadronResult {
        success: true,
        message: format!("Released control of sector '{}'", sector_name),
        squadron: None,
    }
}

// === Building System ===

/// Build a squadron structure (costs reputation).
pub fn build_structure(
    state: &GameState,
    player_id: Uuid,
    building_type: SquadronBuildingType,
    sector_id: Uuid,
) -> SquadronResult {
    // Get player's squadron
    let mut player = match state.player_data.get_mut(&player_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You must be in a squadron to build".to_string(),
                squadron: None,
            }
        }
    };

    // Get squadron and check permissions
    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Squadron not found".to_string(),
                squadron: None,
            }
        }
    };

    if !squadron.can_manage(player_id) {
        return SquadronResult {
            success: false,
            message: "Only officers can build structures".to_string(),
            squadron: None,
        };
    }

    // Check if squadron controls the sector (for stations/outposts)
    let sector = match state.sectors.get(&sector_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Sector not found".to_string(),
                squadron: None,
            }
        }
    };

    let requires_sector_control = matches!(
        building_type,
        SquadronBuildingType::Outpost
            | SquadronBuildingType::Station
            | SquadronBuildingType::Shipyard
    );

    if requires_sector_control && sector.sector.controlling_squadron != Some(squadron_id) {
        return SquadronResult {
            success: false,
            message: "Your squadron must control this sector to build here".to_string(),
            squadron: None,
        };
    }

    drop(sector);

    // Get build cost
    let cost = squadron.build_cost(building_type);
    drop(squadron);

    // Check and spend reputation
    if !spend_reputation(&mut player.resources, cost) {
        return SquadronResult {
            success: false,
            message: format!(
                "Insufficient reputation. Building {:?} costs {} rep.",
                building_type, cost
            ),
            squadron: None,
        };
    }

    drop(player);

    // Get squadron name for naming buildings
    let squadron_name = state.squadrons.get(&squadron_id)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Get sector for position and adding the entity
    let mut sector = match state.sectors.get_mut(&sector_id) {
        Some(s) => s,
        None => {
            return SquadronResult {
                success: false,
                message: "Sector not found".to_string(),
                squadron: None,
            }
        }
    };

    // Generate random position within sector
    let position = sector.sector.random_position();
    let building_name = format!("{:?}", building_type);

    match building_type {
        SquadronBuildingType::Outpost => {
            // Create a small outpost location
            let mut location = Location::new(
                format!("{} Outpost", squadron_name),
                LocationType::CivilianStation,
                position,
            );
            location.description = format!("A small outpost operated by {}", squadron_name);
            location.services = vec![StationService::Refuel, StationService::Repair];
            sector.sector.locations.push(location);
        }
        SquadronBuildingType::Station => {
            // Create a full station with services
            let mut location = Location::new(
                format!("{} Station", squadron_name),
                LocationType::CivilianStation,
                position,
            );
            location.description = format!("A station operated by {}", squadron_name);
            location.services = vec![
                StationService::Refuel,
                StationService::Rearm,
                StationService::Repair,
                StationService::Trade,
            ];
            sector.sector.locations.push(location);
        }
        SquadronBuildingType::Shipyard => {
            // Create a shipyard
            let mut location = Location::new(
                format!("{} Shipyard", squadron_name),
                LocationType::Shipyard,
                position,
            );
            location.description = format!("A shipyard operated by {}", squadron_name);
            sector.sector.locations.push(location);
        }
        SquadronBuildingType::PatrolShip | SquadronBuildingType::DefenseShip => {
            // Create an NPC ship for the squadron
            let ship_class = if building_type == SquadronBuildingType::PatrolShip {
                ShipClass::PatrolCorvette
            } else {
                ShipClass::Frigate
            };

            let ship_name = format!("{} {}", squadron_name,
                if building_type == SquadronBuildingType::PatrolShip { "Patrol" } else { "Defense" }
            );

            let ship = Ship::new_npc_ship(
                ship_name,
                ship_class,
                sector_id,
                position,
                None, // No faction for squadron-owned ships
            );
            let ship_id = ship.id;

            // Store in state
            state.ships.insert(ship_id, ship);
            sector.ship_ids.insert(ship_id, ());
        }
    }

    let sector_name = sector.sector.name.clone();
    drop(sector);

    info!(
        building_type = ?building_type,
        sector_name = %sector_name,
        cost,
        squadron_name = %squadron_name,
        "Structure built"
    );

    SquadronResult {
        success: true,
        message: format!(
            "{} constructed in {}! Cost: {} reputation.",
            building_name, sector_name, cost
        ),
        squadron: None,
    }
}

// === PvP Rules ===

/// Check if two players can engage in PvP.
pub fn can_engage_pvp(state: &GameState, attacker_id: Uuid, target_id: Uuid) -> bool {
    let attacker = match state.player_data.get(&attacker_id) {
        Some(p) => p,
        None => return false,
    };

    let target = match state.player_data.get(&target_id) {
        Some(p) => p,
        None => return false,
    };

    let attacker_squadron_id = attacker.squadron_id;
    let target_squadron_id = target.squadron_id;
    drop(attacker);
    drop(target);

    match (attacker_squadron_id, target_squadron_id) {
        (Some(a_sq), Some(t_sq)) if a_sq == t_sq => {
            // Same squadron - check wargames
            state
                .squadrons
                .get(&a_sq)
                .map(|s| s.wargames_enabled)
                .unwrap_or(false)
        }
        (Some(a_sq), Some(t_sq)) => {
            // Different squadrons - check if at war or both have privateering
            let attacker_sq = state.squadrons.get(&a_sq);
            let target_sq = state.squadrons.get(&t_sq);

            match (attacker_sq, target_sq) {
                (Some(a), Some(t)) => {
                    // At war with each other
                    if a.is_hostile_with(t_sq) || t.is_hostile_with(a_sq) {
                        return true;
                    }

                    // Both have privateering enabled
                    if a.privateering_enabled && t.privateering_enabled {
                        return true;
                    }

                    false
                }
                _ => false,
            }
        }
        _ => {
            // One or both not in squadrons - no PvP (unless implementing dueling)
            false
        }
    }
}

/// Get PvP engagement type for combat rewards/penalties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvpEngagementType {
    /// Training within squadron (no real consequences)
    Wargames,
    /// Consensual PvP between privateering squadrons
    Privateering,
    /// Hostile engagement between squadrons at war
    War,
}

pub fn get_pvp_engagement_type(state: &GameState, attacker_id: Uuid, target_id: Uuid) -> Option<PvpEngagementType> {
    let attacker = state.player_data.get(&attacker_id)?;
    let target = state.player_data.get(&target_id)?;

    let a_sq = attacker.squadron_id?;
    let t_sq = target.squadron_id?;
    drop(attacker);
    drop(target);

    if a_sq == t_sq {
        return Some(PvpEngagementType::Wargames);
    }

    let attacker_sq = state.squadrons.get(&a_sq)?;
    let target_sq = state.squadrons.get(&t_sq)?;

    if attacker_sq.is_hostile_with(t_sq) || target_sq.is_hostile_with(a_sq) {
        return Some(PvpEngagementType::War);
    }

    if attacker_sq.privateering_enabled && target_sq.privateering_enabled {
        return Some(PvpEngagementType::Privateering);
    }

    None
}

// === Collective Bonuses ===

/// Get the collective bonus for a player based on their squadron.
pub fn get_squadron_bonuses(state: &GameState, player_id: Uuid) -> (f32, f32) {
    let player = match state.player_data.get(&player_id) {
        Some(p) => p,
        None => return (0.0, 0.0),
    };

    let squadron_id = match player.squadron_id {
        Some(id) => id,
        None => return (0.0, 0.0),
    };

    drop(player);

    let squadron = match state.squadrons.get(&squadron_id) {
        Some(s) => s,
        None => return (0.0, 0.0),
    };

    (squadron.reputation_bonus, squadron.fame_bonus)
}

/// Apply squadron bonus to a reputation gain.
pub fn apply_squadron_rep_bonus(state: &GameState, player_id: Uuid, base_rep: i32) -> i32 {
    let (rep_bonus, _) = get_squadron_bonuses(state, player_id);
    let multiplier = 1.0 + (rep_bonus / 100.0);
    (base_rep as f32 * multiplier).round() as i32
}

/// Apply squadron bonus to a fame gain.
pub fn apply_squadron_fame_bonus(state: &GameState, player_id: Uuid, base_fame: i32) -> i32 {
    let (_, fame_bonus) = get_squadron_bonuses(state, player_id);
    let multiplier = 1.0 + (fame_bonus / 100.0);
    (base_fame as f32 * multiplier).round() as i32
}

// === DTO Builder ===

fn build_squadron_dto(squadron: &Squadron, leader_name: &str, officer_names: Vec<String>) -> SquadronDto {
    SquadronDto {
        id: squadron.id,
        name: squadron.name.clone(),
        tag: squadron.tag.clone(),
        motto: squadron.motto.clone(),
        leader_name: leader_name.to_string(),
        officer_names,
        member_count: squadron.member_count() as u32,
        reputation_bonus: squadron.reputation_bonus,
        fame_bonus: squadron.fame_bonus,
        is_at_war: !squadron.hostile_squadrons.is_empty(),
    }
}

/// Get squadron details for a player.
pub fn get_squadron_info(state: &GameState, player_id: Uuid) -> Option<SquadronDto> {
    let player = state.player_data.get(&player_id)?;
    let squadron_id = player.squadron_id?;
    drop(player);

    let squadron = state.squadrons.get(&squadron_id)?;

    // Get leader name
    let leader_name = state
        .player_data
        .get(&squadron.leader_id)
        .map(|p| p.username.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Map officer UUIDs to usernames
    let officer_names: Vec<String> = squadron.officers.iter()
        .filter_map(|officer_id| {
            state.player_data.get(officer_id).map(|p| p.username.clone())
        })
        .collect();

    Some(build_squadron_dto(&squadron, &leader_name, officer_names))
}

/// Count the number of ships owned by a squadron in a given sector.
fn count_squadron_ships_in_sector(state: &GameState, squadron_id: Uuid, sector_id: Uuid) -> u32 {
    let sector = match state.sectors.get(&sector_id) {
        Some(s) => s,
        None => return 0,
    };

    let mut count = 0;
    for ship_id in sector.ship_ids.iter() {
        if let Some(ship) = state.ships.get(ship_id.key())
            && let Some(owner_id) = ship.owner_id
                && let Some(player) = state.player_data.get(&owner_id)
                    && player.squadron_id == Some(squadron_id) {
                        count += 1;
                    }
    }

    count
}
