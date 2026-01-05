//! Authentication middleware for Axum
//!
//! Provides extractors that validate session tokens from the Authorization header.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use std::sync::Arc;
use uuid::Uuid;

use bw_auth::hash_token;
use crate::state::GameState;

/// Extractor that validates a session token and provides the authenticated player's info.
///
/// Usage in a handler:
/// ```ignore
/// async fn protected_route(
///     auth: AuthExtractor,
///     State(state): State<Arc<GameState>>,
/// ) -> impl IntoResponse {
///     // auth.player_id is the authenticated player's ID
///     // auth.session_id is the session ID
/// }
/// ```
pub struct AuthExtractor {
    /// The authenticated player's ID.
    pub player_id: Uuid,
    /// The session ID.
    pub session_id: Uuid,
}

impl FromRequestParts<Arc<GameState>> for AuthExtractor {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<GameState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract Bearer token from Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token_hash = hash_token(token);

        // Look up session in database
        let session = state
            .db
            .find_session(&token_hash)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Check if session is expired
        if session.is_expired() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(AuthExtractor {
            player_id: session.player_id,
            session_id: session.id,
        })
    }
}

/// Optional authentication extractor - doesn't reject if not authenticated.
///
/// Useful for routes that work both with and without authentication,
/// providing different responses based on auth status.
#[allow(dead_code)] // Public API for future routes
pub struct OptionalAuth {
    /// The authenticated player's ID, if present and valid.
    pub player_id: Option<Uuid>,
    /// The session ID, if present and valid.
    pub session_id: Option<Uuid>,
}

impl FromRequestParts<Arc<GameState>> for OptionalAuth {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<GameState>,
    ) -> Result<Self, Self::Rejection> {
        // Try to extract auth, but don't fail if missing
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        let Some(auth_header) = auth_header else {
            return Ok(OptionalAuth {
                player_id: None,
                session_id: None,
            });
        };

        let Some(token) = auth_header.strip_prefix("Bearer ") else {
            return Ok(OptionalAuth {
                player_id: None,
                session_id: None,
            });
        };

        let token_hash = hash_token(token);

        // Look up session
        let session = match state.db.find_session(&token_hash).await {
            Ok(Some(s)) if !s.is_expired() => s,
            _ => {
                return Ok(OptionalAuth {
                    player_id: None,
                    session_id: None,
                })
            }
        };

        Ok(OptionalAuth {
            player_id: Some(session.player_id),
            session_id: Some(session.id),
        })
    }
}
