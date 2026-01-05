//! Authentication routes
//!
//! Provides registration, login, logout, and token validation endpoints.

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use bw_core::models::*;

use crate::{
    auth::{generate_token, hash_password, hash_token, verify_password, AuthExtractor},
    state::GameState,
    PlayerSession,
};

/// Build the auth router.
pub fn router() -> Router<Arc<GameState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/validate", post(validate_token))
}

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub faction: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ship_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AuthResponse {
    fn success(player_id: Uuid, ship_id: Uuid, token: String) -> Self {
        Self {
            success: true,
            player_id: Some(player_id),
            ship_id: Some(ship_id),
            token: Some(token),
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            player_id: None,
            ship_id: None,
            token: None,
            error: Some(message.into()),
        }
    }
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<Uuid>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Register a new player account.
async fn register(
    State(state): State<Arc<GameState>>,
    Json(req): Json<RegisterRequest>,
) -> (StatusCode, Json<AuthResponse>) {
    // Validate username length
    if req.username.len() < 3 || req.username.len() > 24 {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse::error("Username must be 3-24 characters")),
        );
    }

    // Validate username characters (alphanumeric, underscore, hyphen)
    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse::error(
                "Username can only contain letters, numbers, underscores, and hyphens",
            )),
        );
    }

    // Validate password strength
    if req.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse::error("Password must be at least 8 characters")),
        );
    }

    // Check username availability
    match state.db.players().username_exists(&req.username).await {
        Ok(true) => {
            return (
                StatusCode::CONFLICT,
                Json(AuthResponse::error("Username already taken")),
            );
        }
        Err(e) => {
            tracing::error!("Database error checking username: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse::error("Internal server error")),
            );
        }
        _ => {}
    }

    // Find faction by name or tag
    let faction_id = state
        .factions
        .iter()
        .find(|f| {
            f.name.eq_ignore_ascii_case(&req.faction) || f.tag.eq_ignore_ascii_case(&req.faction)
        })
        .map(|f| f.id);

    let faction_id = match faction_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(AuthResponse::error("Invalid faction")),
            );
        }
    };

    // Hash password
    let password_hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Password hashing failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse::error("Internal server error")),
            );
        }
    };

    // Get starting sector
    let sector_id = state
        .sectors
        .iter()
        .next()
        .map(|s| s.sector.id)
        .unwrap_or_else(Uuid::new_v4);

    // Create ship first (need ship ID for player)
    let ship = Ship::new_player_ship(
        format!("{}'s Corvette", req.username),
        Uuid::new_v4(), // Temporary, will be updated
        ShipClass::PatrolCorvette,
        sector_id,
        faction_id,
    );
    let ship_id = ship.id;

    // Create player
    let player = Player::new(req.username.clone(), ship_id, sector_id, faction_id);
    let player_id = player.id;

    // Update ship owner
    let mut ship = ship;
    ship.owner_id = Some(player_id);

    // Persist to database
    if let Err(e) = state.db.players().insert(&player, &password_hash).await {
        tracing::error!("Failed to insert player: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse::error("Failed to create account")),
        );
    }

    if let Err(e) = state.db.ships().insert(&ship).await {
        tracing::error!("Failed to insert ship: {}", e);
        // Note: player was created, ideally would rollback here
    }

    // Generate session token
    let token = generate_token();
    if let Err(e) = state
        .db
        .sessions()
        .create(player_id, &token.hash, Duration::days(7))
        .await
    {
        tracing::error!("Failed to create session: {}", e);
        // Continue anyway - player can log in again
    }

    // Add to in-memory cache
    state.ships.insert(ship_id, ship);
    state.player_data.insert(player_id, player);
    state.players.insert(
        player_id,
        PlayerSession {
            player_id,
            ship_id,
            sector_id,
            connection: None,
        },
    );

    // Add ship to sector
    if let Some(sector) = state.sectors.get(&sector_id) {
        sector.ship_ids.insert(ship_id, ());
    }

    tracing::info!("New player registered: {} ({})", req.username, player_id);

    (
        StatusCode::CREATED,
        Json(AuthResponse::success(player_id, ship_id, token.raw)),
    )
}

/// Log in to an existing account.
async fn login(
    State(state): State<Arc<GameState>>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<AuthResponse>) {
    // Find player by username and get password hash
    let (player, password_hash) = match state
        .db
        .players()
        .find_by_username_with_password(&req.username)
        .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthResponse::error("Invalid username or password")),
            );
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse::error("Internal server error")),
            );
        }
    };

    // Verify password
    if !verify_password(&req.password, &password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse::error("Invalid username or password")),
        );
    }

    // Generate new session token
    let token = generate_token();
    if let Err(e) = state
        .db
        .sessions()
        .create(player.id, &token.hash, Duration::days(7))
        .await
    {
        tracing::error!("Failed to create session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse::error("Failed to create session")),
        );
    }

    let player_id = player.id;
    let ship_id = player.active_ship_id;
    let sector_id = player.patrol_sector_id;

    // Update online status
    let mut player = player;
    player.set_online(true);

    if let Err(e) = state.db.players().set_online(player_id, true).await {
        tracing::warn!("Failed to update online status: {}", e);
    }

    // Load into cache
    state.player_data.insert(player_id, player);

    // Load ship if not cached
    if !state.ships.contains_key(&ship_id) {
        if let Ok(Some(ship)) = state.db.ships().find_by_id(ship_id).await {
            state.ships.insert(ship_id, ship);
        }
    }

    // Create session tracking
    state.players.insert(
        player_id,
        PlayerSession {
            player_id,
            ship_id,
            sector_id,
            connection: None,
        },
    );

    tracing::info!("Player logged in: {}", player_id);

    (
        StatusCode::OK,
        Json(AuthResponse::success(player_id, ship_id, token.raw)),
    )
}

/// Log out and invalidate the session token.
async fn logout(
    State(state): State<Arc<GameState>>,
    Json(req): Json<LogoutRequest>,
) -> StatusCode {
    let token_hash = hash_token(&req.token);

    // Find and delete the session
    if let Ok(Some(session)) = state.db.sessions().find_by_token_hash(&token_hash).await {
        let _ = state.db.sessions().delete(session.id).await;

        // Mark player as offline
        if let Err(e) = state.db.players().set_online(session.player_id, false).await {
            tracing::warn!("Failed to update online status: {}", e);
        }

        // Update cache
        if let Some(mut player) = state.player_data.get_mut(&session.player_id) {
            player.set_online(false);
        }

        tracing::info!("Player logged out: {}", session.player_id);
    }

    StatusCode::OK
}

/// Validate a session token.
async fn validate_token(
    State(state): State<Arc<GameState>>,
    Json(req): Json<ValidateRequest>,
) -> Json<ValidateResponse> {
    let token_hash = hash_token(&req.token);

    match state.db.sessions().find_by_token_hash(&token_hash).await {
        Ok(Some(session)) if !session.is_expired() => Json(ValidateResponse {
            valid: true,
            player_id: Some(session.player_id),
        }),
        _ => Json(ValidateResponse {
            valid: false,
            player_id: None,
        }),
    }
}

/// Get the current authenticated player (protected route example).
pub async fn get_current_player(
    auth: AuthExtractor,
    State(state): State<Arc<GameState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get player from cache or database
    let player = if let Some(p) = state.player_data.get(&auth.player_id) {
        p.clone()
    } else if let Ok(Some(p)) = state.db.players().find_by_id(auth.player_id).await {
        state.player_data.insert(auth.player_id, p.clone());
        p
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Player not found" })),
        );
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": player.id,
            "username": player.username,
            "reputation": player.resources.reputation,
            "fame": player.resources.fame,
            "faction_id": player.faction_id,
            "active_ship_id": player.active_ship_id,
            "squadron_id": player.squadron_id,
        })),
    )
}
