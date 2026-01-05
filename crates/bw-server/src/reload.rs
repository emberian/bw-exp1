//! Hot-reload coordination
//!
//! Handles SIGHUP signals and coordinates reloading of config and scripts.

use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};

use crate::config::ConfigManager;
use crate::GameState;

/// Reload all watchable resources.
///
/// This is called on SIGHUP and can also be triggered programmatically.
pub fn reload_all(config: &Arc<ConfigManager>, state: &Arc<GameState>) {
    tracing::info!("Reloading all resources...");

    // Reload config
    config.reload();

    // Reload all scripts
    reload_scripts(state);

    tracing::info!("Reload complete");
}

/// Reload all scripts.
pub fn reload_scripts(state: &Arc<GameState>) {
    tracing::info!("Reloading all scripts...");

    match state.scripts.load_all_scripts() {
        Ok(()) => {
            let count = state.scripts.loaded_scripts().len();
            tracing::info!("Reloaded {} scripts", count);
            state.log_script_info(format!("Reloaded {} scripts", count));
        }
        Err(e) => {
            tracing::error!("Failed to reload scripts: {}", e);
            state.log_script_error(format!("Script reload failed: {}", e));
        }
    }
}

/// Spawn a task that listens for SIGHUP and triggers reload.
///
/// On Unix systems, SIGHUP is traditionally used to signal daemons to reload
/// their configuration. This allows graceful config updates without restart.
///
/// # Example
/// ```bash
/// # Reload server config and scripts
/// kill -HUP $(pidof blackwing-server)
/// ```
pub fn spawn_sighup_handler(config: Arc<ConfigManager>, state: Arc<GameState>) {
    tokio::spawn(async move {
        let mut sighup = match signal(SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to register SIGHUP handler: {}", e);
                return;
            }
        };

        tracing::info!("SIGHUP handler registered (kill -HUP to reload)");

        loop {
            sighup.recv().await;
            tracing::info!("Received SIGHUP, triggering reload...");
            reload_all(&config, &state);
        }
    });
}

#[cfg(test)]
mod tests {
    // Signal handling tests are tricky in unit tests
    // Integration tests would be more appropriate
}
