//! Game API routes

use axum::{
    Router,
    Json,
    extract::{State, Path},
    routing::get,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use bw_shared::dto::*;
use crate::GameState;

pub fn router() -> Router<Arc<GameState>> {
    Router::new()
        .route("/factions", get(list_factions))
        .route("/sectors", get(list_sectors))
        .route("/sector/:id", get(get_sector))
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
