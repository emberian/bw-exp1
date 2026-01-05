//! BLACKWING Server Entry Point

use std::sync::Arc;
use std::net::SocketAddr;
use std::path::Path;

use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower_http::services::{ServeDir, ServeFile};
use axum::http::{Method, HeaderValue};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use bw_server::{
    GameState, Database, routes, ws, simulation,
    init_config, config, spawn_sighup_handler,
};

use anyhow::Context;

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
    let config_manager = init_config("config.toml", true)
        .context("Failed to load config.toml")?;
    let server_config = config().get();

    // Initialize database (env var overrides config)
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| server_config.server.database_url.clone())
        .unwrap_or_else(|| "sqlite:./blackwing.db".to_string());
    tracing::info!("Connecting to database: {}", database_url);
    let db = Database::new(&database_url).await
        .context(format!("Failed to connect to database: {}", database_url))?;
    tracing::info!("Database connected and migrations applied");

    // Seed default admin user if it doesn't exist
    match db.seed_default_admin().await {
        Ok(true) => tracing::info!("Created default admin user (admin:hunter2)"),
        Ok(false) => tracing::debug!("Default admin user already exists"),
        Err(e) => tracing::warn!("Failed to seed default admin: {}", e),
    }

    // Initialize game state
    let state = Arc::new(GameState::new(db).await
        .context("Failed to initialize GameState")?);

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

    // Create shutdown channel for graceful termination
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start game loop with shutdown receiver
    let game_state = state.clone();
    let game_loop_handle = tokio::spawn(async move {
        simulation::run_game_loop(game_state, shutdown_rx).await;
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

    // Serve WASM app at /play (SPA with fallback to index.html)
    let play_dir = Path::new("play");
    if play_dir.exists() {
        tracing::info!("Serving WASM app from ./play at /play/*");
        let play_service = ServeDir::new(play_dir)
            .not_found_service(ServeFile::new(play_dir.join("index.html")));
        app = app.nest_service("/play", play_service);
    } else {
        tracing::info!("No play directory found, skipping WASM app serving");
    }

    // Serve static landing page at root
    let static_dir = Path::new("static");
    if static_dir.exists() {
        tracing::info!("Serving landing page from ./static");
        let static_service = ServeDir::new(static_dir)
            .not_found_service(ServeFile::new(static_dir.join("index.html")));
        app = app.fallback_service(static_service);
    } else {
        tracing::info!("No static directory found, skipping landing page serving");
    }

    // Build CORS layer based on configuration
    let cors = if server_config.server.cors_origins.is_empty() {
        tracing::warn!("CORS configured to allow all origins - this is insecure for production!");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any)
            .allow_credentials(false)
    } else {
        let origins: Vec<HeaderValue> = server_config.server.cors_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();
        tracing::info!("CORS allowed origins: {:?}", server_config.server.cors_origins);
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any)
            .allow_credentials(true)
    };

    // Apply middleware
    let app = app
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // Start server with graceful shutdown
    let addr = SocketAddr::from((
        server_config.server.host.parse::<std::net::IpAddr>().unwrap_or([0, 0, 0, 0].into()),
        server_config.server.port,
    ));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Run server with graceful shutdown on Ctrl+C
    // Use into_make_service_with_connect_info to enable IP-based rate limiting
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            tracing::info!("Received shutdown signal, initiating graceful shutdown...");

            // Signal game loop to stop
            let _ = shutdown_tx.send(true);
        })
        .await?;

    // Wait for game loop to finish
    tracing::info!("Waiting for game loop to finish...");
    let _ = game_loop_handle.await;
    tracing::info!("Server shutdown complete");

    Ok(())
}
