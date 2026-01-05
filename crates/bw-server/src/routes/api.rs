//! Game API routes

use axum::{
    Router,
    Json,
    extract::{State, Path},
    routing::get,
    http::StatusCode,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use bw_shared::dto::*;
use crate::{GameState, auth::AuthExtractor};

pub fn router() -> Router<Arc<GameState>> {
    Router::new()
        .route("/factions", get(list_factions))
        .route("/sectors", get(list_sectors))
        .route("/sector/:id", get(get_sector))
        .route("/sector/:id/missions", get(list_sector_missions))
        .route("/player/me", get(get_current_player))
        .route("/player/:id", get(get_player))
        .route("/player/me/ships", get(get_my_ships))
        .route("/ship/:id", get(get_ship))
        .route("/squadrons", get(list_squadrons))
        .route("/squadron/:id", get(get_squadron))
}

#[derive(Serialize)]
pub struct FactionsResponse {
    factions: Vec<FactionDto>,
}

async fn list_factions(State(state): State<Arc<GameState>>) -> Json<FactionsResponse> {
    let factions: Vec<FactionDto> = state.factions.iter()
        .filter(|f| f.faction_type.is_playable())
        .map(|f| FactionDto {
            id: f.id,
            name: f.name.clone(),
            tag: f.tag.clone(),
            description: f.description.clone(),
            philosophy: f.philosophy.clone(),
            color: f.faction_type.color().to_string(),
            standing: 0,
            rank: "Neutral".to_string(),
        })
        .collect();

    Json(FactionsResponse { factions })
}

#[derive(Serialize)]
pub struct SectorsResponse {
    sectors: Vec<SectorSummary>,
}

#[derive(Serialize)]
pub struct SectorSummary {
    id: Uuid,
    name: String,
    danger_level: String,
    player_count: usize,
}

async fn list_sectors(State(state): State<Arc<GameState>>) -> Json<SectorsResponse> {
    let sectors: Vec<SectorSummary> = state.sectors.iter()
        .map(|s| SectorSummary {
            id: s.sector.id,
            name: s.sector.name.clone(),
            danger_level: format!("{:?}", s.sector.danger_level),
            player_count: s.player_count(),
        })
        .collect();

    Json(SectorsResponse { sectors })
}

async fn get_sector(
    State(state): State<Arc<GameState>>,
    Path(sector_id): Path<Uuid>,
) -> Json<Option<SectorDto>> {
    let sector = state.sectors.get(&sector_id).map(|s| {
        SectorDto {
            id: s.sector.id,
            name: s.sector.name.clone(),
            danger_level: format!("{:?}", s.sector.danger_level),
            controlling_faction: s.sector.controlling_faction.and_then(|fid| {
                state.factions.get(&fid).map(|f| f.tag.clone())
            }),
            locations: s.sector.locations.iter().map(|loc| {
                LocationDto {
                    id: loc.id,
                    name: loc.name.clone(),
                    location_type: format!("{:?}", loc.location_type),
                    position: PositionDto {
                        x: loc.position.x,
                        y: loc.position.y,
                        z: loc.position.z,
                    },
                    faction_tag: loc.faction_id.and_then(|fid| {
                        state.factions.get(&fid).map(|f| f.tag.clone())
                    }),
                    services: loc.services.iter().map(|s| format!("{:?}", s)).collect(),
                }
            }).collect(),
            adjacent_sectors: s.sector.adjacent_sectors.iter().filter_map(|sid| {
                state.sectors.get(sid).map(|adj| AdjacentSectorDto {
                    id: adj.sector.id,
                    name: adj.sector.name.clone(),
                    danger_level: format!("{:?}", adj.sector.danger_level),
                })
            }).collect(),
        }
    });

    Json(sector)
}

// ============================================================================
// Mission Routes
// ============================================================================

#[derive(Serialize)]
pub struct MissionsResponse {
    missions: Vec<MissionDto>,
}

async fn list_sector_missions(
    State(state): State<Arc<GameState>>,
    Path(sector_id): Path<Uuid>,
) -> Json<MissionsResponse> {
    let missions = state.sectors.get(&sector_id)
        .map(|sector| {
            sector.missions.iter()
                .map(|m| MissionDto {
                    id: m.id,
                    title: m.title.clone(),
                    description: m.description.clone(),
                    mission_type: format!("{:?}", m.mission_type),
                    status: format!("{:?}", m.status),
                    priority: format!("{:?}", m.priority),
                    reputation_reward: m.reputation_reward,
                    fame_reward: m.fame_reward,
                    is_high_profile: m.is_high_profile,
                    expires_in_seconds: m.expires_at.map(|e| {
                        let now = chrono::Utc::now();
                        if e > now { (e - now).num_seconds() as u32 } else { 0 }
                    }),
                    progress: m.progress,
                    can_accept: true, // Would need player context to properly check
                })
                .collect()
        })
        .unwrap_or_default();

    Json(MissionsResponse { missions })
}

// ============================================================================
// Player Routes
// ============================================================================

async fn get_current_player(
    auth: AuthExtractor,
    State(state): State<Arc<GameState>>,
) -> Result<Json<PlayerDto>, StatusCode> {
    let player = state.player_data.get(&auth.player_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let dto = PlayerDto {
        id: player.id,
        username: player.username.clone(),
        reputation: player.resources.reputation,
        fame: player.resources.fame,
        faction_tag: state.factions.get(&player.faction_id)
            .map(|f| f.tag.clone())
            .unwrap_or_default(),
        squadron_tag: player.squadron_id.and_then(|sq_id| {
            state.squadrons.get(&sq_id).map(|sq| sq.tag.clone())
        }),
        is_online: player.is_online,
    };

    Ok(Json(dto))
}

async fn get_player(
    _auth: AuthExtractor, // Require authentication to look up players
    State(state): State<Arc<GameState>>,
    Path(player_id): Path<Uuid>,
) -> Result<Json<PlayerDto>, StatusCode> {
    let player = state.player_data.get(&player_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let dto = PlayerDto {
        id: player.id,
        username: player.username.clone(),
        reputation: player.resources.reputation,
        fame: player.resources.fame,
        faction_tag: state.factions.get(&player.faction_id)
            .map(|f| f.tag.clone())
            .unwrap_or_default(),
        squadron_tag: player.squadron_id.and_then(|sq_id| {
            state.squadrons.get(&sq_id).map(|sq| sq.tag.clone())
        }),
        is_online: player.is_online,
    };

    Ok(Json(dto))
}

// ============================================================================
// Ship Routes
// ============================================================================

#[derive(Serialize)]
pub struct ShipsResponse {
    ships: Vec<ShipDto>,
}

async fn get_my_ships(
    auth: AuthExtractor,
    State(state): State<Arc<GameState>>,
) -> Json<ShipsResponse> {
    let ships: Vec<ShipDto> = state.ships.iter()
        .filter(|s| s.owner_id == Some(auth.player_id))
        .map(|s| ShipDto {
            id: s.id,
            name: s.name.clone(),
            owner_id: s.owner_id,
            ship_class: format!("{:?}", s.ship_class),
            position: PositionDto {
                x: s.position.x,
                y: s.position.y,
                z: s.position.z,
            },
            hull_percent: s.hull_integrity,
            shield_percent: s.shield_strength,
            status: format!("{:?}", s.status),
            faction_tag: s.faction_id.and_then(|fid| {
                state.factions.get(&fid).map(|f| f.tag.clone())
            }),
            is_player: true,
            is_hostile: false,
        })
        .collect();

    Json(ShipsResponse { ships })
}

async fn get_ship(
    _auth: AuthExtractor, // Require authentication to look up ships
    State(state): State<Arc<GameState>>,
    Path(ship_id): Path<Uuid>,
) -> Result<Json<ShipDto>, StatusCode> {
    let ship = state.ships.get(&ship_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let dto = ShipDto {
        id: ship.id,
        name: ship.name.clone(),
        owner_id: ship.owner_id,
        ship_class: format!("{:?}", ship.ship_class),
        position: PositionDto {
            x: ship.position.x,
            y: ship.position.y,
            z: ship.position.z,
        },
        hull_percent: ship.hull_integrity,
        shield_percent: ship.shield_strength,
        status: format!("{:?}", ship.status),
        faction_tag: ship.faction_id.and_then(|fid| {
            state.factions.get(&fid).map(|f| f.tag.clone())
        }),
        is_player: ship.is_player_ship,
        is_hostile: ship.ship_class.is_hostile(),
    };

    Ok(Json(dto))
}

// ============================================================================
// Squadron Routes
// ============================================================================

#[derive(Serialize)]
pub struct SquadronsResponse {
    squadrons: Vec<SquadronSummary>,
}

#[derive(Serialize)]
pub struct SquadronSummary {
    id: Uuid,
    name: String,
    tag: String,
    member_count: usize,
    leader_name: String,
}

async fn list_squadrons(
    State(state): State<Arc<GameState>>,
) -> Json<SquadronsResponse> {
    let squadrons: Vec<SquadronSummary> = state.squadrons.iter()
        .map(|sq| {
            let leader_name = state.player_data.get(&sq.leader_id)
                .map(|p| p.username.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            SquadronSummary {
                id: sq.id,
                name: sq.name.clone(),
                tag: sq.tag.clone(),
                member_count: sq.member_count(),
                leader_name,
            }
        })
        .collect();

    Json(SquadronsResponse { squadrons })
}

async fn get_squadron(
    State(state): State<Arc<GameState>>,
    Path(squadron_id): Path<Uuid>,
) -> Result<Json<SquadronDto>, StatusCode> {
    let squadron = state.squadrons.get(&squadron_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let leader_name = state.player_data.get(&squadron.leader_id)
        .map(|p| p.username.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let dto = SquadronDto {
        id: squadron.id,
        name: squadron.name.clone(),
        tag: squadron.tag.clone(),
        motto: squadron.motto.clone(),
        leader_name,
        member_count: squadron.member_count() as u32,
        reputation_bonus: squadron.reputation_bonus,
        fame_bonus: squadron.fame_bonus,
        is_at_war: !squadron.hostile_squadrons.is_empty(),
    };

    Ok(Json(dto))
}
