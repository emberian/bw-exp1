//! HTTP routes

mod api;
mod auth;

use axum::Router;
use std::sync::Arc;

use crate::GameState;

pub use api::*;
pub use auth::*;

/// Build the API router.
pub fn api_router() -> Router<Arc<GameState>> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/game", api::router())
}
