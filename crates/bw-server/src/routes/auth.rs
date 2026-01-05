//! Authentication routes

use axum::{
    Router,
    Json,
    extract::State,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use bw_core::models::*;
use crate::GameState;

pub fn router() -> Router<Arc<GameState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    faction: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    success: bool,
    player_id: Option<Uuid>,
    ship_id: Option<Uuid>,
    token: Option<String>,
    error: Option<String>,
}

async fn register(
    State(state): State<Arc<GameState>>,
    Json(req): Json<RegisterRequest>,
) -> Json<AuthResponse> {
    // Validate username
    if req.username.len() < 3 || req.username.len() > 24 {
        return Json(AuthResponse {
            success: false,
            player_id: None,
            ship_id: None,
            token: None,
            error: Some("Username must be 3-24 characters".to_string()),
        });
    }

    // Find faction
    let faction_id = state.factions.iter()
        .find(|f| f.name.to_lowercase() == req.faction.to_lowercase()
            || f.tag.to_lowercase() == req.faction.to_lowercase())
        .map(|f| f.id);

    let faction_id = match faction_id {
        Some(id) => id,
        None => {
            return Json(AuthResponse {
                success: false,
                player_id: None,
                ship_id: None,
                token: None,
                error: Some("Invalid faction".to_string()),
            });
        }
    };

    // Get starting sector
    let sector_id = state.sectors.iter().next()
        .map(|s| s.sector.id)
        .unwrap_or_else(Uuid::new_v4);

    // Create ship
    let ship = Ship::new_player_ship(
        format!("{}'s Corvette", req.username),
        Uuid::new_v4(), // Will be updated with player ID
        ShipClass::PatrolCorvette,
        sector_id,
        faction_id,
    );
    let ship_id = ship.id;

    // Create player
    let player = Player::new(
        req.username,
        ship_id,
        sector_id,
        faction_id,
    );
    let player_id = player.id;

    // Update ship owner
    let mut ship = ship;
    ship.owner_id = Some(player_id);

    // Store in state
    state.ships.insert(ship_id, ship);

    // Store full player data
    state.player_data.insert(player_id, player);

    // Store player session (connection tracking)
    state.players.insert(player_id, crate::PlayerSession {
        player_id,
        ship_id,
        sector_id,
        connection: None,
    });

    // Add ship to sector
    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.ship_ids.insert(ship_id, ());
    }

    // Generate token (simplified - in production use JWT)
    let token = format!("{}:{}", player_id, ship_id);

    Json(AuthResponse {
        success: true,
        player_id: Some(player_id),
        ship_id: Some(ship_id),
        token: Some(token),
        error: None,
    })
}

#[derive(Deserialize)]
pub struct LoginRequest {
    token: String,
}

async fn login(
    State(state): State<Arc<GameState>>,
    Json(req): Json<LoginRequest>,
) -> Json<AuthResponse> {
    // Parse token (simplified)
    let parts: Vec<&str> = req.token.split(':').collect();
    if parts.len() != 2 {
        return Json(AuthResponse {
            success: false,
            player_id: None,
            ship_id: None,
            token: None,
            error: Some("Invalid token".to_string()),
        });
    }

    let player_id = match Uuid::parse_str(parts[0]) {
        Ok(id) => id,
        Err(_) => {
            return Json(AuthResponse {
                success: false,
                player_id: None,
                ship_id: None,
                token: None,
                error: Some("Invalid token".to_string()),
            });
        }
    };

    let ship_id = match Uuid::parse_str(parts[1]) {
        Ok(id) => id,
        Err(_) => {
            return Json(AuthResponse {
                success: false,
                player_id: None,
                ship_id: None,
                token: None,
                error: Some("Invalid token".to_string()),
            });
        }
    };

    // Verify player exists
    if !state.players.contains_key(&player_id) {
        return Json(AuthResponse {
            success: false,
            player_id: None,
            ship_id: None,
            token: None,
            error: Some("Player not found".to_string()),
        });
    }

    Json(AuthResponse {
        success: true,
        player_id: Some(player_id),
        ship_id: Some(ship_id),
        token: Some(req.token),
        error: None,
    })
}
