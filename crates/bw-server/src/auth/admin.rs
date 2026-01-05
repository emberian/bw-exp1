//! Admin authentication
//!
//! Provides an extractor that requires admin privileges.
//! Admin users are identified by the ADMIN_USERNAMES environment variable,
//! which should be a comma-separated list of usernames.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth::hash_token, state::GameState};

/// Extractor that validates a session token and requires admin privileges.
///
/// Admin users are determined by the `ADMIN_USERNAMES` environment variable,
/// which should contain a comma-separated list of usernames.
///
/// Usage in a handler:
/// ```ignore
/// async fn admin_only_route(
///     admin: AdminAuth,
///     State(state): State<Arc<GameState>>,
/// ) -> impl IntoResponse {
///     // Only admins can reach here
/// }
/// ```
pub struct AdminAuth {
    /// The authenticated admin player's ID.
    pub player_id: Uuid,
    /// The session ID.
    pub session_id: Uuid,
    /// The admin's username.
    pub username: String,
}

impl FromRequestParts<Arc<GameState>> for AdminAuth {
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
            .sessions()
            .find_by_token_hash(&token_hash)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Check if session is expired
        if session.is_expired() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // Get player to check admin status
        let player = state
            .player_data
            .get(&session.player_id)
            .map(|p| p.clone())
            .ok_or(StatusCode::FORBIDDEN)?;

        // Check if player is an admin
        if !is_admin(&player.username) {
            tracing::warn!(
                player_id = %session.player_id,
                username = %player.username,
                "Non-admin attempted to access admin endpoint"
            );
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(AdminAuth {
            player_id: session.player_id,
            session_id: session.id,
            username: player.username,
        })
    }
}

/// Check if a username is an admin.
///
/// Reads from the `ADMIN_USERNAMES` environment variable.
fn is_admin(username: &str) -> bool {
    // Get admin usernames from environment
    let admin_usernames = std::env::var("ADMIN_USERNAMES")
        .unwrap_or_default();

    // Parse comma-separated list
    admin_usernames
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|admin| admin == username.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_admin_with_env() {
        // SAFETY: Test-only, single-threaded access to env vars
        unsafe {
            std::env::set_var("ADMIN_USERNAMES", "admin, superuser, GM");
        }

        assert!(is_admin("admin"));
        assert!(is_admin("ADMIN")); // Case insensitive
        assert!(is_admin("superuser"));
        assert!(is_admin("gm"));
        assert!(!is_admin("regularuser"));
        assert!(!is_admin(""));

        // SAFETY: Test-only cleanup
        unsafe {
            std::env::remove_var("ADMIN_USERNAMES");
        }
    }

    #[test]
    fn test_is_admin_empty_env() {
        // SAFETY: Test-only, single-threaded access to env vars
        unsafe {
            std::env::remove_var("ADMIN_USERNAMES");
        }
        assert!(!is_admin("admin"));
    }
}
