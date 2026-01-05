//! Script hot-reload system
//!
//! Watches the scripts directory for changes and automatically reloads
//! modified scripts without requiring a server restart.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use notify::{Watcher, RecursiveMode, Event, EventKind, Config, RecommendedWatcher};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::GameState;

/// Watches for script file changes and triggers reloads.
pub struct ScriptWatcher {
    /// Kept alive to maintain the file watch; dropping stops watching.
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    cancel_token: CancellationToken,
}

impl ScriptWatcher {
    /// Create a new script watcher.
    ///
    /// # Arguments
    /// * `scripts_dir` - Path to the scripts directory (relative or absolute)
    /// * `state` - Arc reference to the game state
    ///
    /// # Returns
    /// A `ScriptWatcher` that will continue watching until dropped or stopped.
    pub fn new(scripts_dir: impl Into<String>, state: Arc<GameState>) -> anyhow::Result<Self> {
        let scripts_dir = scripts_dir.into();
        let scripts_dir_clone = scripts_dir.clone();

        // Create async channel for file events (larger buffer to handle bursts)
        let (tx, mut rx) = mpsc::channel::<Event>(256);

        // Create watcher with custom config
        let config = Config::default()
            .with_poll_interval(Duration::from_millis(500));

        let watcher_tx = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    // Use blocking send since this is called from a non-async context
                    // Log if channel is full (indicates potential event loss)
                    if let Err(e) = watcher_tx.blocking_send(event) {
                        tracing::warn!("Script watcher channel error: {}", e);
                    }
                }
            },
            config,
        )?;

        // Watch the scripts directory
        watcher.watch(Path::new(&scripts_dir), RecursiveMode::Recursive)?;

        tracing::info!("Script hot-reload watching: {}", scripts_dir);

        // Create cancellation token for graceful shutdown
        let cancel_token = CancellationToken::new();
        let task_token = cancel_token.clone();

        // Spawn handler task
        tokio::spawn(async move {
            let debounce_duration = Duration::from_millis(200);
            let mut pending_reloads: Vec<String> = Vec::new();
            let mut last_event = std::time::Instant::now();

            loop {
                tokio::select! {
                    // Check for cancellation
                    _ = task_token.cancelled() => {
                        tracing::info!("Script watcher shutting down gracefully");
                        // Process any remaining pending reloads before exit
                        for script_path in pending_reloads.drain(..) {
                            Self::reload_script(&state, &script_path);
                        }
                        break;
                    }
                    // Wait for events with timeout
                    result = tokio::time::timeout(debounce_duration, rx.recv()) => {
                        match result {
                            Ok(Some(event)) => {
                                // Check if this is a modify or create event
                                if matches!(
                                    event.kind,
                                    EventKind::Modify(_) | EventKind::Create(_)
                                ) {
                                    for path in event.paths {
                                        // Only process .rhai files
                                        if path.extension().is_some_and(|e| e == "rhai") {
                                            // Convert to relative path
                                            let relative = path
                                                .strip_prefix(&scripts_dir_clone)
                                                .unwrap_or(&path)
                                                .to_string_lossy()
                                                .to_string();

                                            // Add to pending if not already there
                                            if !pending_reloads.contains(&relative) {
                                                pending_reloads.push(relative);
                                            }
                                            last_event = std::time::Instant::now();
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                // Channel closed, exit
                                tracing::debug!("Script watcher channel closed");
                                break;
                            }
                            Err(_) => {
                                // Timeout - check if we should process pending reloads
                                if !pending_reloads.is_empty() && last_event.elapsed() >= debounce_duration {
                                    for script_path in pending_reloads.drain(..) {
                                        Self::reload_script(&state, &script_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Self { watcher, cancel_token })
    }

    /// Stop the script watcher gracefully.
    ///
    /// This will cancel the background task and process any pending reloads
    /// before shutting down.
    pub fn stop(&self) {
        tracing::info!("Stopping script watcher");
        self.cancel_token.cancel();
    }

    /// Check if the watcher is still running.
    pub fn is_running(&self) -> bool {
        !self.cancel_token.is_cancelled()
    }

    /// Reload a single script.
    fn reload_script(state: &Arc<GameState>, script_path: &str) {
        tracing::info!("Hot-reloading script: {}", script_path);

        match state.scripts.load_script(script_path) {
            Ok(()) => {
                tracing::info!("Successfully reloaded: {}", script_path);
                state.log_script_info(format!("Hot-reloaded: {}", script_path));

                // Reinitialize if this is an action script (calls init() to re-register handlers)
                state.reinitialize_action_script(script_path);
            }
            Err(e) => {
                tracing::error!("Failed to reload {}: {}", script_path, e);
                state.log_script_error(format!("Reload failed for {}: {}", script_path, e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Hot-reload tests would require filesystem manipulation
    // which is better suited for integration tests
}
