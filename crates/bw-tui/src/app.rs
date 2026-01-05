//! Main application logic and event loop

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Frame, Terminal, prelude::*};
use tokio::sync::mpsc;

use crate::{
    Args,
    input::InputHandler,
    network::{message_handler, NetworkClient, NetworkEvent},
    state::{AppState, DebugState, GameState, UiState},
    ui,
};

/// Main application struct
pub struct App {
    /// Command line arguments
    args: Args,
    /// Combined application state
    pub state: AppState,
    /// Input handler
    input: InputHandler,
    /// Network client (initialized on run)
    network: Option<NetworkClient>,
    /// Channel to receive network events
    network_rx: Option<mpsc::UnboundedReceiver<NetworkEvent>>,
    /// Whether the app should quit
    should_quit: bool,
}

impl App {
    pub fn new(args: Args) -> Self {
        let debug_enabled = args.debug;
        Self {
            args,
            state: AppState {
                game: GameState::default(),
                ui: UiState::default(),
                debug: DebugState::new(debug_enabled),
            },
            input: InputHandler::new(),
            network: None,
            network_rx: None,
            should_quit: false,
        }
    }

    /// Main application loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Initialize network connection
        let (network, rx) = NetworkClient::new(
            &self.args.server,
            self.args.token.clone(),
        );
        self.network = Some(network);
        self.network_rx = Some(rx);

        // Start network connection in background
        if let Some(ref mut network) = self.network {
            network.connect().await?;
        }

        // Main event loop
        loop {
            // Cleanup expired notifications
            self.state.ui.cleanup_notifications();

            // Draw UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle events with timeout for responsiveness
            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    // Only handle key press events (not release)
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code, key.modifiers);
                    }
                }
            }

            // Process network events
            self.process_network_events();

            // Check if we should quit
            if self.should_quit {
                break;
            }
        }

        // Cleanup network
        if let Some(ref mut network) = self.network {
            network.disconnect().await;
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        ui::draw(frame, &self.state);
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Global quit handling
        if code == KeyCode::Char('q') && modifiers.is_empty() && !self.state.ui.is_input_mode() {
            self.should_quit = true;
            return;
        }

        // Ctrl+C always quits
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        // Delegate to input handler
        if let Some(action) = self.input.handle_key(code, modifiers, &mut self.state) {
            // Send action to network if connected
            if let Some(ref network) = self.network {
                let _ = network.send(action);
            }
        }
    }

    fn process_network_events(&mut self) {
        if let Some(ref mut rx) = self.network_rx {
            // Process all pending network events
            while let Ok(event) = rx.try_recv() {
                match event {
                    NetworkEvent::Connected => {
                        self.state.game.connected = true;
                        self.state.ui.add_notification("Connected to server".into());
                    }
                    NetworkEvent::Disconnected(reason) => {
                        self.state.game.connected = false;
                        self.state.ui.add_notification(format!("Disconnected: {}", reason));
                    }
                    NetworkEvent::Message(msg) => {
                        // Log raw message if debug enabled
                        if self.state.debug.show_messages {
                            self.state.debug.log_message(false, &msg);
                        }

                        // Check if message should trigger a notification
                        if let Some(notif) = message_handler::should_notify(&msg) {
                            self.state.ui.add_notification(notif);
                        }

                        // Process message
                        self.state.game.handle_server_message(msg);
                    }
                    NetworkEvent::Error(err) => {
                        self.state.ui.add_notification(format!("Error: {}", err));
                    }
                }
            }
        }
    }
}
