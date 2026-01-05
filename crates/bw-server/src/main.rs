//! BLACKWING Server Entry Point

use std::sync::Arc;
use std::net::SocketAddr;
use std::path::Path;

use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use bw_server::{
    GameState, Database, routes, ws, simulation,
    init_config, config, spawn_sighup_handler,
};

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

    // Load configuration
    // File watching for auto-reload is controlled by config, SIGHUP always works
    let config_manager = init_config("config.toml", true)?;
    let server_config = config().get();

    // Initialize database (env var overrides config)
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| server_config.server.database_url.clone())
        .unwrap_or_else(|| "sqlite:./blackwing.db".to_string());
    tracing::info!("Connecting to database: {}", database_url);
    let db = Database::new(&database_url).await?;
    tracing::info!("Database connected and migrations applied");

    // Initialize game state
    let state = Arc::new(GameState::new(db).await?);

    // Load scripts
    let scripts_dir = &server_config.scripting.scripts_dir;
    tracing::info!("Loading game scripts from {}...", scripts_dir);
    if let Err(e) = state.scripts.load_all_scripts() {
        tracing::warn!("Failed to load some scripts: {}", e);
    }
    tracing::info!("Loaded {} scripts", state.scripts.loaded_scripts().len());

    // Initialize scripting systems (must be after Arc<GameState> is created)
    state.initialize_scripting();

    // Start SIGHUP handler for manual reload (kill -HUP <pid>)
    spawn_sighup_handler(config_manager, state.clone());

    // Start file watcher for automatic hot-reload (if enabled in config)
    let _script_watcher = if server_config.scripting.hot_reload {
        match bw_server::scripting::ScriptWatcher::new(scripts_dir, state.clone()) {
            Ok(watcher) => {
                tracing::info!("Script hot-reload enabled (watching {})", scripts_dir);
                Some(watcher)
            }
            Err(e) => {
                tracing::warn!("Failed to start script hot-reload: {}", e);
                None
            }
        }
    } else {
        tracing::info!("Script hot-reload disabled (use SIGHUP to reload manually)");
        None
    };

    // Start game loop
    let game_state = state.clone();
    tokio::spawn(async move {
        simulation::run_game_loop(game_state).await;
    });

    // Build router
    let mut app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))
        // API routes
        .nest("/api", routes::api_router())
        // WebSocket
        .route("/ws", get(ws::ws_handler))
        // State
        .with_state(state);

    // Serve static frontend files if the directory exists
    let static_dir = Path::new("static");
    if static_dir.exists() {
        tracing::info!("Serving static files from ./static");
        let serve_dir = ServeDir::new(static_dir)
            .not_found_service(ServeFile::new(static_dir.join("index.html")));
        app = app.fallback_service(serve_dir);
    } else {
        tracing::info!("No static directory found, skipping static file serving");
    }

    // Apply middleware
    let app = app
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from((
        server_config.server.host.parse::<std::net::IpAddr>().unwrap_or([0, 0, 0, 0].into()),
        server_config.server.port,
    ));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
