//! Debug Controller
//!
//! Manages debug sessions, breakpoints, and communication channels
//! between the GM Editor and script execution.

use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;
use rhai::Engine;

use super::types::*;
use super::rhai_integration::register_debugger;

/// Channel capacity for debug events
const EVENT_CHANNEL_CAPACITY: usize = 64;
/// Channel capacity for debug commands
const COMMAND_CHANNEL_CAPACITY: usize = 16;

/// Central controller for all debug sessions.
pub struct DebugController {
    /// Active debug sessions by ID
    sessions: DashMap<Uuid, Arc<DebugSession>>,
    /// Pause event broadcasters by session ID
    pause_notifiers: DashMap<Uuid, broadcast::Sender<PausedState>>,
    /// Command senders by session ID
    command_senders: DashMap<Uuid, mpsc::Sender<DebugCommand>>,
    /// Command receivers by session ID (wrapped for thread-safe access)
    command_receivers: DashMap<Uuid, Arc<std::sync::Mutex<mpsc::Receiver<DebugCommand>>>>,
}

impl Default for DebugController {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugController {
    /// Create a new debug controller.
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            pause_notifiers: DashMap::new(),
            command_senders: DashMap::new(),
            command_receivers: DashMap::new(),
        }
    }

    /// Start a new debug session.
    ///
    /// Returns the session ID and a receiver for pause events.
    pub fn start_session(
        &self,
        owner_id: Uuid,
        target: DebugTarget,
    ) -> (Uuid, broadcast::Receiver<PausedState>) {
        let session = Arc::new(DebugSession::new(owner_id, target));
        let session_id = session.id;

        // Create channels
        let (pause_tx, pause_rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let (cmd_tx, cmd_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);

        // Store everything
        self.sessions.insert(session_id, session);
        self.pause_notifiers.insert(session_id, pause_tx);
        self.command_senders.insert(session_id, cmd_tx);
        self.command_receivers.insert(session_id, Arc::new(std::sync::Mutex::new(cmd_rx)));

        tracing::info!(session_id = %session_id, "Debug session started");

        (session_id, pause_rx)
    }

    /// End a debug session.
    pub fn end_session(&self, session_id: Uuid) {
        self.sessions.remove(&session_id);
        self.pause_notifiers.remove(&session_id);
        self.command_senders.remove(&session_id);
        self.command_receivers.remove(&session_id);

        tracing::info!(session_id = %session_id, "Debug session ended");
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: Uuid) -> Option<Arc<DebugSession>> {
        self.sessions.get(&session_id).map(|r| Arc::clone(&*r))
    }

    /// Get session by owner ID.
    pub fn get_session_by_owner(&self, owner_id: Uuid) -> Option<Arc<DebugSession>> {
        self.sessions
            .iter()
            .find(|r| r.owner_id == owner_id)
            .map(|r| Arc::clone(&*r))
    }

    /// Check if there's an active debug session for a target.
    pub fn has_session_for_target(&self, target: &DebugTarget) -> bool {
        self.sessions.iter().any(|r| &r.target == target && r.active)
    }

    /// Set a breakpoint in a session.
    pub fn set_breakpoint(&self, session_id: Uuid, breakpoint: Breakpoint) -> Option<Uuid> {
        let session = self.sessions.get(&session_id)?;
        let bp_id = breakpoint.id;
        session.add_breakpoint(breakpoint);
        tracing::debug!(session_id = %session_id, breakpoint_id = %bp_id, "Breakpoint set");
        Some(bp_id)
    }

    /// Remove a breakpoint from a session.
    pub fn remove_breakpoint(&self, session_id: Uuid, breakpoint_id: Uuid) -> bool {
        if let Some(session) = self.sessions.get(&session_id) {
            session.remove_breakpoint(breakpoint_id)
        } else {
            false
        }
    }

    /// Enable/disable a breakpoint.
    pub fn set_breakpoint_enabled(&self, session_id: Uuid, breakpoint_id: Uuid, enabled: bool) -> bool {
        if let Some(session) = self.sessions.get(&session_id) {
            session.set_breakpoint_enabled(breakpoint_id, enabled)
        } else {
            false
        }
    }

    /// Get all breakpoints for a session.
    pub fn get_breakpoints(&self, session_id: Uuid) -> Vec<Breakpoint> {
        self.sessions
            .get(&session_id)
            .map(|s| s.breakpoints.read().clone())
            .unwrap_or_default()
    }

    /// Send a debug command to a session.
    pub fn send_command(&self, session_id: Uuid, command: DebugCommand) -> bool {
        if let Some(sender) = self.command_senders.get(&session_id) {
            match sender.try_send(command) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(session_id = %session_id, error = %e, "Failed to send debug command");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get the pause notifier for a session (to send pause events).
    pub fn get_pause_notifier(&self, session_id: Uuid) -> Option<broadcast::Sender<PausedState>> {
        self.pause_notifiers.get(&session_id).map(|r| (*r).clone())
    }

    /// Get the command receiver for a session (for the debugger callback).
    pub fn get_command_receiver(&self, session_id: Uuid) -> Option<Arc<std::sync::Mutex<mpsc::Receiver<DebugCommand>>>> {
        self.command_receivers.get(&session_id).map(|r| Arc::clone(&*r))
    }

    /// Get shared breakpoints for a session (for the debugger callback).
    /// This returns the SAME Arc that the session uses, so changes are shared.
    pub fn get_shared_breakpoints(&self, session_id: Uuid) -> Option<SharedBreakpoints> {
        self.sessions.get(&session_id).map(|s| s.shared_breakpoints())
    }

    /// Get shared function breakpoints for a session.
    pub fn get_shared_function_breakpoints(&self, session_id: Uuid) -> Option<SharedFunctionBreakpoints> {
        self.sessions.get(&session_id).map(|s| s.shared_function_breakpoints())
    }

    /// Update paused state for a session.
    pub fn set_paused(&self, session_id: Uuid, state: Option<PausedState>) {
        if let Some(session) = self.sessions.get(&session_id) {
            *session.paused_at.write() = state;
        }
    }

    /// Check if a session is paused.
    pub fn is_paused(&self, session_id: Uuid) -> bool {
        self.sessions
            .get(&session_id)
            .map(|s| s.is_paused())
            .unwrap_or(false)
    }

    /// Get paused state for a session.
    pub fn get_paused_state(&self, session_id: Uuid) -> Option<PausedState> {
        self.sessions
            .get(&session_id)
            .and_then(|s| s.paused_at.read().clone())
    }

    /// List all active sessions.
    pub fn list_sessions(&self) -> Vec<(Uuid, DebugTarget, bool)> {
        self.sessions
            .iter()
            .map(|r| (r.id, r.target.clone(), r.is_paused()))
            .collect()
    }

    /// Get session count.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Find sessions debugging a specific script.
    pub fn sessions_for_script(&self, script_path: &str) -> Vec<Uuid> {
        self.sessions
            .iter()
            .filter(|r| {
                r.breakpoints
                    .read()
                    .iter()
                    .any(|bp| bp.source == script_path && bp.enabled)
            })
            .map(|r| r.id)
            .collect()
    }

    /// Find sessions debugging a specific script with a specific target.
    ///
    /// This is used by BehaviorManagers to find only debug sessions that apply
    /// to their context (Live server or a specific Playtest).
    pub fn sessions_for_script_with_target(&self, script_path: &str, target: &DebugTarget) -> Vec<Uuid> {
        self.sessions
            .iter()
            .filter(|r| {
                // Match target
                &r.target == target &&
                // Has enabled breakpoints for this script
                r.breakpoints
                    .read()
                    .iter()
                    .any(|bp| bp.source == script_path && bp.enabled)
            })
            .map(|r| r.id)
            .collect()
    }

    /// Create a debug-enabled Rhai engine for a specific session.
    ///
    /// The returned engine will:
    /// - Stop at breakpoints set for this session
    /// - Send pause notifications via the session's broadcast channel
    /// - Wait for commands from the session's command channel
    ///
    /// This takes an owned engine (created via `ScriptEngine::create_engine_with_bindings()`)
    /// and registers the debugger on it.
    ///
    /// Returns None if the session doesn't exist.
    pub fn create_debug_engine(
        &self,
        session_id: Uuid,
        mut base_engine: Engine,
        script_path: &str,
        entity_context: Option<EntityContext>,
    ) -> Option<Engine> {
        let session = self.sessions.get(&session_id)?;

        // Get the channels and shared state
        let pause_tx = self.get_pause_notifier(session_id)?;
        let command_rx = self.get_command_receiver(session_id)?;
        let breakpoints = session.shared_breakpoints();
        let function_breakpoints = session.shared_function_breakpoints();

        // Register the debugger on the provided engine
        register_debugger(
            &mut base_engine,
            session_id,
            breakpoints,
            function_breakpoints,
            pause_tx,
            command_rx,
            script_path.to_string(),
            entity_context,
        );

        Some(base_engine)
    }

    /// Check if a session has breakpoints for a specific script.
    pub fn has_breakpoints_for_script(&self, session_id: Uuid, script_path: &str) -> bool {
        self.sessions
            .get(&session_id)
            .map(|s| {
                s.breakpoints
                    .read()
                    .iter()
                    .any(|bp| bp.source == script_path && bp.enabled)
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let controller = DebugController::new();
        let owner_id = Uuid::new_v4();

        // Start session
        let (session_id, _rx) = controller.start_session(owner_id, DebugTarget::Live);
        assert_eq!(controller.session_count(), 1);

        // Get session
        let session = controller.get_session(session_id).unwrap();
        assert_eq!(session.owner_id, owner_id);

        // End session
        controller.end_session(session_id);
        assert_eq!(controller.session_count(), 0);
    }

    #[test]
    fn test_breakpoint_management() {
        let controller = DebugController::new();
        let owner_id = Uuid::new_v4();
        let (session_id, _rx) = controller.start_session(owner_id, DebugTarget::Live);

        // Set breakpoint
        let bp = Breakpoint::at_line("test.rhai", 10);
        let bp_id = bp.id;
        controller.set_breakpoint(session_id, bp);

        // Get breakpoints
        let bps = controller.get_breakpoints(session_id);
        assert_eq!(bps.len(), 1);
        assert_eq!(bps[0].line, 10);

        // Disable breakpoint
        controller.set_breakpoint_enabled(session_id, bp_id, false);
        let bps = controller.get_breakpoints(session_id);
        assert!(!bps[0].enabled);

        // Remove breakpoint
        assert!(controller.remove_breakpoint(session_id, bp_id));
        assert!(controller.get_breakpoints(session_id).is_empty());
    }
}
