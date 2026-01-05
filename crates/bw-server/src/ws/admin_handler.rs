//! Admin/GM WebSocket message handler
//!
//! Handles privileged operations for Game Masters.

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use bw_shared::{
    AdminClientMessage, AdminServerMessage, EntityFilter, EntitySummary, EntityType,
    ScriptFileInfo, SectorSummaryAdmin, ServerMessage, StagedChange, ChangePreview,
    ValidationError, SimConfigSection, ScriptErrorDto,
    ForkConfigDto, PromoteConfigDto, PlaytestSummaryDto, PlaytestDetailDto, PlaytestParticipantDto,
    dto::{
        BreakpointDto, FunctionBreakpointDto, StackFrameDto, VariableDto, PauseReasonDto, DebugTargetDto,
        ArchetypeSchemaDto, FieldSchemaDto, ActionSchemaDto, ParamSchemaDto,
        ValidationIssueDto, ValidationSeverity, WatchDto,
    },
};
use bw_scripting::debug::{Breakpoint, DebugTarget, DebugCommand, PauseReason};
use bw_scripting::schema::{
    DefinitionSchemaRegistry, DefFieldType,
    SHIP_SCHEMA, WEAPON_SCHEMA, EFFECT_SCHEMA, CARGO_SCHEMA, ABILITY_SCHEMA, FACTION_SCHEMA,
    ActionSchemaRegistry,
};
use crate::{config::config, GameState};

/// Handle an admin message from a connected client.
///
/// Returns None if the user is not authorized for admin operations.
pub async fn handle_admin_message(
    state: &Arc<GameState>,
    tx: &mpsc::Sender<ServerMessage>,
    _player_id: Uuid,
    username: &str,
    msg: AdminClientMessage,
) -> bool {
    // Verify admin status
    if !config().is_admin(username) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Admin privileges required".to_string(),
        })).await;
        return false;
    }

    match msg {
        AdminClientMessage::ListScripts => {
            handle_list_scripts(tx).await;
        }

        AdminClientMessage::ReadScript { path } => {
            handle_read_script(tx, &path).await;
        }

        AdminClientMessage::WriteScript { path, content } => {
            handle_write_script(tx, &path, &content).await;
        }

        AdminClientMessage::GetSimConfig => {
            handle_get_sim_config(tx).await;
        }

        AdminClientMessage::QueryEntities { entity_type, filters, limit, offset } => {
            handle_query_entities(state, tx, entity_type, filters, limit, offset).await;
        }

        AdminClientMessage::GetEntity { entity_type, id } => {
            handle_get_entity(state, tx, entity_type, id).await;
        }

        AdminClientMessage::QuerySectors => {
            handle_query_sectors(state, tx).await;
        }

        AdminClientMessage::PreviewStaged { changes } => {
            handle_preview_staged(state, tx, &changes).await;
        }

        AdminClientMessage::CommitStaged { changes } => {
            handle_commit_staged(state, tx, username, changes).await;
        }

        // === Playtest Management ===

        AdminClientMessage::CreatePlaytest { name, fork_config } => {
            handle_create_playtest(state, tx, _player_id, name, fork_config).await;
        }

        AdminClientMessage::ListPlaytests => {
            handle_list_playtests(state, tx).await;
        }

        AdminClientMessage::GetPlaytest { playtest_id } => {
            handle_get_playtest(state, tx, playtest_id).await;
        }

        AdminClientMessage::JoinPlaytest { playtest_id } => {
            handle_join_playtest(state, tx, _player_id, playtest_id).await;
        }

        AdminClientMessage::LeavePlaytest => {
            handle_leave_playtest(state, tx, _player_id).await;
        }

        AdminClientMessage::InviteToPlaytest { playtest_id, player_id } => {
            handle_invite_to_playtest(state, tx, _player_id, playtest_id, player_id).await;
        }

        AdminClientMessage::KickFromPlaytest { playtest_id, player_id } => {
            handle_kick_from_playtest(state, tx, _player_id, playtest_id, player_id).await;
        }

        AdminClientMessage::SetPlaytestPaused { playtest_id, paused } => {
            handle_set_playtest_paused(state, tx, _player_id, playtest_id, paused).await;
        }

        AdminClientMessage::SetPlaytestTimeScale { playtest_id, scale } => {
            handle_set_playtest_time_scale(state, tx, _player_id, playtest_id, scale).await;
        }

        AdminClientMessage::DestroyPlaytest { playtest_id } => {
            handle_destroy_playtest(state, tx, _player_id, playtest_id).await;
        }

        AdminClientMessage::PromotePlaytest { playtest_id, promote_config } => {
            handle_promote_playtest(state, tx, _player_id, playtest_id, promote_config).await;
        }

        AdminClientMessage::PreviewPromote { playtest_id, promote_config } => {
            handle_preview_promote(state, tx, _player_id, playtest_id, promote_config).await;
        }

        // === Script Debugging ===

        AdminClientMessage::SubscribeScriptErrors => {
            // Script error subscription is handled by a separate mechanism
            // For now, just acknowledge
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::SubscribedToScriptErrors)).await;
        }

        AdminClientMessage::UnsubscribeScriptErrors => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::UnsubscribedFromScriptErrors)).await;
        }

        AdminClientMessage::ReloadScript { path } => {
            handle_reload_script(state, tx, &path).await;
        }

        AdminClientMessage::ReloadDefinitions => {
            handle_reload_definitions(state, tx).await;
        }

        AdminClientMessage::GetRecentScriptErrors { limit } => {
            handle_get_recent_errors(tx, limit).await;
        }

        // === Interactive Debugger ===

        AdminClientMessage::StartDebugSession { target } => {
            handle_start_debug_session(state.clone(), tx, _player_id, target).await;
        }

        AdminClientMessage::EndDebugSession => {
            handle_end_debug_session(state, tx, _player_id).await;
        }

        AdminClientMessage::SetBreakpoint { script, line, condition } => {
            handle_set_breakpoint(state, tx, _player_id, script, line, condition).await;
        }

        AdminClientMessage::SetFunctionBreakpoint { function_name, break_on_entry, break_on_exit } => {
            handle_set_function_breakpoint(state, tx, _player_id, function_name, break_on_entry, break_on_exit).await;
        }

        AdminClientMessage::RemoveBreakpoint { breakpoint_id } => {
            handle_remove_breakpoint(state, tx, _player_id, breakpoint_id).await;
        }

        AdminClientMessage::ToggleBreakpoint { breakpoint_id, enabled } => {
            handle_toggle_breakpoint(state, tx, _player_id, breakpoint_id, enabled).await;
        }

        AdminClientMessage::ListBreakpoints => {
            handle_list_breakpoints(state, tx, _player_id).await;
        }

        AdminClientMessage::DebugContinue => {
            handle_debug_continue(state, tx, _player_id).await;
        }

        AdminClientMessage::DebugPause => {
            handle_debug_pause(state, tx, _player_id).await;
        }

        AdminClientMessage::DebugStepInto => {
            handle_debug_step(state, tx, _player_id, DebugCommand::StepInto).await;
        }

        AdminClientMessage::DebugStepOver => {
            handle_debug_step(state, tx, _player_id, DebugCommand::StepOver).await;
        }

        AdminClientMessage::DebugStepOut => {
            handle_debug_step(state, tx, _player_id, DebugCommand::StepOut).await;
        }

        AdminClientMessage::GetVariables { frame_index } => {
            handle_get_variables(state, tx, _player_id, frame_index).await;
        }

        AdminClientMessage::ExpandVariable { variable_path } => {
            handle_expand_variable(state, tx, _player_id, variable_path).await;
        }

        AdminClientMessage::EvaluateExpression { expression, frame_index } => {
            handle_evaluate_expression(state, tx, _player_id, expression, frame_index).await;
        }

        AdminClientMessage::GetCallStack => {
            handle_get_call_stack(state, tx, _player_id).await;
        }

        // === Schema Introspection ===

        AdminClientMessage::GetArchetypeSchemas => {
            handle_get_archetype_schemas(tx).await;
        }

        AdminClientMessage::GetArchetypeSchema { archetype_type } => {
            handle_get_archetype_schema(tx, &archetype_type).await;
        }

        AdminClientMessage::GetActionSchemas => {
            handle_get_action_schemas(tx).await;
        }

        // === Script Validation ===

        AdminClientMessage::ValidateScript { path, content } => {
            handle_validate_script(tx, &path, &content).await;
        }

        AdminClientMessage::ValidateDefinition { definition_type, content } => {
            handle_validate_definition(tx, &definition_type, &content).await;
        }

        // === State Introspection (stub for Phase 2) ===

        AdminClientMessage::SubscribeStateUpdates { entity_types, .. } => {
            // TODO: Implement state subscription
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::StateSubscribed {
                entity_types,
            })).await;
        }

        AdminClientMessage::UnsubscribeStateUpdates => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::StateUnsubscribed)).await;
        }

        AdminClientMessage::GetStateSnapshot { entity_type, limit, offset, .. } => {
            handle_get_state_snapshot(state, tx, entity_type, limit, offset).await;
        }

        AdminClientMessage::CreateWatch { expression, name } => {
            handle_create_watch(tx, expression, name).await;
        }

        AdminClientMessage::RemoveWatch { watch_id } => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::WatchRemoved { watch_id })).await;
        }

        AdminClientMessage::ListWatches => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::WatchList {
                watches: vec![],
            })).await;
        }

        // === Export (stub for Phase 3) ===

        AdminClientMessage::CreateExport { name, .. } => {
            let export_id = Uuid::new_v4();
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ExportCreated {
                export_id,
                name,
            })).await;
            // TODO: Implement actual export creation
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ExportFailed {
                export_id,
                error: "Export not yet implemented".to_string(),
            })).await;
        }

        AdminClientMessage::GetExportStatus { export_id } => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "NOT_IMPLEMENTED".to_string(),
                message: format!("Export {} status not yet implemented", export_id),
            })).await;
        }

        AdminClientMessage::ListExports => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ExportList {
                exports: vec![],
            })).await;
        }

        AdminClientMessage::DeleteExport { export_id } => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ExportDeleted { export_id })).await;
        }

        AdminClientMessage::DownloadExport { export_id } => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "NOT_IMPLEMENTED".to_string(),
                message: format!("Export {} download not yet implemented", export_id),
            })).await;
        }
    }

    true
}

// =============================================================================
// Script Operations
// =============================================================================

async fn handle_list_scripts(tx: &mpsc::Sender<ServerMessage>) {
    let scripts_dir = config().get().scripting.scripts_dir.clone();
    let base_path = Path::new(&scripts_dir);

    let mut files = Vec::new();

    if let Ok(entries) = walk_scripts_dir(base_path, base_path) {
        files = entries;
    }

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ScriptList { files })).await;
}

fn walk_scripts_dir(base: &Path, current: &Path) -> std::io::Result<Vec<ScriptFileInfo>> {
    let mut files = Vec::new();

    if current.is_dir() {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                files.extend(walk_scripts_dir(base, &path)?);
            } else if path.extension().map(|e| e == "rhai").unwrap_or(false) {
                let relative_path = path.strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                let metadata = std::fs::metadata(&path)?;
                let last_modified = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                files.push(ScriptFileInfo {
                    path: relative_path,
                    name: path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    size_bytes: metadata.len(),
                    last_modified,
                });
            }
        }
    }

    Ok(files)
}

async fn handle_read_script(tx: &mpsc::Sender<ServerMessage>, path: &str) {
    // Validate path (prevent directory traversal)
    if path.contains("..") || path.starts_with('/') {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "INVALID_PATH".to_string(),
            message: "Invalid script path".to_string(),
        })).await;
        return;
    }

    let scripts_dir = config().get().scripting.scripts_dir.clone();
    let full_path = Path::new(&scripts_dir).join(path);

    // Ensure path is still within scripts directory
    if !full_path.starts_with(&scripts_dir) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "INVALID_PATH".to_string(),
            message: "Path outside scripts directory".to_string(),
        })).await;
        return;
    }

    match std::fs::read_to_string(&full_path) {
        Ok(content) => {
            let last_modified = std::fs::metadata(&full_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ScriptContent {
                path: path.to_string(),
                content,
                last_modified,
            })).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "READ_FAILED".to_string(),
                message: format!("Failed to read script: {}", e),
            })).await;
        }
    }
}

async fn handle_write_script(tx: &mpsc::Sender<ServerMessage>, path: &str, content: &str) {
    // Validate path
    if path.contains("..") || path.starts_with('/') {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "INVALID_PATH".to_string(),
            message: "Invalid script path".to_string(),
        })).await;
        return;
    }

    let scripts_dir = config().get().scripting.scripts_dir.clone();
    let full_path = Path::new(&scripts_dir).join(path);

    if !full_path.starts_with(&scripts_dir) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "INVALID_PATH".to_string(),
            message: "Path outside scripts directory".to_string(),
        })).await;
        return;
    }

    // Ensure parent directories exist
    if let Some(parent) = full_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "WRITE_FAILED".to_string(),
                message: format!("Failed to create directories: {}", e),
            })).await;
            return;
        }
    }

    match std::fs::write(&full_path, content) {
        Ok(()) => {
            tracing::info!("Admin wrote script: {}", path);

            let last_modified = std::fs::metadata(&full_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ScriptContent {
                path: path.to_string(),
                content: content.to_string(),
                last_modified,
            })).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "WRITE_FAILED".to_string(),
                message: format!("Failed to write script: {}", e),
            })).await;
        }
    }
}

async fn handle_reload_script(state: &Arc<GameState>, tx: &mpsc::Sender<ServerMessage>, path: &str) {
    // Validate path
    if path.contains("..") || path.starts_with('/') {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "INVALID_PATH".to_string(),
            message: "Invalid script path".to_string(),
        })).await;
        return;
    }

    // Try to reload the script in the engine
    match state.scripts.reload_script(path) {
        Ok(warnings) => {
            tracing::info!("Reloaded script: {}", path);

            // Reinitialize if this is an action script (calls init() to re-register handlers)
            state.reinitialize_action_script(path);

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ScriptReloaded {
                path: path.to_string(),
                success: true,
                error: None,
                warnings,
            })).await;
        }
        Err(e) => {
            tracing::warn!("Failed to reload script {}: {}", path, e);
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ScriptReloaded {
                path: path.to_string(),
                success: false,
                error: Some(e.to_string()),
                warnings: vec![],
            })).await;
        }
    }
}

async fn handle_reload_definitions(state: &Arc<GameState>, tx: &mpsc::Sender<ServerMessage>) {
    // Reload archetype definitions
    match state.scripts.reload_definitions() {
        Ok((ships, weapons, errors)) => {
            tracing::info!("Reloaded definitions: {} ships, {} weapons", ships, weapons);
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DefinitionsReloaded {
                file: "definitions".to_string(),
                ships_loaded: ships,
                weapons_loaded: weapons,
                errors,
            })).await;
        }
        Err(e) => {
            tracing::warn!("Failed to reload definitions: {}", e);
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "RELOAD_FAILED".to_string(),
                message: format!("Failed to reload definitions: {}", e),
            })).await;
        }
    }
}

async fn handle_get_recent_errors(tx: &mpsc::Sender<ServerMessage>, limit: usize) {
    // Get errors from the thread-local buffer
    // Note: This gets errors from the current thread only
    // A more robust implementation would use a shared error buffer
    let errors: Vec<ScriptErrorDto> = bw_scripting::take_errors()
        .into_iter()
        .take(limit)
        .map(|e| {
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            ScriptErrorDto {
                script: e.script,
                function: e.function,
                message: e.message,
                line: e.line,
                column: e.column,
                tick: e.tick,
                timestamp_ms,
            }
        })
        .collect();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ScriptErrors { errors })).await;
}

// =============================================================================
// Config Operations
// =============================================================================

async fn handle_get_sim_config(tx: &mpsc::Sender<ServerMessage>) {
    let sim_config = &config().get().simulation;

    match serde_json::to_value(sim_config) {
        Ok(value) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::SimConfigData {
                config: value,
            })).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "SERIALIZE_FAILED".to_string(),
                message: format!("Failed to serialize config: {}", e),
            })).await;
        }
    }
}

// =============================================================================
// Entity Operations
// =============================================================================

async fn handle_query_entities(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    entity_type: EntityType,
    filters: Vec<EntityFilter>,
    limit: usize,
    offset: usize,
) {
    let mut entities = Vec::new();
    let mut total_count = 0;

    match entity_type {
        EntityType::Ship => {
            let all_ships: Vec<_> = state.ships.iter()
                .filter(|ship| apply_ship_filters(&ship, &filters))
                .collect();

            total_count = all_ships.len();

            for ship in all_ships.into_iter().skip(offset).take(limit) {
                entities.push(EntitySummary {
                    id: ship.id,
                    name: ship.name.clone(),
                    entity_type: EntityType::Ship,
                    sector_id: Some(ship.sector_id),
                    status: format!("{:?}", ship.status),
                    extra: serde_json::json!({
                        "ship_class": format!("{:?}", ship.ship_class),
                        "hull_percent": ship.hull_integrity,
                        "is_player": ship.is_player_ship,
                    }),
                });
            }
        }

        EntityType::Player => {
            let all_players: Vec<_> = state.player_data.iter()
                .filter(|p| apply_player_filters(&p, &filters))
                .collect();

            total_count = all_players.len();

            for player in all_players.into_iter().skip(offset).take(limit) {
                entities.push(EntitySummary {
                    id: player.id,
                    name: player.username.clone(),
                    entity_type: EntityType::Player,
                    sector_id: state.players.get(&player.id).map(|s| s.sector_id),
                    status: if player.is_online { "Online" } else { "Offline" }.to_string(),
                    extra: serde_json::json!({
                        "reputation": player.resources.reputation,
                        "fame": player.resources.fame,
                    }),
                });
            }
        }

        EntityType::Mission => {
            let mut all_missions = Vec::new();

            for sector in state.sectors.iter() {
                for mission in sector.missions.iter() {
                    if apply_mission_filters(&mission, &sector.sector.id, &filters) {
                        all_missions.push((sector.sector.id, mission.clone()));
                    }
                }
            }

            total_count = all_missions.len();

            for (sector_id, mission) in all_missions.into_iter().skip(offset).take(limit) {
                entities.push(EntitySummary {
                    id: mission.id,
                    name: mission.title.clone(),
                    entity_type: EntityType::Mission,
                    sector_id: Some(sector_id),
                    status: format!("{:?}", mission.status),
                    extra: serde_json::json!({
                        "mission_type": format!("{:?}", mission.mission_type),
                        "reputation_reward": mission.reputation_reward,
                    }),
                });
            }
        }

        EntityType::Station => {
            let mut all_stations = Vec::new();

            for sector in state.sectors.iter() {
                for location in &sector.sector.locations {
                    if location.location_type.is_station() {
                        all_stations.push((sector.sector.id, location.clone()));
                    }
                }
            }

            total_count = all_stations.len();

            for (sector_id, station) in all_stations.into_iter().skip(offset).take(limit) {
                entities.push(EntitySummary {
                    id: station.id,
                    name: station.name.clone(),
                    entity_type: EntityType::Station,
                    sector_id: Some(sector_id),
                    status: "Active".to_string(),
                    extra: serde_json::json!({
                        "services": station.services.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>(),
                    }),
                });
            }
        }

        EntityType::Sector => {
            let all_sectors: Vec<_> = state.sectors.iter().collect();

            total_count = all_sectors.len();

            for sector in all_sectors.into_iter().skip(offset).take(limit) {
                entities.push(EntitySummary {
                    id: sector.sector.id,
                    name: sector.sector.name.clone(),
                    entity_type: EntityType::Sector,
                    sector_id: None,
                    status: format!("{:?}", sector.sector.danger_level),
                    extra: serde_json::json!({
                        "ship_count": sector.ship_ids.len(),
                        "location_count": sector.sector.locations.len(),
                    }),
                });
            }
        }
    }

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::EntityList {
        entity_type,
        entities,
        total_count,
    })).await;
}

fn apply_ship_filters(ship: &bw_core::models::Ship, filters: &[EntityFilter]) -> bool {
    for filter in filters {
        match filter {
            EntityFilter::NameContains(s) => {
                if !ship.name.to_lowercase().contains(&s.to_lowercase()) {
                    return false;
                }
            }
            EntityFilter::InSector(sector_id) => {
                if ship.sector_id != *sector_id {
                    return false;
                }
            }
            EntityFilter::IsNpc(is_npc) => {
                if ship.is_player_ship == *is_npc {
                    return false;
                }
            }
            EntityFilter::IsHostile(is_hostile) => {
                if ship.ship_class.is_hostile() != *is_hostile {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn apply_player_filters(player: &bw_core::models::Player, filters: &[EntityFilter]) -> bool {
    for filter in filters {
        match filter {
            EntityFilter::NameContains(s) => {
                if !player.username.to_lowercase().contains(&s.to_lowercase()) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn apply_mission_filters(mission: &bw_core::models::Mission, sector_id: &Uuid, filters: &[EntityFilter]) -> bool {
    for filter in filters {
        match filter {
            EntityFilter::NameContains(s) => {
                if !mission.title.to_lowercase().contains(&s.to_lowercase()) {
                    return false;
                }
            }
            EntityFilter::InSector(sid) => {
                if sector_id != sid {
                    return false;
                }
            }
            EntityFilter::ByStatus(status) => {
                if format!("{:?}", mission.status).to_lowercase() != status.to_lowercase() {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

async fn handle_get_entity(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    entity_type: EntityType,
    id: Uuid,
) {
    let data = match entity_type {
        EntityType::Ship => {
            state.ships.get(&id).map(|s| serde_json::to_value(&*s).ok()).flatten()
        }
        EntityType::Player => {
            state.player_data.get(&id).map(|p| serde_json::to_value(&*p).ok()).flatten()
        }
        EntityType::Mission => {
            let mut found = None;
            for sector in state.sectors.iter() {
                if let Some(mission) = sector.missions.get(&id) {
                    found = serde_json::to_value(&*mission).ok();
                    break;
                }
            }
            found
        }
        EntityType::Station => {
            let mut found = None;
            for sector in state.sectors.iter() {
                if let Some(loc) = sector.sector.locations.iter().find(|l| l.id == id) {
                    found = serde_json::to_value(loc).ok();
                    break;
                }
            }
            found
        }
        EntityType::Sector => {
            state.sectors.get(&id).map(|s| serde_json::to_value(&s.sector).ok()).flatten()
        }
    };

    match data {
        Some(data) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::EntityDetails {
                entity_type,
                id,
                data,
            })).await;
        }
        None => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "NOT_FOUND".to_string(),
                message: format!("{:?} with id {} not found", entity_type, id),
            })).await;
        }
    }
}

// =============================================================================
// Sector Operations
// =============================================================================

async fn handle_query_sectors(state: &GameState, tx: &mpsc::Sender<ServerMessage>) {
    let sectors: Vec<SectorSummaryAdmin> = state.sectors.iter()
        .map(|sector| {
            let player_count = sector.ship_ids.iter()
                .filter(|entry| {
                    state.ships.get(entry.key())
                        .map(|s| s.is_player_ship)
                        .unwrap_or(false)
                })
                .count();

            let npc_count = sector.ship_ids.len() - player_count;

            SectorSummaryAdmin {
                id: sector.sector.id,
                name: sector.sector.name.clone(),
                danger_level: format!("{:?}", sector.sector.danger_level),
                ship_count: sector.ship_ids.len(),
                player_count,
                npc_count,
                mission_count: sector.missions.len(),
                location_count: sector.sector.locations.len(),
            }
        })
        .collect();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::SectorList { sectors })).await;
}

// =============================================================================
// Staged Changes
// =============================================================================

async fn handle_preview_staged(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    changes: &[StagedChange],
) {
    let mut previews = Vec::new();
    let mut errors = Vec::new();

    for (index, change) in changes.iter().enumerate() {
        match validate_and_preview_change(state, change) {
            Ok(preview) => previews.push(preview),
            Err(err) => errors.push(ValidationError {
                change_index: index,
                field: err.0,
                message: err.1,
            }),
        }
    }

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::StagedPreview {
        changes: previews,
        errors,
    })).await;
}

fn validate_and_preview_change(
    state: &GameState,
    change: &StagedChange,
) -> Result<ChangePreview, (String, String)> {
    match change {
        StagedChange::ScriptUpdate { path, content } => {
            // Validate path
            if path.contains("..") || path.starts_with('/') {
                return Err(("path".to_string(), "Invalid path".to_string()));
            }

            // Try to read existing content
            let scripts_dir = config().get().scripting.scripts_dir.clone();
            let full_path = Path::new(&scripts_dir).join(path);
            let before = std::fs::read_to_string(&full_path).ok().map(|c| serde_json::json!(c));

            Ok(ChangePreview {
                change_type: "ScriptUpdate".to_string(),
                target: path.clone(),
                before,
                after: serde_json::json!(content),
            })
        }

        StagedChange::SimConfigUpdate { section, value } => {
            let current_config = &config().get().simulation;
            let before = match section {
                SimConfigSection::NpcSpawning => serde_json::to_value(&current_config.npc_spawning).ok(),
                SimConfigSection::Combat => serde_json::to_value(&current_config.combat).ok(),
                SimConfigSection::DangerMultipliers => serde_json::to_value(&current_config.npc_spawning.danger_multipliers).ok(),
                SimConfigSection::Full => serde_json::to_value(current_config).ok(),
            };

            Ok(ChangePreview {
                change_type: "SimConfigUpdate".to_string(),
                target: format!("{:?}", section),
                before,
                after: value.clone(),
            })
        }

        StagedChange::EntityUpdate { entity_type, id, changes } => {
            let before = match entity_type {
                EntityType::Ship => state.ships.get(id).map(|s| serde_json::to_value(&*s).ok()).flatten(),
                EntityType::Player => state.player_data.get(id).map(|p| serde_json::to_value(&*p).ok()).flatten(),
                _ => None,
            };

            if before.is_none() {
                return Err(("id".to_string(), format!("{:?} not found", entity_type)));
            }

            Ok(ChangePreview {
                change_type: "EntityUpdate".to_string(),
                target: format!("{:?}:{}", entity_type, id),
                before,
                after: changes.clone(),
            })
        }

        StagedChange::EntitySpawn { entity_type, config } => {
            Ok(ChangePreview {
                change_type: "EntitySpawn".to_string(),
                target: format!("New {:?}", entity_type),
                before: None,
                after: config.clone(),
            })
        }

        StagedChange::EntityDelete { entity_type, id } => {
            let before = match entity_type {
                EntityType::Ship => state.ships.get(id).map(|s| serde_json::to_value(&*s).ok()).flatten(),
                EntityType::Mission => {
                    let mut found = None;
                    for sector in state.sectors.iter() {
                        if let Some(m) = sector.missions.get(id) {
                            found = serde_json::to_value(&*m).ok();
                            break;
                        }
                    }
                    found
                }
                _ => None,
            };

            if before.is_none() {
                return Err(("id".to_string(), format!("{:?} not found", entity_type)));
            }

            Ok(ChangePreview {
                change_type: "EntityDelete".to_string(),
                target: format!("{:?}:{}", entity_type, id),
                before,
                after: serde_json::json!(null),
            })
        }

        StagedChange::SectorUpdate { id, changes } => {
            let before = state.sectors.get(id)
                .map(|s| serde_json::to_value(&s.sector).ok())
                .flatten();

            if before.is_none() {
                return Err(("id".to_string(), "Sector not found".to_string()));
            }

            Ok(ChangePreview {
                change_type: "SectorUpdate".to_string(),
                target: format!("Sector:{}", id),
                before,
                after: changes.clone(),
            })
        }

        StagedChange::LocationAdd { sector_id, location } => {
            if !state.sectors.contains_key(sector_id) {
                return Err(("sector_id".to_string(), "Sector not found".to_string()));
            }

            Ok(ChangePreview {
                change_type: "LocationAdd".to_string(),
                target: format!("Sector:{}", sector_id),
                before: None,
                after: location.clone(),
            })
        }

        StagedChange::LocationRemove { sector_id, location_id } => {
            let before = state.sectors.get(sector_id)
                .and_then(|s| {
                    s.sector.locations.iter()
                        .find(|l| l.id == *location_id)
                        .map(|l| serde_json::to_value(l).ok())
                        .flatten()
                });

            if before.is_none() {
                return Err(("location_id".to_string(), "Location not found".to_string()));
            }

            Ok(ChangePreview {
                change_type: "LocationRemove".to_string(),
                target: format!("Location:{}", location_id),
                before,
                after: serde_json::json!(null),
            })
        }
    }
}

async fn handle_commit_staged(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    username: &str,
    changes: Vec<StagedChange>,
) {
    let mut applied_count = 0;
    let mut errors = Vec::new();

    // First validate all changes
    for (index, change) in changes.iter().enumerate() {
        if let Err((field, msg)) = validate_and_preview_change(state, change) {
            errors.push(format!("Change {}: {} - {}", index, field, msg));
        }
    }

    if !errors.is_empty() {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::CommitResult {
            success: false,
            applied_count: 0,
            errors,
        })).await;
        return;
    }

    // Apply all changes
    for change in changes {
        match apply_change(state, &change).await {
            Ok(()) => {
                applied_count += 1;
                tracing::info!("Admin {} applied change: {:?}", username, change);
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::CommitResult {
        success: errors.is_empty(),
        applied_count,
        errors,
    })).await;
}

async fn apply_change(state: &GameState, change: &StagedChange) -> Result<(), String> {
    match change {
        StagedChange::ScriptUpdate { path, content } => {
            let scripts_dir = config().get().scripting.scripts_dir.clone();
            let full_path = Path::new(&scripts_dir).join(path);

            // Ensure parent directories exist
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }

            std::fs::write(&full_path, content)
                .map_err(|e| format!("Failed to write script: {}", e))?;

            Ok(())
        }

        StagedChange::SimConfigUpdate { section: _, value: _ } => {
            // Config updates require writing to config.toml and reloading
            // For now, just reload the config to pick up any manual changes
            config().reload();
            Ok(())
        }

        StagedChange::EntityUpdate { entity_type, id, changes } => {
            match entity_type {
                EntityType::Ship => {
                    if let Some(mut ship) = state.ships.get_mut(id) {
                        // Apply partial updates from JSON
                        if let Some(name) = changes.get("name").and_then(|v| v.as_str()) {
                            ship.name = name.to_string();
                        }
                        if let Some(hull) = changes.get("hull_integrity").and_then(|v| v.as_f64()) {
                            ship.hull_integrity = hull as f32;
                        }
                        if let Some(shield) = changes.get("shield_strength").and_then(|v| v.as_f64()) {
                            ship.shield_strength = shield as f32;
                        }
                    }
                    Ok(())
                }
                EntityType::Player => {
                    if let Some(mut player) = state.player_data.get_mut(id) {
                        if let Some(rep) = changes.get("reputation").and_then(|v| v.as_i64()) {
                            player.resources.reputation = rep as i32;
                        }
                        if let Some(fame) = changes.get("fame").and_then(|v| v.as_i64()) {
                            player.resources.fame = fame as i32;
                        }
                    }
                    Ok(())
                }
                _ => Err(format!("Cannot update {:?} entities", entity_type)),
            }
        }

        StagedChange::EntitySpawn { entity_type, config } => {
            match entity_type {
                EntityType::Ship => {
                    // Parse ship config and spawn
                    let name = config.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Ship");
                    let sector_id = config.get("sector_id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .ok_or("Invalid sector_id")?;

                    let ship = bw_core::models::Ship::new_npc_ship(
                        name.to_string(),
                        bw_core::models::ShipClass::PirateRaider, // Default
                        sector_id,
                        bw_core::models::Position::new(0.0, 0.0, 0.0),
                        None,
                    );

                    let ship_id = ship.id;
                    state.ships.insert(ship_id, ship);

                    if let Some(sector) = state.sectors.get(&sector_id) {
                        sector.ship_ids.insert(ship_id, ());
                    }

                    Ok(())
                }
                _ => Err(format!("Cannot spawn {:?} entities", entity_type)),
            }
        }

        StagedChange::EntityDelete { entity_type, id } => {
            match entity_type {
                EntityType::Ship => {
                    if let Some((_, ship)) = state.ships.remove(id) {
                        // Remove from sector
                        if let Some(sector) = state.sectors.get(&ship.sector_id) {
                            sector.ship_ids.remove(id);
                        }
                    }
                    Ok(())
                }
                EntityType::Mission => {
                    for sector in state.sectors.iter() {
                        if sector.missions.remove(id).is_some() {
                            break;
                        }
                    }
                    Ok(())
                }
                _ => Err(format!("Cannot delete {:?} entities", entity_type)),
            }
        }

        StagedChange::SectorUpdate { id: _, changes: _ } => {
            // Sector updates are complex - skip for now
            Err("Sector updates not yet implemented".to_string())
        }

        StagedChange::LocationAdd { sector_id: _, location: _ } => {
            // Location adds are complex - skip for now
            Err("Location adds not yet implemented".to_string())
        }

        StagedChange::LocationRemove { sector_id, location_id } => {
            if let Some(mut sector) = state.sectors.get_mut(sector_id) {
                sector.sector.locations.retain(|l| l.id != *location_id);
            }
            Ok(())
        }
    }
}

// =============================================================================
// Playtest Operations
// =============================================================================

async fn handle_create_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    owner_id: Uuid,
    name: String,
    fork_config_dto: ForkConfigDto,
) {
    use crate::playtest::{ForkConfig, PlaytestBuilder};

    // Validate owner has an active session
    let Some(session) = state.players.get(&owner_id) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "NO_SESSION".to_string(),
            message: "You must be connected to create a playtest".to_string(),
        })).await;
        return;
    };

    // Check if already in a playtest
    if session.playtest_id.is_some() {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "ALREADY_IN_PLAYTEST".to_string(),
            message: "Leave your current playtest first".to_string(),
        })).await;
        return;
    }

    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session); // Release lock before further operations

    // Convert DTO to internal config
    let fork_config = ForkConfig {
        sectors: fork_config_dto.sectors,
        include_ships: fork_config_dto.include_ships,
        specific_ships: fork_config_dto.specific_ships,
        include_missions: fork_config_dto.include_missions,
        include_npcs: fork_config_dto.include_npcs,
        include_other_players: fork_config_dto.include_other_players,
    };

    // Validate fork config - check specified sectors exist
    for sector_id in &fork_config.sectors {
        if !state.sectors.contains_key(sector_id) {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "INVALID_SECTOR".to_string(),
                message: format!("Sector {} does not exist", sector_id),
            })).await;
            return;
        }
    }

    // Create playtest builder
    let builder = PlaytestBuilder::new(
        owner_id,
        name.clone(),
        state.get_tick(),
        fork_config.clone(),
    );
    let playtest_id = builder.id();

    // Build the instance with shared faction data and scripting components
    let factions = state.get_factions_arc();
    let faction_tags = state.get_faction_tags_arc();
    let instance = builder.build(
        factions,
        faction_tags,
        state.scripts.clone(),
        state.debug_controller.clone(),
    );

    // Fork state into the instance
    state.fork_to_playtest(&instance, &fork_config);

    // Add owner as participant
    let _ = instance.add_participant(owner_id, true, ship_id, sector_id);

    // Update session BEFORE registering to avoid race condition where
    // messages could be routed before session knows about playtest
    if let Some(mut session) = state.players.get_mut(&owner_id) {
        session.playtest_id = Some(playtest_id);
    }

    // Register with manager
    match state.playtest_manager.register(instance) {
        Ok(instance) => {
            // Initialize scripting systems now that the instance is Arc-wrapped
            instance.initialize_scripting();

            // Attach behaviors from live server to forked NPC ships
            let live_behaviors = state.behavior_manager.read().list_all();
            instance.attach_forked_behaviors(&live_behaviors);

            // Spawn the simulation loop for this playtest
            let handle = crate::playtest::spawn_playtest_loop(instance);
            state.playtest_manager.register_simulation_task(playtest_id, handle);

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestCreated {
                playtest_id,
                name,
            })).await;
        }
        Err(e) => {
            // Rollback session update on failure
            if let Some(mut session) = state.players.get_mut(&owner_id) {
                session.playtest_id = None;
            }
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "PLAYTEST_CREATE_FAILED".to_string(),
                message: format!("{}", e),
            })).await;
        }
    }
}

async fn handle_list_playtests(state: &GameState, tx: &mpsc::Sender<ServerMessage>) {
    let playtests: Vec<PlaytestSummaryDto> = state.playtest_manager.list_all()
        .iter()
        .map(|instance| {
            let owner_name = state.player_data.get(&instance.owner_id)
                .map(|p| p.username.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            PlaytestSummaryDto {
                id: instance.id,
                name: instance.name.clone(),
                owner_id: instance.owner_id,
                owner_name,
                participant_count: instance.participant_count(),
                created_at_tick: instance.created_at_tick,
                current_tick: instance.get_tick(),
                paused: instance.is_paused(),
                time_scale: instance.get_time_scale(),
            }
        })
        .collect();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestList { playtests })).await;
}

async fn handle_get_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    playtest_id: Uuid,
) {
    match state.playtest_manager.get(playtest_id) {
        Some(instance) => {
            let owner_name = state.player_data.get(&instance.owner_id)
                .map(|p| p.username.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let participants: Vec<PlaytestParticipantDto> = instance.participants.iter()
                .map(|p| {
                    let player_name = state.player_data.get(&p.player_id)
                        .map(|pl| pl.username.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    PlaytestParticipantDto {
                        player_id: p.player_id,
                        player_name,
                        is_gm: p.is_gm,
                        ship_id: p.playtest_ship_id,
                        sector_id: p.playtest_sector_id,
                        is_connected: p.connection.is_some(),
                    }
                })
                .collect();

            let playtest = PlaytestDetailDto {
                id: instance.id,
                name: instance.name.clone(),
                owner_id: instance.owner_id,
                owner_name,
                participants,
                created_at_tick: instance.created_at_tick,
                current_tick: instance.get_tick(),
                paused: instance.is_paused(),
                time_scale: instance.get_time_scale(),
                sector_count: instance.sector_count(),
                ship_count: instance.ship_count(),
                player_count: instance.player_data.len(),
            };

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestDetails { playtest })).await;
        }
        None => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "NOT_FOUND".to_string(),
                message: "Playtest not found".to_string(),
            })).await;
        }
    }
}

async fn handle_join_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    playtest_id: Uuid,
) {
    // Get player's current ship and sector - require active session
    let Some(session) = state.players.get(&player_id) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "NO_SESSION".to_string(),
            message: "You must be connected to join a playtest".to_string(),
        })).await;
        return;
    };
    let ship_id = session.ship_id;
    let sector_id = session.sector_id;
    drop(session);

    match state.playtest_manager.join_playtest(playtest_id, player_id, ship_id, sector_id) {
        Ok(_) => {
            // Update player session
            if let Some(mut session) = state.players.get_mut(&player_id) {
                session.playtest_id = Some(playtest_id);
            }

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestJoined {
                playtest_id,
                ship_id,
                sector_id,
            })).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "JOIN_FAILED".to_string(),
                message: format!("{}", e),
            })).await;
        }
    }
}

async fn handle_leave_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
) {
    match state.playtest_manager.leave_playtest(player_id) {
        Ok(()) => {
            // Update player session
            if let Some(mut session) = state.players.get_mut(&player_id) {
                session.playtest_id = None;
            }

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestLeft)).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "LEAVE_FAILED".to_string(),
                message: format!("{}", e),
            })).await;
        }
    }
}

async fn handle_invite_to_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    inviter_id: Uuid,
    playtest_id: Uuid,
    invitee_id: Uuid,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, inviter_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can invite players".to_string(),
        })).await;
        return;
    }

    // Get invitee's ship and sector - require active session
    let Some(invitee_session) = state.players.get(&invitee_id) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "PLAYER_NOT_CONNECTED".to_string(),
            message: "Invitee must be connected to be invited".to_string(),
        })).await;
        return;
    };
    let ship_id = invitee_session.ship_id;
    let sector_id = invitee_session.sector_id;
    drop(invitee_session);

    // Add them to the playtest
    match state.playtest_manager.join_playtest(playtest_id, invitee_id, ship_id, sector_id) {
        Ok(_) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestInviteSent {
                playtest_id,
                player_id: invitee_id,
            })).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "INVITE_FAILED".to_string(),
                message: format!("{}", e),
            })).await;
        }
    }
}

async fn handle_kick_from_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    kicker_id: Uuid,
    playtest_id: Uuid,
    kickee_id: Uuid,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, kicker_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can kick players".to_string(),
        })).await;
        return;
    }

    // Can't kick yourself (use leave instead)
    if kicker_id == kickee_id {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "INVALID_OPERATION".to_string(),
            message: "Cannot kick yourself, use leave instead".to_string(),
        })).await;
        return;
    }

    // Remove from playtest manager
    let _ = state.playtest_manager.leave_playtest(kickee_id);

    // Update kicked player's session
    if let Some(mut session) = state.players.get_mut(&kickee_id) {
        session.playtest_id = None;
    }

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestPlayerKicked {
        playtest_id,
        player_id: kickee_id,
    })).await;
}

async fn handle_set_playtest_paused(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    playtest_id: Uuid,
    paused: bool,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, player_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can change pause state".to_string(),
        })).await;
        return;
    }

    if let Some(instance) = state.playtest_manager.get(playtest_id) {
        instance.set_paused(paused);

        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestStateChanged {
            playtest_id,
            paused,
            time_scale: instance.get_time_scale(),
            tick: instance.get_tick(),
        })).await;
    }
}

async fn handle_set_playtest_time_scale(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    playtest_id: Uuid,
    scale: f32,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, player_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can change time scale".to_string(),
        })).await;
        return;
    }

    if let Some(instance) = state.playtest_manager.get(playtest_id) {
        instance.set_time_scale(scale);

        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestStateChanged {
            playtest_id,
            paused: instance.is_paused(),
            time_scale: instance.get_time_scale(),
            tick: instance.get_tick(),
        })).await;
    }
}

async fn handle_destroy_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    playtest_id: Uuid,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, player_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can destroy it".to_string(),
        })).await;
        return;
    }

    // Notify all participants and clear their sessions
    if let Some(instance) = state.playtest_manager.get(playtest_id) {
        let destroy_msg = ServerMessage::Admin(AdminServerMessage::PlaytestDestroyed {
            playtest_id,
        });

        for participant in instance.participants.iter() {
            // Notify participant via their playtest connection
            if let Some(ref conn) = participant.connection {
                let _ = conn.send(destroy_msg.clone()).await;
            }

            // Also notify via their live session connection (in case they're viewing both)
            if let Some(session) = state.players.get(&participant.player_id) {
                if let Some(ref conn) = session.connection {
                    let _ = conn.send(destroy_msg.clone()).await;
                }
            }

            // Clear playtest_id from session
            if let Some(mut session) = state.players.get_mut(&participant.player_id) {
                session.playtest_id = None;
            }
        }
    }

    match state.playtest_manager.destroy(playtest_id) {
        Ok(()) => {
            // Owner already notified above, but send confirmation
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PlaytestDestroyed {
                playtest_id,
            })).await;
        }
        Err(e) => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "DESTROY_FAILED".to_string(),
                message: format!("{}", e),
            })).await;
        }
    }
}

async fn handle_promote_playtest(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    playtest_id: Uuid,
    promote_config: PromoteConfigDto,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, player_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can promote changes".to_string(),
        })).await;
        return;
    }

    // Get the playtest instance
    let Some(instance) = state.playtest_manager.get(playtest_id) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "NOT_FOUND".to_string(),
            message: "Playtest not found".to_string(),
        })).await;
        return;
    };

    let mut applied_count = 0;
    let mut errors = Vec::new();

    // Promote ship changes
    let ships_to_update: Vec<Uuid> = if promote_config.ships.is_empty() {
        // All modified ships in playtest (that exist in live)
        instance.ships.iter()
            .filter(|s| state.ships.contains_key(&s.id))
            .map(|s| s.id)
            .collect()
    } else {
        promote_config.ships.clone()
    };

    for ship_id in ships_to_update {
        if let Some(playtest_ship) = instance.ships.get(&ship_id) {
            if let Some(mut live_ship) = state.ships.get_mut(&ship_id) {
                // Validate and clamp values before applying
                let hull = playtest_ship.hull_integrity.clamp(0.0, 100.0);
                let shield = playtest_ship.shield_strength.clamp(0.0, 100.0);

                // Skip if ship was destroyed in playtest (hull <= 0)
                if hull <= 0.0 {
                    errors.push(format!("Ship {} was destroyed in playtest, skipping", ship_id));
                    continue;
                }

                // Check if ship changed sectors - skip position/status if so to prevent data corruption
                let sector_changed = playtest_ship.sector_id != live_ship.sector_id;
                if sector_changed {
                    errors.push(format!("Ship {} changed sectors in playtest, skipping position/status", ship_id));
                }

                // Copy relevant fields from playtest to live with validation
                live_ship.hull_integrity = hull;
                live_ship.shield_strength = shield;
                live_ship.resources = playtest_ship.resources.clone();
                live_ship.crew = playtest_ship.crew.clone();
                live_ship.combat_stance = playtest_ship.combat_stance;
                live_ship.cargo = playtest_ship.cargo.clone();
                live_ship.upgrades = playtest_ship.upgrades.clone();

                // Only promote position/status if sector didn't change
                if !sector_changed {
                    live_ship.position = playtest_ship.position;
                    live_ship.status = playtest_ship.status.clone();
                }
                applied_count += 1;
            } else {
                errors.push(format!("Ship {} not found in live state", ship_id));
            }
        }
    }

    // Promote player changes
    let players_to_update: Vec<Uuid> = if promote_config.players.is_empty() {
        // All modified players in playtest (that exist in live)
        instance.player_data.iter()
            .filter(|p| state.player_data.contains_key(&p.id))
            .map(|p| p.id)
            .collect()
    } else {
        promote_config.players.clone()
    };

    for player_id in players_to_update {
        if let Some(playtest_player) = instance.player_data.get(&player_id) {
            if let Some(mut live_player) = state.player_data.get_mut(&player_id) {
                // Copy relevant fields from playtest to live
                live_player.resources = playtest_player.resources.clone();
                live_player.credits = playtest_player.credits;
                applied_count += 1;
            } else {
                errors.push(format!("Player {} not found in live state", player_id));
            }
        }
    }

    // Spawn new entities created in playtest
    if promote_config.spawn_new_entities {
        for entry in instance.created_ship_ids.iter() {
            let ship_id = *entry.key();
            if let Some(playtest_ship) = instance.ships.get(&ship_id) {
                // Verify the target sector exists to prevent orphaned ships
                if !state.sectors.contains_key(&playtest_ship.sector_id) {
                    errors.push(format!(
                        "Cannot spawn ship {} - sector {} doesn't exist in live",
                        ship_id, playtest_ship.sector_id
                    ));
                    continue;
                }

                let ship_clone = (*playtest_ship).clone();
                state.ships.insert(ship_id, ship_clone.clone());

                // Add to sector (guaranteed to exist from check above)
                if let Some(sector) = state.sectors.get(&ship_clone.sector_id) {
                    sector.ship_ids.insert(ship_id, ());
                }
                applied_count += 1;
            }
        }
    }

    // Apply deletions
    if promote_config.apply_deletions {
        for entry in instance.deleted_ship_ids.iter() {
            let ship_id = *entry.key();
            if let Some((_, ship)) = state.ships.remove(&ship_id) {
                // Remove from sector
                if let Some(sector) = state.sectors.get(&ship.sector_id) {
                    sector.ship_ids.remove(&ship_id);
                }
                applied_count += 1;
            }
        }
    }

    tracing::info!(
        playtest_id = %playtest_id,
        applied_count = applied_count,
        error_count = errors.len(),
        "Promoted playtest changes to live"
    );

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PromoteResult {
        success: errors.is_empty(),
        applied_count,
        errors,
    })).await;
}

async fn handle_preview_promote(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    playtest_id: Uuid,
    promote_config: PromoteConfigDto,
) {
    // Check authorization
    if !state.playtest_manager.is_authorized(playtest_id, player_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "UNAUTHORIZED".to_string(),
            message: "Only the playtest owner can preview promotion".to_string(),
        })).await;
        return;
    }

    // Get the playtest instance
    let Some(instance) = state.playtest_manager.get(playtest_id) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
            code: "NOT_FOUND".to_string(),
            message: "Playtest not found".to_string(),
        })).await;
        return;
    };

    // Count ships that would be updated
    let ships_to_update = if promote_config.ships.is_empty() {
        // Count all ships in playtest that exist in live
        instance.ships.iter()
            .filter(|s| state.ships.contains_key(&s.id))
            .count()
    } else {
        // Count specified ships that exist in both playtest and live
        promote_config.ships.iter()
            .filter(|id| instance.ships.contains_key(id) && state.ships.contains_key(id))
            .count()
    };

    // Count players that would be updated
    let players_to_update = if promote_config.players.is_empty() {
        // Count all players in playtest that exist in live
        instance.player_data.iter()
            .filter(|p| state.player_data.contains_key(&p.id))
            .count()
    } else {
        // Count specified players that exist in both playtest and live
        promote_config.players.iter()
            .filter(|id| instance.player_data.contains_key(id) && state.player_data.contains_key(id))
            .count()
    };

    // Count new entities that would be spawned
    let new_entities = if promote_config.spawn_new_entities {
        instance.created_ship_ids.len()
    } else {
        0
    };

    // Count deletions that would be applied
    let deletions = if promote_config.apply_deletions {
        instance.deleted_ship_ids.len()
    } else {
        0
    };

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::PromotePreview {
        ships_to_update,
        players_to_update,
        new_entities,
        deletions,
    })).await;
}

// =============================================================================
// Debug Operations
// =============================================================================

/// Track which player has which debug session.
/// This is a simple in-memory mapping stored in the handler context.
/// For a more robust solution, this could be stored in GameState.
static DEBUG_SESSIONS: std::sync::LazyLock<dashmap::DashMap<Uuid, Uuid>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);

async fn handle_start_debug_session(
    state: Arc<GameState>,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    target_dto: DebugTargetDto,
) {
    // Check if player already has a debug session
    if DEBUG_SESSIONS.contains_key(&player_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Debug session already active. End it first.".to_string(),
        })).await;
        return;
    }

    // Convert DTO to internal type and validate
    let target = match target_dto {
        DebugTargetDto::Playtest(playtest_id) => {
            // Validate playtest exists and player has access
            match state.playtest_manager.get(playtest_id) {
                Some(pt) if pt.is_owner(player_id) || pt.is_participant(player_id) => {
                    DebugTarget::Playtest(playtest_id)
                }
                Some(_) => {
                    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
                        message: "You don't have access to debug this playtest".to_string(),
                    })).await;
                    return;
                }
                None => {
                    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
                        message: format!("Playtest {} not found", playtest_id),
                    })).await;
                    return;
                }
            }
        }
        DebugTargetDto::Live => DebugTarget::Live,
    };

    // Start session
    let (session_id, mut pause_rx) = state.debug_controller.start_session(player_id, target);
    DEBUG_SESSIONS.insert(player_id, session_id);

    // Spawn a task to forward pause events to the client
    let tx_clone = tx.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Ok(paused_state) = pause_rx.recv().await {
            // Store the paused state in the session for later queries
            state_clone.debug_controller.set_paused(session_id, Some(paused_state.clone()));

            let msg = ServerMessage::Admin(AdminServerMessage::DebugPaused {
                script: paused_state.script_path.clone(),
                line: paused_state.line,
                column: paused_state.column,
                reason: pause_reason_to_dto(&paused_state.reason),
                call_stack: paused_state.call_stack.iter().map(|f| StackFrameDto {
                    index: f.index,
                    function_name: f.function_name.clone(),
                    source: f.source.clone(),
                    line: f.line,
                    column: f.column,
                }).collect(),
                entity_context: paused_state.entity_context.as_ref().map(|ctx| {
                    bw_shared::dto::EntityContextDto {
                        entity_type: ctx.entity_type.clone(),
                        entity_id: ctx.entity_id,
                        entity_name: ctx.entity_name.clone(),
                        sector_id: ctx.sector_id,
                    }
                }),
            });
            if tx_clone.send(msg).await.is_err() {
                break;
            }
        }
    });

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugSessionStarted {
        session_id,
    })).await;

    tracing::info!(player_id = %player_id, session_id = %session_id, "Debug session started");
}

async fn handle_end_debug_session(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
) {
    if let Some((_, session_id)) = DEBUG_SESSIONS.remove(&player_id) {
        state.debug_controller.end_session(session_id);

        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugSessionEnded)).await;
        tracing::info!(player_id = %player_id, session_id = %session_id, "Debug session ended");
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
    }
}

async fn handle_set_breakpoint(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    script: String,
    line: usize,
    condition: Option<String>,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    let mut bp = Breakpoint::at_line(&script, line);
    if let Some(ref cond) = condition {
        bp = bp.with_condition(cond.clone());
    }
    let bp_id = bp.id;

    if let Some(_) = state.debug_controller.set_breakpoint(session_id, bp) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::BreakpointSet {
            breakpoint: BreakpointDto {
                id: bp_id,
                script,
                line,
                column: None,
                condition,
                hit_count: 0,
                enabled: true,
            },
        })).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Failed to set breakpoint".to_string(),
        })).await;
    }
}

async fn handle_set_function_breakpoint(
    _state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    function_name: String,
    break_on_entry: bool,
    break_on_exit: bool,
) {
    let Some(_session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    // Function breakpoints require Rhai's AtFunctionName or AtFunctionCall breakpoint types
    // For now, just acknowledge - full implementation would need breakpoint type extensions
    let bp_id = Uuid::new_v4();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::FunctionBreakpointSet {
        breakpoint: FunctionBreakpointDto {
            id: bp_id,
            function_name,
            break_on_entry,
            break_on_exit,
            enabled: true,
        },
    })).await;
}

async fn handle_remove_breakpoint(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    breakpoint_id: Uuid,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    if state.debug_controller.remove_breakpoint(session_id, breakpoint_id) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::BreakpointRemoved {
            breakpoint_id,
        })).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Breakpoint not found".to_string(),
        })).await;
    }
}

async fn handle_toggle_breakpoint(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    breakpoint_id: Uuid,
    enabled: bool,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    if state.debug_controller.set_breakpoint_enabled(session_id, breakpoint_id, enabled) {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::BreakpointToggled {
            breakpoint_id,
            enabled,
        })).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Breakpoint not found".to_string(),
        })).await;
    }
}

async fn handle_list_breakpoints(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    let bps = state.debug_controller.get_breakpoints(session_id);
    let breakpoints: Vec<BreakpointDto> = bps.iter().map(|bp| BreakpointDto {
        id: bp.id,
        script: bp.source.clone(),
        line: bp.line,
        column: bp.column,
        condition: bp.condition.clone(),
        hit_count: bp.hit_count,
        enabled: bp.enabled,
    }).collect();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::BreakpointList {
        breakpoints,
        function_breakpoints: vec![], // Function breakpoints not yet implemented
    })).await;
}

async fn handle_debug_continue(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    if state.debug_controller.send_command(session_id, DebugCommand::Continue) {
        // Clear the paused state since we're resuming
        state.debug_controller.set_paused(session_id, None);
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugResumed)).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Failed to send continue command".to_string(),
        })).await;
    }
}

async fn handle_debug_pause(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    if state.debug_controller.send_command(session_id, DebugCommand::Pause) {
        // Pause will be acknowledged when the debugger actually pauses
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugResumed)).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Failed to send pause command".to_string(),
        })).await;
    }
}

async fn handle_debug_step(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    command: DebugCommand,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    if state.debug_controller.send_command(session_id, command) {
        // Clear the paused state since execution will resume (until next pause point)
        state.debug_controller.set_paused(session_id, None);
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugResumed)).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Failed to send step command".to_string(),
        })).await;
    }
}

async fn handle_get_variables(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    frame_index: usize,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    // Get the paused state to access variables
    if let Some(paused) = state.debug_controller.get_paused_state(session_id) {
        let variables: Vec<VariableDto> = paused.local_variables.iter().map(|v| VariableDto {
            name: v.name.clone(),
            value: v.value.clone(),
            type_name: v.type_name.clone(),
            expandable: v.expandable,
            path: v.path.clone(),
        }).collect();

        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugVariables {
            frame_index,
            variables,
        })).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Not currently paused".to_string(),
        })).await;
    }
}

async fn handle_expand_variable(
    _state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    variable_path: String,
) {
    let Some(_session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    // Variable expansion requires access to the actual Rhai Dynamic values
    // which are not currently persisted in PausedState.
    // For now, return an error - this would need rhai_integration.rs changes.
    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
        message: format!("Variable expansion not yet implemented for path: {}", variable_path),
    })).await;
}

async fn handle_evaluate_expression(
    _state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
    expression: String,
    frame_index: Option<usize>,
) {
    let Some(_session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    // Expression evaluation requires sending a command to the paused script context
    // For now, return an error - this would need rhai_integration.rs changes.
    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::EvaluationResult {
        expression,
        result: String::new(),
        type_name: String::new(),
        success: false,
        error: Some("Expression evaluation not yet implemented".to_string()),
    })).await;
    let _ = frame_index; // Silence unused warning
}

async fn handle_get_call_stack(
    state: &GameState,
    tx: &mpsc::Sender<ServerMessage>,
    player_id: Uuid,
) {
    let Some(session_id) = DEBUG_SESSIONS.get(&player_id).map(|r| *r) else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "No active debug session".to_string(),
        })).await;
        return;
    };

    // Get the paused state to access call stack
    if let Some(paused) = state.debug_controller.get_paused_state(session_id) {
        let call_stack: Vec<StackFrameDto> = paused.call_stack.iter().map(|f| StackFrameDto {
            index: f.index,
            function_name: f.function_name.clone(),
            source: f.source.clone(),
            line: f.line,
            column: f.column,
        }).collect();

        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::CallStack {
            frames: call_stack,
        })).await;
    } else {
        let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DebugError {
            message: "Not currently paused".to_string(),
        })).await;
    }
}

/// Convert internal PauseReason to DTO
fn pause_reason_to_dto(reason: &PauseReason) -> PauseReasonDto {
    match reason {
        PauseReason::Breakpoint { breakpoint_id } => PauseReasonDto::Breakpoint {
            breakpoint_id: *breakpoint_id,
        },
        PauseReason::FunctionEntry { function_name } => PauseReasonDto::FunctionEntry {
            function_name: function_name.clone(),
        },
        PauseReason::FunctionExit { function_name } => PauseReasonDto::FunctionExit {
            function_name: function_name.clone(),
        },
        PauseReason::Step => PauseReasonDto::Step,
        PauseReason::Exception { message } => PauseReasonDto::Exception {
            message: message.clone(),
        },
        PauseReason::Pause => PauseReasonDto::Pause,
    }
}

// =============================================================================
// Schema Introspection Handlers
// =============================================================================

/// Get all archetype schemas (summary)
async fn handle_get_archetype_schemas(tx: &mpsc::Sender<ServerMessage>) {
    let schemas = vec![
        ArchetypeSchemaDto {
            archetype_type: "ship".to_string(),
            name: SHIP_SCHEMA.name.to_string(),
            field_count: SHIP_SCHEMA.fields.len(),
            required_count: SHIP_SCHEMA.required_fields().count(),
        },
        ArchetypeSchemaDto {
            archetype_type: "weapon".to_string(),
            name: WEAPON_SCHEMA.name.to_string(),
            field_count: WEAPON_SCHEMA.fields.len(),
            required_count: WEAPON_SCHEMA.required_fields().count(),
        },
        ArchetypeSchemaDto {
            archetype_type: "effect".to_string(),
            name: EFFECT_SCHEMA.name.to_string(),
            field_count: EFFECT_SCHEMA.fields.len(),
            required_count: EFFECT_SCHEMA.required_fields().count(),
        },
        ArchetypeSchemaDto {
            archetype_type: "cargo".to_string(),
            name: CARGO_SCHEMA.name.to_string(),
            field_count: CARGO_SCHEMA.fields.len(),
            required_count: CARGO_SCHEMA.required_fields().count(),
        },
        ArchetypeSchemaDto {
            archetype_type: "ability".to_string(),
            name: ABILITY_SCHEMA.name.to_string(),
            field_count: ABILITY_SCHEMA.fields.len(),
            required_count: ABILITY_SCHEMA.required_fields().count(),
        },
        ArchetypeSchemaDto {
            archetype_type: "faction".to_string(),
            name: FACTION_SCHEMA.name.to_string(),
            field_count: FACTION_SCHEMA.fields.len(),
            required_count: FACTION_SCHEMA.required_fields().count(),
        },
    ];

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ArchetypeSchemas { schemas })).await;
}

/// Get detailed schema for a specific archetype type
async fn handle_get_archetype_schema(tx: &mpsc::Sender<ServerMessage>, archetype_type: &str) {
    let registry = DefinitionSchemaRegistry::with_builtins();

    let schema = match registry.get(archetype_type) {
        Some(s) => s,
        None => {
            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::AdminError {
                code: "UNKNOWN_ARCHETYPE".to_string(),
                message: format!("Unknown archetype type: {}", archetype_type),
            })).await;
            return;
        }
    };

    let fields: Vec<FieldSchemaDto> = schema.fields.iter().map(|f| {
        FieldSchemaDto {
            name: f.name.to_string(),
            field_type: f.field_type.name().to_string(),
            rust_type: match f.field_type {
                DefFieldType::String => "String".to_string(),
                DefFieldType::Number => "f64".to_string(),
                DefFieldType::Bool => "bool".to_string(),
                DefFieldType::Array => "Vec<Dynamic>".to_string(),
                DefFieldType::Map => "Map".to_string(),
                DefFieldType::Any => "Dynamic".to_string(),
            },
            required: f.required,
            description: None,
        }
    }).collect();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ArchetypeSchemaDetail {
        archetype_type: archetype_type.to_string(),
        name: schema.name.to_string(),
        fields,
    })).await;
}

/// Get all action parameter schemas
async fn handle_get_action_schemas(tx: &mpsc::Sender<ServerMessage>) {
    let scripts_dir = config().get().scripting.scripts_dir.clone();
    let schema_path = Path::new(&scripts_dir).parent()
        .unwrap_or(Path::new("."))
        .join("scripts")
        .join("schemas")
        .join("actions.toml");

    let actions = match ActionSchemaRegistry::load_from_file(&schema_path) {
        Ok(registry) => {
            registry.actions.into_iter().map(|(name, schema)| {
                let params = schema.params.into_iter().map(|(param_name, param)| {
                    (param_name, ParamSchemaDto {
                        param_type: param.param_type,
                        required: param.required,
                    })
                }).collect();

                (name, ActionSchemaDto {
                    description: schema.description,
                    params,
                })
            }).collect()
        }
        Err(e) => {
            tracing::warn!("Failed to load action schemas: {}", e);
            std::collections::HashMap::new()
        }
    };

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ActionSchemas { actions })).await;
}

// =============================================================================
// Validation Handlers
// =============================================================================

/// Validate a script without saving
async fn handle_validate_script(tx: &mpsc::Sender<ServerMessage>, path: &str, content: &str) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Compile the script with Rhai to catch syntax errors
    let engine = rhai::Engine::new();
    if let Err(e) = engine.compile(content) {
        let (line, col) = match e.position() {
            rhai::Position::NONE => (1, 1),
            pos => (pos.line().unwrap_or(1), pos.position().unwrap_or(1)),
        };

        errors.push(ValidationIssueDto {
            severity: ValidationSeverity::Error,
            message: e.to_string(),
            line,
            column: col,
            end_line: None,
            end_column: None,
            code: Some("SYNTAX_ERROR".to_string()),
            suggestion: None,
        });
    }

    // Basic static analysis
    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;

        if line.contains("print(") || line.contains("debug(") {
            warnings.push(ValidationIssueDto {
                severity: ValidationSeverity::Warning,
                message: "Debug print statement found".to_string(),
                line: line_num,
                column: line.find("print(").or_else(|| line.find("debug(")).unwrap_or(0) + 1,
                end_line: None,
                end_column: None,
                code: Some("DEBUG_PRINT".to_string()),
                suggestion: Some("Remove debug statements before production".to_string()),
            });
        }

        if line.contains("TODO") || line.contains("FIXME") {
            warnings.push(ValidationIssueDto {
                severity: ValidationSeverity::Info,
                message: "TODO/FIXME comment found".to_string(),
                line: line_num,
                column: line.find("TODO").or_else(|| line.find("FIXME")).unwrap_or(0) + 1,
                end_line: None,
                end_column: None,
                code: Some("TODO_COMMENT".to_string()),
                suggestion: None,
            });
        }
    }

    let is_valid = errors.is_empty();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::ValidationResult {
        path: path.to_string(),
        errors,
        warnings,
        is_valid,
    })).await;
}

/// Validate an archetype definition
async fn handle_validate_definition(tx: &mpsc::Sender<ServerMessage>, definition_type: &str, content: &str) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let registry = DefinitionSchemaRegistry::with_builtins();
    let schema = match registry.get(definition_type) {
        Some(s) => s,
        None => {
            errors.push(ValidationIssueDto {
                severity: ValidationSeverity::Error,
                message: format!("Unknown definition type: {}", definition_type),
                line: 1,
                column: 1,
                end_line: None,
                end_column: None,
                code: Some("UNKNOWN_TYPE".to_string()),
                suggestion: Some("Valid types: ship, weapon, effect, cargo, ability, faction".to_string()),
            });

            let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DefinitionValidationResult {
                definition_type: definition_type.to_string(),
                errors,
                warnings,
                is_valid: false,
            })).await;
            return;
        }
    };

    let engine = rhai::Engine::new();
    match engine.compile(content) {
        Ok(ast) => {
            let mut scope = rhai::Scope::new();
            match engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast) {
                Ok(result) => {
                    if let Some(map) = result.try_cast::<rhai::Map>() {
                        for field_name in schema.required_fields() {
                            if !map.contains_key(field_name) {
                                errors.push(ValidationIssueDto {
                                    severity: ValidationSeverity::Error,
                                    message: format!("Missing required field: {}", field_name),
                                    line: 1,
                                    column: 1,
                                    end_line: None,
                                    end_column: None,
                                    code: Some("MISSING_FIELD".to_string()),
                                    suggestion: Some(format!("Add '{}' to the definition", field_name)),
                                });
                            }
                        }

                        for key in map.keys() {
                            let key_str = key.to_string();
                            if !schema.has_field(&key_str) {
                                warnings.push(ValidationIssueDto {
                                    severity: ValidationSeverity::Warning,
                                    message: format!("Unknown field: {}", key_str),
                                    line: 1,
                                    column: 1,
                                    end_line: None,
                                    end_column: None,
                                    code: Some("UNKNOWN_FIELD".to_string()),
                                    suggestion: None,
                                });
                            }
                        }
                    } else {
                        errors.push(ValidationIssueDto {
                            severity: ValidationSeverity::Error,
                            message: "Definition must return an object/map".to_string(),
                            line: 1,
                            column: 1,
                            end_line: None,
                            end_column: None,
                            code: Some("INVALID_TYPE".to_string()),
                            suggestion: Some("Wrap your definition in #{ ... }".to_string()),
                        });
                    }
                }
                Err(e) => {
                    errors.push(ValidationIssueDto {
                        severity: ValidationSeverity::Error,
                        message: format!("Evaluation error: {}", e),
                        line: 1,
                        column: 1,
                        end_line: None,
                        end_column: None,
                        code: Some("EVAL_ERROR".to_string()),
                        suggestion: None,
                    });
                }
            }
        }
        Err(e) => {
            let (line, col) = match e.position() {
                rhai::Position::NONE => (1, 1),
                pos => (pos.line().unwrap_or(1), pos.position().unwrap_or(1)),
            };

            errors.push(ValidationIssueDto {
                severity: ValidationSeverity::Error,
                message: e.to_string(),
                line,
                column: col,
                end_line: None,
                end_column: None,
                code: Some("SYNTAX_ERROR".to_string()),
                suggestion: None,
            });
        }
    }

    let is_valid = errors.is_empty();

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::DefinitionValidationResult {
        definition_type: definition_type.to_string(),
        errors,
        warnings,
        is_valid,
    })).await;
}

// =============================================================================
// State Introspection Handlers (Phase 2)
// =============================================================================

/// Get a snapshot of state for entity type
async fn handle_get_state_snapshot(
    state: &Arc<GameState>,
    tx: &mpsc::Sender<ServerMessage>,
    entity_type: EntityType,
    limit: usize,
    offset: usize,
) {
    let tick = state.get_tick();
    let mut entities = Vec::new();
    let mut total_count = 0usize;

    match entity_type {
        EntityType::Ship => {
            let all_ships: Vec<_> = state.ships.iter().collect();
            total_count = all_ships.len();
            for ship in all_ships.into_iter().skip(offset).take(limit) {
                entities.push(serde_json::json!({
                    "id": ship.id,
                    "name": &ship.name,
                    "sector_id": ship.sector_id,
                    "status": format!("{:?}", ship.status),
                    "ship_class": format!("{:?}", ship.ship_class),
                    "hull_integrity": ship.hull_integrity,
                    "is_player_ship": ship.is_player_ship,
                    "position": ship.position,
                }));
            }
        }
        EntityType::Player => {
            let all_players: Vec<_> = state.player_data.iter().collect();
            total_count = all_players.len();
            for player in all_players.into_iter().skip(offset).take(limit) {
                entities.push(serde_json::json!({
                    "id": player.id,
                    "username": &player.username,
                    "is_online": player.is_online,
                    "reputation": player.resources.reputation,
                    "fame": player.resources.fame,
                    "active_ship_id": player.active_ship_id,
                }));
            }
        }
        EntityType::Sector => {
            let all_sectors: Vec<_> = state.sectors.iter().collect();
            total_count = all_sectors.len();
            for sector in all_sectors.into_iter().skip(offset).take(limit) {
                entities.push(serde_json::json!({
                    "id": sector.sector.id,
                    "name": &sector.sector.name,
                    "danger_level": format!("{:?}", sector.sector.danger_level),
                    "ship_count": sector.ship_ids.len(),
                    "location_count": sector.sector.locations.len(),
                }));
            }
        }
        EntityType::Mission => {
            let mut all_missions = Vec::new();
            for sector in state.sectors.iter() {
                for mission in sector.missions.iter() {
                    all_missions.push((sector.sector.id, mission.clone()));
                }
            }
            total_count = all_missions.len();
            for (sector_id, mission) in all_missions.into_iter().skip(offset).take(limit) {
                entities.push(serde_json::json!({
                    "id": mission.id,
                    "title": &mission.title,
                    "sector_id": sector_id,
                    "status": format!("{:?}", mission.status),
                    "mission_type": format!("{:?}", mission.mission_type),
                }));
            }
        }
        EntityType::Station => {
            let mut all_stations = Vec::new();
            for sector in state.sectors.iter() {
                for loc in &sector.sector.locations {
                    if loc.location_type.is_station() {
                        all_stations.push((sector.sector.id, loc.clone()));
                    }
                }
            }
            total_count = all_stations.len();
            for (sector_id, station) in all_stations.into_iter().skip(offset).take(limit) {
                entities.push(serde_json::json!({
                    "id": station.id,
                    "name": &station.name,
                    "sector_id": sector_id,
                    "services": station.services.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>(),
                }));
            }
        }
    }

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::StateSnapshot {
        tick,
        entity_type,
        entities,
        total_count,
    })).await;
}

/// Create a watch expression
async fn handle_create_watch(
    tx: &mpsc::Sender<ServerMessage>,
    expression: String,
    name: Option<String>,
) {
    let watch = WatchDto {
        id: Uuid::new_v4(),
        expression,
        name,
        last_value: None,
        history_length: 0,
    };

    let _ = tx.send(ServerMessage::Admin(AdminServerMessage::WatchCreated { watch })).await;
}
