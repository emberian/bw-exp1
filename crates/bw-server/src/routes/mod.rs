//! HTTP routes

mod admin;
mod api;
pub mod auth;

use axum::Router;
use std::sync::Arc;

use crate::GameState;

/// Build the API router.
pub fn api_router() -> Router<Arc<GameState>> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/game", api::router())
        .nest("/admin", admin::router())
}
