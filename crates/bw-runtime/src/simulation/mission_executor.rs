//! Mission Execution System
//!
//! Executes Rhai mission scripts and processes player choices.

use uuid::Uuid;
use tracing::{debug, info, instrument, trace, warn};

use bw_core::models::{Mission, MissionStatus, Ship};
use bw_scripting::{MissionContext, MissionRunner};
use bw_shared::dto::ChoiceDto;
use bw_shared::ServerMessage;

use crate::GameState;

/// Result of mission execution.
#[derive(Debug)]
pub struct MissionExecutionResult {
    /// Narrative text to show player
    pub narrative: String,
    /// Choices available
    pub choices: Vec<ChoiceDto>,
    /// Resource changes to apply
    pub reputation_change: i32,
    pub fame_change: i32,
    pub ammo_change: f32,
    pub fuel_change: f32,
    pub morale_change: f32,
    pub xp_change: i32,
    /// Whether combat should start
    pub spawn_combat: Option<SpawnCombat>,
    /// Whether mission completed
    pub is_complete: bool,
    /// Success/failure
    pub success: bool,
}

/// Combat to spawn as result of mission.
#[derive(Debug)]
pub struct SpawnCombat {
    pub enemy_type: String,
    pub enemy_name: String,
    pub enemy_count: u32,
}

impl Default for MissionExecutionResult {
    fn default() -> Self {
        Self {
            narrative: String::new(),
            choices: Vec::new(),
            reputation_change: 0,
            fame_change: 0,
            ammo_change: 0.0,
            fuel_change: 0.0,
            morale_change: 0.0,
            xp_change: 0,
            spawn_combat: None,
            is_complete: false,
            success: false,
        }
    }
}

/// Start a mission for a player.
#[instrument(skip(state), fields(player_id = %player_id, mission_id = %mission_id))]
pub fn start_mission(
    state: &GameState,
    player_id: Uuid,
    mission_id: Uuid,
) -> Result<MissionExecutionResult, String> {
    // Get player session
    let session = state
        .players
        .get(&player_id)
        .ok_or("Player not found")?;
    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session);

    // Get ship
    let ship = state
        .ships
        .get(&ship_id)
        .ok_or("Ship not found")?;

    // Get sector and mission
    let sector = state
        .sectors
        .get(&sector_id)
        .ok_or("Sector not found")?;

    let mut mission = sector
        .missions
        .get_mut(&mission_id)
        .ok_or("Mission not found")?;

    // Check if can accept
    if !mission.can_accept(player_id, None) {
        debug!(mission_title = %mission.title, "Player cannot accept mission");
        return Err("Cannot accept this mission".to_string());
    }

    // Check distance to mission target (if mission has a target position)
    // Missions can be accepted from anywhere in the sector, but some may require proximity
    if let Some(target_pos) = &mission.target_position {
        let distance = ship.position.distance_to(target_pos);
        // Allow accepting from reasonable distance (500 units) - can approach later
        if distance > 500.0 {
            debug!(distance, "Player too far from mission area");
            return Err(format!(
                "Too far from mission area (distance: {:.0}, need < 500)",
                distance
            ));
        }
    }

    // Build mission context
    let ctx = build_mission_context(state, &mission, &ship, player_id);

    // Run mission script
    let runner = MissionRunner::new(&state.scripts);
    let outcome = runner.start_mission(&ctx).map_err(|e| {
        warn!(error = %e, "Mission script failed to start");
        e.to_string()
    })?;

    info!(
        mission_title = %mission.title,
        mission_type = ?mission.mission_type,
        "Mission started"
    );

    // Update mission state
    mission.status = MissionStatus::InProgress;
    mission.accept(player_id);
    mission.current_state = outcome.new_state.to_string();

    // Build result
    let mut result = MissionExecutionResult::default();
    result.narrative = outcome.narrative.clone();
    result.choices = outcome
        .choices
        .iter()
        .map(|c| ChoiceDto {
            id: c.id.clone(),
            text: c.text.clone(),
            is_available: c.is_available(&ctx),
            requirement_text: c.requirements.first().map(|r| r.description()),
        })
        .collect();

    result.reputation_change = outcome.resource_changes.reputation;
    result.fame_change = outcome.resource_changes.fame;
    result.ammo_change = outcome.resource_changes.ammunition;
    result.fuel_change = outcome.resource_changes.fuel;
    result.morale_change = outcome.resource_changes.morale;
    result.xp_change = outcome.resource_changes.experience;

    result.is_complete = outcome.is_complete;
    result.success = outcome.success;

    // Check for combat spawn
    if let Some(ref combat) = outcome.spawn_combat {
        result.spawn_combat = Some(SpawnCombat {
            enemy_type: combat.enemy_type.clone(),
            enemy_name: format!("{} {}", combat.enemy_type, "raiders"),
            enemy_count: combat.enemy_count,
        });
    }

    Ok(result)
}

/// Process a player's mission choice.
#[instrument(skip(state), fields(player_id = %player_id, mission_id = %mission_id, choice_id = %choice_id))]
pub fn process_mission_choice(
    state: &GameState,
    player_id: Uuid,
    mission_id: Uuid,
    choice_id: &str,
) -> Result<MissionExecutionResult, String> {
    // Get player session
    let session = state
        .players
        .get(&player_id)
        .ok_or("Player not found")?;
    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session);

    // Get ship
    let ship = state
        .ships
        .get(&ship_id)
        .ok_or("Ship not found")?;

    // Get sector and mission
    let sector = state
        .sectors
        .get(&sector_id)
        .ok_or("Sector not found")?;

    let mut mission = sector
        .missions
        .get_mut(&mission_id)
        .ok_or("Mission not found")?;

    // Check if player is assigned
    if mission.assigned_to != Some(player_id) {
        debug!("Player not assigned to this mission");
        return Err("Not assigned to this mission".to_string());
    }

    let mission_title = mission.title.clone();

    // Build mission context
    let ctx = build_mission_context(state, &mission, &ship, player_id);

    // Run choice handler
    let runner = MissionRunner::new(&state.scripts);
    let outcome = runner
        .process_choice(&ctx, choice_id)
        .map_err(|e| {
            warn!(error = %e, "Mission choice processing failed");
            e.to_string()
        })?;

    debug!(
        mission_title = %mission_title,
        choice_id,
        new_state = %outcome.new_state,
        "Mission choice processed"
    );

    // Update mission state
    mission.current_state = outcome.new_state.to_string();
    mission.progress = 0.5; // Midway through

    if outcome.is_complete {
        mission.status = MissionStatus::Completed { success: outcome.success };
        mission.progress = 1.0;
        info!(
            mission_title = %mission_title,
            success = outcome.success,
            "Mission completed"
        );
    }

    // Build result
    let mut result = MissionExecutionResult::default();
    result.narrative = outcome.narrative.clone();
    result.choices = outcome
        .choices
        .iter()
        .map(|c| ChoiceDto {
            id: c.id.clone(),
            text: c.text.clone(),
            is_available: c.is_available(&ctx),
            requirement_text: c.requirements.first().map(|r| r.description()),
        })
        .collect();

    result.reputation_change = outcome.resource_changes.reputation;
    result.fame_change = outcome.resource_changes.fame;
    result.ammo_change = outcome.resource_changes.ammunition;
    result.fuel_change = outcome.resource_changes.fuel;
    result.morale_change = outcome.resource_changes.morale;
    result.xp_change = outcome.resource_changes.experience;

    result.is_complete = outcome.is_complete;
    result.success = outcome.success;

    if let Some(ref combat) = outcome.spawn_combat {
        result.spawn_combat = Some(SpawnCombat {
            enemy_type: combat.enemy_type.clone(),
            enemy_name: format!("{} {}", combat.enemy_type, "raiders"),
            enemy_count: combat.enemy_count,
        });
    }

    Ok(result)
}

/// Process combat result for a mission.
#[instrument(skip(state), fields(player_id = %player_id, mission_id = %mission_id, player_won, enemy_fled))]
pub fn process_mission_combat_result(
    state: &GameState,
    player_id: Uuid,
    mission_id: Uuid,
    player_won: bool,
    enemy_fled: bool,
) -> Result<MissionExecutionResult, String> {
    debug!(player_won, enemy_fled, "Processing mission combat result");

    // Get player session
    let session = state
        .players
        .get(&player_id)
        .ok_or("Player not found")?;
    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session);

    // Get ship
    let ship = state
        .ships
        .get(&ship_id)
        .ok_or("Ship not found")?;

    // Get sector and mission
    let sector = state
        .sectors
        .get(&sector_id)
        .ok_or("Sector not found")?;

    let mut mission = sector
        .missions
        .get_mut(&mission_id)
        .ok_or("Mission not found")?;

    let mission_title = mission.title.clone();

    // Build mission context
    let ctx = build_mission_context(state, &mission, &ship, player_id);

    // Run combat result handler
    let runner = MissionRunner::new(&state.scripts);
    let outcome = runner
        .process_combat_result(&ctx, player_won, enemy_fled)
        .map_err(|e| {
            warn!(error = %e, "Mission combat result processing failed");
            e.to_string()
        })?;

    // Update mission state
    mission.current_state = outcome.new_state.to_string();

    if outcome.is_complete {
        mission.status = MissionStatus::Completed { success: outcome.success };
        mission.progress = 1.0;
        info!(
            mission_title = %mission_title,
            success = outcome.success,
            player_won,
            "Mission completed via combat"
        );
    }

    // Build result
    let mut result = MissionExecutionResult::default();
    result.narrative = outcome.narrative.clone();
    result.choices = outcome
        .choices
        .iter()
        .map(|c| ChoiceDto {
            id: c.id.clone(),
            text: c.text.clone(),
            is_available: c.is_available(&ctx),
            requirement_text: c.requirements.first().map(|r| r.description()),
        })
        .collect();

    result.reputation_change = outcome.resource_changes.reputation;
    result.fame_change = outcome.resource_changes.fame;
    result.ammo_change = outcome.resource_changes.ammunition;
    result.fuel_change = outcome.resource_changes.fuel;
    result.morale_change = outcome.resource_changes.morale;
    result.xp_change = outcome.resource_changes.experience;

    result.is_complete = outcome.is_complete;
    result.success = outcome.success;

    Ok(result)
}

/// Build a mission context from game state.
fn build_mission_context(
    state: &GameState,
    mission: &Mission,
    ship: &Ship,
    player_id: Uuid,
) -> MissionContext {
    // Get player reputation and fame from player_data
    let (player_reputation, player_fame) = state.player_data.get(&player_id)
        .map(|p| (p.resources.reputation, p.resources.fame))
        .unwrap_or((100, 0));

    MissionContext {
        mission_id: mission.id,
        mission_type: format!("{:?}", mission.mission_type).to_lowercase(),
        script_path: mission.script_path.clone(),
        current_state: mission.current_state.clone(),
        data: Default::default(),
        player_id,
        player_reputation,
        player_fame,
        ship_id: ship.id,
        ship_hull: ship.hull_integrity,
        ship_ammo: ship.resources.ammunition,
        ship_fuel: ship.resources.fuel,
        crew_morale: ship.crew.morale,
        crew_experience: ship.crew.experience,
        sector_id: ship.sector_id,
    }
}

/// Apply resource changes to a player's ship and player model.
#[instrument(skip(state, result), fields(
    player_id = %player_id,
    rep_change = result.reputation_change,
    fame_change = result.fame_change,
))]
pub fn apply_resource_changes(
    state: &GameState,
    player_id: Uuid,
    result: &MissionExecutionResult,
) -> Option<ServerMessage> {
    use bw_game::systems::{apply_reputation_change, apply_fame_change};

    let session = state.players.get(&player_id)?;
    let ship_id = session.ship_id;
    drop(session);

    let mut ship = state.ships.get_mut(&ship_id)?;

    // Apply ship resource changes
    ship.resources.ammunition = (ship.resources.ammunition + result.ammo_change).clamp(0.0, 100.0);
    ship.resources.fuel = (ship.resources.fuel + result.fuel_change).clamp(0.0, 100.0);
    ship.crew.morale = (ship.crew.morale + result.morale_change).clamp(0.0, 100.0);
    ship.crew.experience = (ship.crew.experience + result.xp_change).max(0);

    let ammunition = ship.resources.ammunition;
    let fuel = ship.resources.fuel;
    let morale = ship.crew.morale;
    let experience = ship.crew.experience;
    drop(ship);

    // Apply reputation/fame changes to player model (with fame multiplier)
    let (final_reputation, final_fame) = if let Some(mut player) = state.player_data.get_mut(&player_id) {
        // Apply reputation with fame multiplier
        let _actual_rep_change = apply_reputation_change(&mut player.resources, result.reputation_change);
        // Apply fame change directly
        apply_fame_change(&mut player.resources, result.fame_change);

        (player.resources.reputation, player.resources.fame)
    } else {
        (100, 0)
    };

    trace!(
        reputation = final_reputation,
        fame = final_fame,
        ammo = ammunition,
        fuel = fuel,
        morale = morale,
        xp = experience,
        "Resource changes applied"
    );

    // Build resource update message
    Some(ServerMessage::ResourceUpdate {
        reputation: final_reputation,
        fame: final_fame,
        ammunition,
        fuel,
        morale,
        experience,
    })
}
