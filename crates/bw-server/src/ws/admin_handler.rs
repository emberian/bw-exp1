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
    ValidationError, SimConfigSection,
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
