//! BLACKWING Server Entry Point

use std::sync::Arc;
use std::net::SocketAddr;

use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use bw_server::{GameState, Database, routes, ws, simulation};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bw_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting BLACKWING server...");

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./blackwing.db".to_string());
    tracing::info!("Connecting to database: {}", database_url);
    let db = Database::new(&database_url).await?;
    tracing::info!("Database connected and migrations applied");

    // Initialize game state
    let state = Arc::new(GameState::new(db).await?);

    // Load scripts
    tracing::info!("Loading game scripts...");
    if let Err(e) = state.scripts.load_all_scripts() {
        tracing::warn!("Failed to load some scripts: {}", e);
    }
    tracing::info!("Loaded {} scripts", state.scripts.loaded_scripts().len());

    // Initialize scripting systems (must be after Arc<GameState> is created)
    state.initialize_scripting();

    // Start hot-reload watcher (debug builds only)
    #[cfg(debug_assertions)]
    let _script_watcher = {
        match bw_server::scripting::ScriptWatcher::new("scripts", state.clone()) {
            Ok(watcher) => {
                tracing::info!("Script hot-reload enabled");
                Some(watcher)
            }
            Err(e) => {
                tracing::warn!("Failed to start script hot-reload: {}", e);
                None
            }
        }
    };

    // Start game loop
    let game_state = state.clone();
    tokio::spawn(async move {
        simulation::run_game_loop(game_state).await;
    });

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))
        // API routes
        .nest("/api", routes::api_router())
        // WebSocket
        .route("/ws", get(ws::ws_handler))
        // State
        .with_state(state)
        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
