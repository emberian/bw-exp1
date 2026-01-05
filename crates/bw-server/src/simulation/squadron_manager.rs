//! Squadron Management System
//!
//! Handles squadron CRUD, membership, sector control, building, and PvP rules.

use uuid::Uuid;

use bw_core::models::{Squadron, SquadronBuildingType, SquadronRank};
use bw_core::systems::spend_reputation;
use bw_shared::dto::SquadronDto;

use crate::GameState;

/// Result of a squadron operation.
#[derive(Debug)]
pub struct SquadronResult {
    pub success: bool,
    pub message: String,
    pub squadron: Option<SquadronDto>,
}

/// Create a new squadron.
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

    // Build DTO before inserting
    let dto = build_squadron_dto(&squadron, &leader_name);

    // Store squadron
    state.squadrons.insert(squadron_id, squadron);

    tracing::info!(
        "Squadron '{}' [{}] created by player {}",
        name,
        tag,
        player_id
    );

    SquadronResult {
        success: true,
        message: format!("Squadron '{}' [{}] created successfully!", name, tag),
        squadron: Some(dto),
    }
}

/// Invite a player to the squadron.
pub fn invite_to_squadron(
    state: &GameState,
    inviter_id: Uuid,
    invitee_id: Uuid,
) -> SquadronResult {
    // Get inviter's squadron
    let inviter = match state.player_data.get(&inviter_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Player not found".to_string(),
                squadron: None,
            }
        }
    };

    let squadron_id = match inviter.squadron_id {
        Some(id) => id,
        None => {
            return SquadronResult {
                success: false,
                message: "You are not in a squadron".to_string(),
                squadron: None,
            }
        }
    };

    drop(inviter);

    // Get squadron
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

    // Check if inviter can invite (leader, officer, or members_can_invite)
    let can_invite = squadron.can_manage(inviter_id) || squadron.settings.members_can_invite;
    if !can_invite {
        return SquadronResult {
            success: false,
            message: "You don't have permission to invite".to_string(),
            squadron: None,
        };
    }

    drop(squadron);

    // Check invitee
    let mut invitee = match state.player_data.get_mut(&invitee_id) {
        Some(p) => p,
        None => {
            return SquadronResult {
                success: false,
                message: "Target player not found".to_string(),
                squadron: None,
            }
        }
    };

    if invitee.squadron_id.is_some() {
        return SquadronResult {
            success: false,
            message: "Target player is already in a squadron".to_string(),
            squadron: None,
        };
    }

    // For now, auto-accept (TODO: implement invitation system)
    invitee.squadron_id = Some(squadron_id);
    invitee.squadron_rank = Some(SquadronRank::Member);
    let invitee_name = invitee.username.clone();
    drop(invitee);

    // Add to squadron
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

    squadron.add_member(invitee_id);
    let squadron_name = squadron.name.clone();

    tracing::info!(
        "Player {} joined squadron '{}'",
        invitee_name,
        squadron_name
    );

    SquadronResult {
        success: true,
        message: format!("{} has joined the squadron!", invitee_name),
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

            tracing::info!("Squadron '{}' disbanded", squadron_name);

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
    tracing::info!(
        "Squadron {} declared war on {}",
        squadron_id,
        target_squadron_id
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

    tracing::info!(
        "Squadron {} made peace with {}",
        squadron_id,
        target_squadron_id
    );

    SquadronResult {
        success: true,
        message: "Peace declared. You can no longer engage their members.".to_string(),
        squadron: None,
    }
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
            message: "Only the leader can form alliances".to_string(),
            squadron: None,
        };
    }

    if !state.squadrons.contains_key(&target_squadron_id) {
        return SquadronResult {
            success: false,
            message: "Target squadron not found".to_string(),
            squadron: None,
        };
    }

    // For now, one-sided alliance offer (TODO: implement acceptance)
    squadron.form_alliance(target_squadron_id);

    tracing::info!(
        "Squadron {} formed alliance with {}",
        squadron_id,
        target_squadron_id
    );

    SquadronResult {
        success: true,
        message: "Alliance formed! Allied squadrons share sector bonuses.".to_string(),
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

        // TODO: Implement contested sector mechanics
        return SquadronResult {
            success: false,
            message: "Contested sector claim not yet implemented".to_string(),
            squadron: None,
        };
    }

    // Claim uncontrolled sector
    sector.sector.controlling_squadron = Some(squadron_id);
    let sector_name = sector.sector.name.clone();
    drop(sector);

    // Update squadron
    if let Some(mut squadron) = state.squadrons.get_mut(&squadron_id) {
        if !squadron.patrol_sectors.contains(&sector_id) {
            squadron.patrol_sectors.push(sector_id);
            squadron.stats.sectors_controlled = squadron.patrol_sectors.len() as i32;
        }
    }

    tracing::info!("Squadron {} claimed sector '{}'", squadron_id, sector_name);

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

    tracing::info!("Squadron {} released sector '{}'", squadron_id, sector_name);

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

    // TODO: Actually create the station/ship entity
    // For now, just log and track in squadron

    let building_name = format!("{:?}", building_type);
    tracing::info!(
        "Player {} built {:?} in sector {} for {} rep",
        player_id,
        building_type,
        sector_id,
        cost
    );

    SquadronResult {
        success: true,
        message: format!(
            "Construction of {} initiated! Cost: {} reputation.",
            building_name, cost
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

fn build_squadron_dto(squadron: &Squadron, leader_name: &str) -> SquadronDto {
    SquadronDto {
        id: squadron.id,
        name: squadron.name.clone(),
        tag: squadron.tag.clone(),
        motto: squadron.motto.clone(),
        leader_name: leader_name.to_string(),
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

    Some(build_squadron_dto(&squadron, &leader_name))
}
