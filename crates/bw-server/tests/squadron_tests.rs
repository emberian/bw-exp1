//! Integration tests for the squadron system

use std::sync::Arc;
use uuid::Uuid;

use bw_core::models::{Player, SquadronRank};
use bw_scripting::{
    BehaviorManager, CoroutineScheduler, EventRegistry, EventDispatcher,
    ActionRegistry, ActionDispatcher,
    debug::DebugController,
};
use bw_server::simulation::{
    create_squadron, invite_to_squadron, accept_squadron_invite, leave_squadron, process_squadron_action,
    can_engage_pvp, get_pvp_engagement_type, PvpEngagementType,
    get_squadron_bonuses, apply_squadron_rep_bonus, apply_squadron_fame_bonus,
};
use bw_server::GameState;
use bw_server::scripting::ScriptLogBuffer;
use bw_shared::SquadronAction;

/// Helper to create a test player with sufficient resources.
fn create_test_player(username: &str, reputation: i32) -> Player {
    let mut player = Player::new(
        username.to_string(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    player.resources.reputation = reputation;
    player
}

/// Helper to add a member to squadron (invite + accept).
fn add_member_to_squadron(state: &GameState, inviter_id: Uuid, invitee_id: Uuid) -> bool {
    let invite_result = invite_to_squadron(state, inviter_id, invitee_id);
    if !invite_result.success {
        return false;
    }
    let invite = invite_result.invite.unwrap();
    let accept_result = accept_squadron_invite(state, invitee_id, invite.id);
    accept_result.success
}

/// Helper to set up game state with test data.
async fn setup_test_state() -> GameState {
    // Create in-memory database for tests
    let db = bw_server::Database::new_in_memory().await.unwrap();
    let persist = db.spawn_persistence();
    let db = std::sync::Arc::new(db);
    let scripts = std::sync::Arc::new(bw_scripting::ScriptEngine::new("../../scripts"));
    let (broadcaster, _) = tokio::sync::broadcast::channel(1000);

    // Create scripting systems
    let event_registry = Arc::new(EventRegistry::new());
    let action_registry = Arc::new(ActionRegistry::new());
    let debug_controller = Arc::new(DebugController::new());
    let mut behavior_manager = BehaviorManager::new(scripts.clone());
    behavior_manager.set_debug_controller(debug_controller.clone());
    let coroutine_scheduler = CoroutineScheduler::new(scripts.clone());
    let event_dispatcher = EventDispatcher::new(event_registry.clone(), scripts.clone());
    let action_dispatcher = ActionDispatcher::new(action_registry.clone(), scripts.clone());

    let mut playtest_manager = bw_server::playtest::PlaytestManager::new(10);
    playtest_manager.set_debug_controller(debug_controller.clone());

    GameState {
        db,
        persist,
        scripts: scripts.clone(),
        sectors: dashmap::DashMap::new(),
        player_data: bw_server::persistence::TrackedDashMap::new(),
        players: dashmap::DashMap::new(),
        ships: bw_server::persistence::TrackedDashMap::new(),
        factions: dashmap::DashMap::new(),
        faction_tags: dashmap::DashMap::new(),
        squadrons: bw_server::persistence::TrackedDashMap::new(),
        pending_squadron_invites: dashmap::DashMap::new(),
        pending_alliances: dashmap::DashMap::new(),
        contested_sectors: dashmap::DashMap::new(),
        broadcaster,
        tick: std::sync::atomic::AtomicU64::new(0),
        // Scripting systems
        behavior_manager: parking_lot::RwLock::new(behavior_manager),
        coroutine_scheduler: parking_lot::RwLock::new(coroutine_scheduler),
        event_registry,
        event_dispatcher: parking_lot::RwLock::new(event_dispatcher),
        action_registry,
        action_dispatcher: parking_lot::RwLock::new(action_dispatcher),
        state_accessor: parking_lot::RwLock::new(None),
        script_logs: parking_lot::RwLock::new(ScriptLogBuffer::new(100)),
        metrics: bw_server::simulation::metrics::MetricsStore::new(),
        playtest_manager,
        debug_controller,
    }
}

// ============== Squadron Creation Tests ==============

#[tokio::test]
async fn test_create_squadron_success() {
    let state = setup_test_state().await;

    // Create player with enough reputation
    let player = create_test_player("TestPlayer", 100);
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let result = create_squadron(&state, player_id, "Test Squadron".to_string(), "TST".to_string());

    assert!(result.success, "Squadron creation should succeed: {}", result.message);
    assert!(result.squadron.is_some());

    let dto = result.squadron.unwrap();
    assert_eq!(dto.name, "Test Squadron");
    assert_eq!(dto.tag, "TST");
    assert_eq!(dto.member_count, 1);

    // Check player was updated
    let player = state.player_data.get(&player_id).unwrap();
    assert!(player.squadron_id.is_some());
    assert_eq!(player.squadron_rank, Some(SquadronRank::Leader));
    assert_eq!(player.resources.reputation, 50); // 100 - 50 cost
}

#[tokio::test]
async fn test_create_squadron_invalid_tag_too_short() {
    let state = setup_test_state().await;

    let player = create_test_player("TestPlayer", 100);
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let result = create_squadron(&state, player_id, "Test Squadron".to_string(), "T".to_string());

    assert!(!result.success);
    assert!(result.message.contains("2-5 uppercase"));
}

#[tokio::test]
async fn test_create_squadron_invalid_tag_lowercase() {
    let state = setup_test_state().await;

    let player = create_test_player("TestPlayer", 100);
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let result = create_squadron(&state, player_id, "Test Squadron".to_string(), "abc".to_string());

    assert!(!result.success);
    assert!(result.message.contains("uppercase"));
}

#[tokio::test]
async fn test_create_squadron_invalid_name_too_short() {
    let state = setup_test_state().await;

    let player = create_test_player("TestPlayer", 100);
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let result = create_squadron(&state, player_id, "AB".to_string(), "TST".to_string());

    assert!(!result.success);
    assert!(result.message.contains("3-32 characters"));
}

#[tokio::test]
async fn test_create_squadron_insufficient_reputation() {
    let state = setup_test_state().await;

    // Player with only 30 reputation (need 50)
    let player = create_test_player("TestPlayer", 30);
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let result = create_squadron(&state, player_id, "Test Squadron".to_string(), "TST".to_string());

    assert!(!result.success);
    assert!(result.message.contains("Insufficient reputation"));
}

#[tokio::test]
async fn test_create_squadron_already_in_squadron() {
    let state = setup_test_state().await;

    let mut player = create_test_player("TestPlayer", 200);
    player.squadron_id = Some(Uuid::new_v4()); // Already in a squadron
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let result = create_squadron(&state, player_id, "Test Squadron".to_string(), "TST".to_string());

    assert!(!result.success);
    assert!(result.message.contains("already in a squadron"));
}

#[tokio::test]
async fn test_create_squadron_duplicate_tag() {
    let state = setup_test_state().await;

    // Create first squadron
    let player1 = create_test_player("Player1", 100);
    let player1_id = player1.id;
    state.player_data.insert(player1_id, player1);

    let result = create_squadron(&state, player1_id, "First Squadron".to_string(), "DUP".to_string());
    assert!(result.success);

    // Try to create second squadron with same tag
    let player2 = create_test_player("Player2", 100);
    let player2_id = player2.id;
    state.player_data.insert(player2_id, player2);

    let result = create_squadron(&state, player2_id, "Second Squadron".to_string(), "DUP".to_string());

    assert!(!result.success);
    assert!(result.message.contains("already in use"));
}

// ============== Squadron Membership Tests ==============

#[tokio::test]
async fn test_invite_to_squadron() {
    let state = setup_test_state().await;

    // Create leader and squadron
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    let result = create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());
    assert!(result.success);

    // Create invitee
    let invitee = create_test_player("Invitee", 50);
    let invitee_id = invitee.id;
    state.player_data.insert(invitee_id, invitee);

    // Invite creates pending invite
    let result = invite_to_squadron(&state, leader_id, invitee_id);
    assert!(result.success, "Invite should succeed: {}", result.message);
    assert!(result.invite.is_some());
    let invite = result.invite.unwrap();

    // Invitee should NOT be added yet (pending invite)
    let invitee = state.player_data.get(&invitee_id).unwrap();
    assert!(invitee.squadron_id.is_none(), "Should not be added until invite accepted");
    drop(invitee);

    // Accept the invite
    let accept_result = accept_squadron_invite(&state, invitee_id, invite.id);
    assert!(accept_result.success, "Accept should succeed: {}", accept_result.message);

    // Now invitee should be added
    let invitee = state.player_data.get(&invitee_id).unwrap();
    assert!(invitee.squadron_id.is_some());
    assert_eq!(invitee.squadron_rank, Some(SquadronRank::Member));
}

#[tokio::test]
async fn test_invite_player_already_in_squadron() {
    let state = setup_test_state().await;

    // Create leader and squadron
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    // Create player already in another squadron
    let mut other_player = create_test_player("Other", 50);
    other_player.squadron_id = Some(Uuid::new_v4());
    let other_id = other_player.id;
    state.player_data.insert(other_id, other_player);

    let result = invite_to_squadron(&state, leader_id, other_id);

    assert!(!result.success);
    assert!(result.message.contains("already in a squadron"));
}

#[tokio::test]
async fn test_leave_squadron_member() {
    let state = setup_test_state().await;

    // Create squadron with two members
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);

    // Use helper to invite + accept
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Member leaves
    let result = leave_squadron(&state, member_id);

    assert!(result.success, "Leave should succeed: {}", result.message);

    let member = state.player_data.get(&member_id).unwrap();
    assert!(member.squadron_id.is_none());
    assert!(member.squadron_rank.is_none());
}

#[tokio::test]
async fn test_leave_squadron_leader_with_members() {
    let state = setup_test_state().await;

    // Create squadron with two members
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);

    // Use helper to invite + accept
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Leader tries to leave
    let result = leave_squadron(&state, leader_id);

    assert!(!result.success);
    assert!(result.message.contains("transfer leadership"));
}

#[tokio::test]
async fn test_leave_squadron_leader_disbands() {
    let state = setup_test_state().await;

    // Create squadron with only leader
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    let result = create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());
    let squadron_id = result.squadron.unwrap().id;

    // Leader leaves - should disband
    let result = leave_squadron(&state, leader_id);

    assert!(result.success);
    assert!(result.message.contains("disbanded"));

    // Squadron should no longer exist
    assert!(!state.squadrons.contains_key(&squadron_id));
}

// ============== Squadron Actions Tests ==============

#[tokio::test]
async fn test_promote_member_to_officer() {
    let state = setup_test_state().await;

    // Setup
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);

    // Use helper to invite + accept
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Promote
    let action = SquadronAction::PromoteToOfficer { player_id: member_id };
    let result = process_squadron_action(&state, leader_id, action);

    assert!(result.success, "Promote should succeed: {}", result.message);

    let member = state.player_data.get(&member_id).unwrap();
    assert_eq!(member.squadron_rank, Some(SquadronRank::Officer));
}

#[tokio::test]
async fn test_demote_officer() {
    let state = setup_test_state().await;

    // Setup
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);

    // Use helper to invite + accept
    assert!(add_member_to_squadron(&state, leader_id, member_id));
    process_squadron_action(&state, leader_id, SquadronAction::PromoteToOfficer { player_id: member_id });

    // Demote
    let action = SquadronAction::DemoteOfficer { player_id: member_id };
    let result = process_squadron_action(&state, leader_id, action);

    assert!(result.success);

    let member = state.player_data.get(&member_id).unwrap();
    assert_eq!(member.squadron_rank, Some(SquadronRank::Member));
}

#[tokio::test]
async fn test_kick_member() {
    let state = setup_test_state().await;

    // Setup
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);

    // Use helper to invite + accept
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Kick
    let action = SquadronAction::KickMember { player_id: member_id };
    let result = process_squadron_action(&state, leader_id, action);

    assert!(result.success);

    let member = state.player_data.get(&member_id).unwrap();
    assert!(member.squadron_id.is_none());
}

#[tokio::test]
async fn test_transfer_leadership() {
    let state = setup_test_state().await;

    // Setup
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);

    // Use helper to invite + accept
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Transfer
    let action = SquadronAction::TransferLeadership { player_id: member_id };
    let result = process_squadron_action(&state, leader_id, action);

    assert!(result.success);

    let old_leader = state.player_data.get(&leader_id).unwrap();
    let new_leader = state.player_data.get(&member_id).unwrap();

    assert_eq!(old_leader.squadron_rank, Some(SquadronRank::Officer));
    assert_eq!(new_leader.squadron_rank, Some(SquadronRank::Leader));
}

#[tokio::test]
async fn test_set_motto() {
    let state = setup_test_state().await;

    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    let result = create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());
    let squadron_id = result.squadron.unwrap().id;

    let action = SquadronAction::SetMotto { motto: "For glory!".to_string() };
    let result = process_squadron_action(&state, leader_id, action);

    assert!(result.success);

    let squadron = state.squadrons.get(&squadron_id).unwrap();
    assert_eq!(squadron.motto, Some("For glory!".to_string()));
}

// ============== PvP Rules Tests ==============

#[tokio::test]
async fn test_pvp_wargames_same_squadron() {
    let state = setup_test_state().await;

    // Create squadron with wargames enabled
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    let result = create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());
    let _squadron_id = result.squadron.unwrap().id;

    // Enable wargames
    process_squadron_action(&state, leader_id, SquadronAction::EnableWargames { enabled: true });

    // Add second member using helper
    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Should be able to engage in PvP
    assert!(can_engage_pvp(&state, leader_id, member_id));
    assert_eq!(
        get_pvp_engagement_type(&state, leader_id, member_id),
        Some(PvpEngagementType::Wargames)
    );
}

#[tokio::test]
async fn test_pvp_no_wargames() {
    let state = setup_test_state().await;

    // Create squadron without wargames
    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // Should NOT be able to engage in PvP
    assert!(!can_engage_pvp(&state, leader_id, member_id));
}

#[tokio::test]
async fn test_pvp_war_between_squadrons() {
    let state = setup_test_state().await;

    // Create first squadron
    let leader1 = create_test_player("Leader1", 100);
    let leader1_id = leader1.id;
    state.player_data.insert(leader1_id, leader1);

    let result1 = create_squadron(&state, leader1_id, "Squadron One".to_string(), "ONE".to_string());
    let sq1_id = result1.squadron.unwrap().id;

    // Create second squadron
    let leader2 = create_test_player("Leader2", 100);
    let leader2_id = leader2.id;
    state.player_data.insert(leader2_id, leader2);

    let result2 = create_squadron(&state, leader2_id, "Squadron Two".to_string(), "TWO".to_string());
    let sq2_id = result2.squadron.unwrap().id;

    // No PvP before war
    assert!(!can_engage_pvp(&state, leader1_id, leader2_id));

    // Declare war
    process_squadron_action(&state, leader1_id, SquadronAction::DeclareWar { squadron_id: sq2_id });

    // Now can engage
    assert!(can_engage_pvp(&state, leader1_id, leader2_id));
    assert_eq!(
        get_pvp_engagement_type(&state, leader1_id, leader2_id),
        Some(PvpEngagementType::War)
    );
}

#[tokio::test]
async fn test_pvp_privateering() {
    let state = setup_test_state().await;

    // Create first squadron with privateering
    let leader1 = create_test_player("Leader1", 100);
    let leader1_id = leader1.id;
    state.player_data.insert(leader1_id, leader1);

    create_squadron(&state, leader1_id, "Squadron One".to_string(), "ONE".to_string());
    process_squadron_action(&state, leader1_id, SquadronAction::EnablePrivateering { enabled: true });

    // Create second squadron with privateering
    let leader2 = create_test_player("Leader2", 100);
    let leader2_id = leader2.id;
    state.player_data.insert(leader2_id, leader2);

    create_squadron(&state, leader2_id, "Squadron Two".to_string(), "TWO".to_string());
    process_squadron_action(&state, leader2_id, SquadronAction::EnablePrivateering { enabled: true });

    // Can engage through privateering
    assert!(can_engage_pvp(&state, leader1_id, leader2_id));
    assert_eq!(
        get_pvp_engagement_type(&state, leader1_id, leader2_id),
        Some(PvpEngagementType::Privateering)
    );
}

// ============== Squadron Bonuses Tests ==============

#[tokio::test]
async fn test_squadron_bonuses() {
    let state = setup_test_state().await;

    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    // Add a member to trigger bonus recalculation
    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    let (rep_bonus, fame_bonus) = get_squadron_bonuses(&state, leader_id);

    // With 2 members: sqrt(2) * 2 ≈ 2.83, sqrt(2) * 1.5 ≈ 2.12
    assert!((rep_bonus - 2.83).abs() < 0.1, "rep_bonus was {}", rep_bonus);
    assert!((fame_bonus - 2.12).abs() < 0.1, "fame_bonus was {}", fame_bonus);
}

#[tokio::test]
async fn test_apply_squadron_bonus() {
    let state = setup_test_state().await;

    let leader = create_test_player("Leader", 100);
    let leader_id = leader.id;
    state.player_data.insert(leader_id, leader);

    create_squadron(&state, leader_id, "Test Squadron".to_string(), "TST".to_string());

    // Add a member to trigger bonus recalculation
    let member = create_test_player("Member", 50);
    let member_id = member.id;
    state.player_data.insert(member_id, member);
    assert!(add_member_to_squadron(&state, leader_id, member_id));

    // With 2 members: sqrt(2) * 2 ≈ 2.83% bonus
    // Base 100 rep with ~2.83% bonus ≈ 103
    let boosted = apply_squadron_rep_bonus(&state, leader_id, 100);
    assert!(boosted >= 102 && boosted <= 103, "boosted rep was {}", boosted);

    // With 2 members: sqrt(2) * 1.5 ≈ 2.12% bonus
    // Base 100 fame with ~2.12% bonus ≈ 102
    let boosted = apply_squadron_fame_bonus(&state, leader_id, 100);
    assert!(boosted >= 102 && boosted <= 103, "boosted fame was {}", boosted);
}

#[tokio::test]
async fn test_no_bonus_without_squadron() {
    let state = setup_test_state().await;

    let player = create_test_player("Solo", 100);
    let player_id = player.id;
    state.player_data.insert(player_id, player);

    let (rep_bonus, fame_bonus) = get_squadron_bonuses(&state, player_id);

    assert_eq!(rep_bonus, 0.0);
    assert_eq!(fame_bonus, 0.0);

    // No bonus applied
    let boosted = apply_squadron_rep_bonus(&state, player_id, 100);
    assert_eq!(boosted, 100);
}
