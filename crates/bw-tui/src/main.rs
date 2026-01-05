//! Blackwing TUI - Terminal client for the Blackwing space strategy game
//!
//! A debug-friendly terminal interface with full gameplay support and GM tools.

mod app;
mod input;
mod network;
mod state;
mod ui;

use std::io;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::app::App;

/// Blackwing TUI - Terminal client for Blackwing
#[derive(Parser, Debug)]
#[command(name = "bw-tui")]
#[command(about = "Terminal UI client for Blackwing space strategy game")]
pub struct Args {
    /// Server address (e.g., ws://localhost:3000/ws)
    #[arg(short, long, default_value = "ws://localhost:3000/ws")]
    pub server: String,

    /// Authentication token (if not provided, will prompt)
    #[arg(short, long)]
    pub token: Option<String>,

    /// Enable debug panel on startup
    #[arg(short, long)]
    pub debug: bool,

    /// Log file path (if provided, logs to file instead of stderr)
    #[arg(long)]
    pub log_file: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    // When running TUI, we can't log to stderr as it corrupts the display
    // Use file logging or disable
    if let Some(log_path) = &args.log_file {
        let file = std::fs::File::create(log_path)?;
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_ansi(false);

        tracing_subscriber::registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
            .with(file_layer)
            .init();
    }

    // Run the async runtime
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run(args))
}

async fn run(args: Args) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Create and run app
    let mut app = App::new(args);
    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
