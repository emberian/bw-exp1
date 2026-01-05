//! Input event processing

use bw_shared::messages::ClientMessage;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::state::{AppState, InputMode, Modal, PanelFocus};

use super::keybindings::*;

/// Input handler for processing keyboard events
pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle a key press, returning an optional ClientMessage to send
    pub fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        // Handle input modes first
        match state.ui.input_mode {
            InputMode::Insert => return self.handle_insert_mode(code, modifiers, state),
            InputMode::Command => return self.handle_command_mode(code, modifiers, state),
            InputMode::Normal => {}
        }

        // Handle mission choice dialog (takes priority)
        if state.game.mission_choice.is_some() {
            return self.handle_mission_choice(code, state);
        }

        // Handle modal if present
        if state.ui.modal.is_some() {
            return self.handle_modal(code, modifiers, state);
        }

        // Global keybindings
        if HELP.matches(code, modifiers) {
            state.ui.show_help = !state.ui.show_help;
            return None;
        }

        if TAB_NEXT.matches(code, modifiers) {
            state.ui.cycle_focus();
            return None;
        }

        if TAB_PREV.matches(code, modifiers) {
            state.ui.cycle_focus_back();
            return None;
        }

        if DEBUG_TOGGLE.matches(code, modifiers) {
            state.debug.toggle();
            return None;
        }

        if COMMAND_MODE.matches(code, modifiers) {
            state.ui.input_mode = InputMode::Command;
            state.ui.command_buffer.clear();
            return None;
        }

        // Panel-specific handling
        match state.ui.focus {
            PanelFocus::SectorMap => self.handle_map_input(code, modifiers, state),
            PanelFocus::LeftSidebar => self.handle_left_sidebar(code, modifiers, state),
            PanelFocus::RightSidebar => self.handle_right_sidebar(code, modifiers, state),
            _ => None,
        }
    }

    fn handle_insert_mode(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        match code {
            KeyCode::Esc => {
                state.ui.input_mode = InputMode::Normal;
                state.ui.input_buffer.clear();
            }
            KeyCode::Enter => {
                if !state.ui.input_buffer.is_empty() {
                    let msg = state.ui.input_buffer.clone();
                    state.ui.input_buffer.clear();
                    state.ui.input_mode = InputMode::Normal;
                    return Some(ClientMessage::SendChat {
                        message: msg,
                        channel: bw_shared::messages::ChatChannel::Sector,
                    });
                }
            }
            KeyCode::Backspace => {
                state.ui.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                state.ui.input_buffer.push(c);
            }
            _ => {}
        }
        None
    }

    fn handle_command_mode(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        match code {
            KeyCode::Esc => {
                state.ui.input_mode = InputMode::Normal;
                state.ui.command_buffer.clear();
            }
            KeyCode::Enter => {
                let cmd = state.ui.command_buffer.clone();
                state.ui.command_buffer.clear();
                state.ui.input_mode = InputMode::Normal;
                return self.execute_command(&cmd, state);
            }
            KeyCode::Backspace => {
                state.ui.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                state.ui.command_buffer.push(c);
            }
            _ => {}
        }
        None
    }

    fn execute_command(&self, cmd: &str, state: &mut AppState) -> Option<ClientMessage> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "q" | "quit" => {
                // This should trigger quit, but we can't from here
                // The app.rs will handle 'q' key separately
                state.ui.add_notification("Use 'q' to quit".into());
            }
            "help" => {
                state.ui.show_help = true;
            }
            "debug" => {
                state.debug.toggle();
            }
            "dock" => {
                // Find nearest station and dock
                if let Some(station) = state
                    .game
                    .locations
                    .iter()
                    .find(|l| l.location_type == "Station")
                {
                    return Some(ClientMessage::dock(station.id));
                }
                state.ui.add_notification("No station nearby".into());
            }
            "undock" => {
                return Some(ClientMessage::undock());
            }
            "stop" => {
                return Some(ClientMessage::stop_movement());
            }
            "goto" => {
                if parts.len() >= 3 {
                    if let (Ok(x), Ok(y)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
                        return Some(ClientMessage::move_to_position(x, y, 0.0));
                    }
                }
                state.ui.add_notification("Usage: goto <x> <y>".into());
            }
            "chat" => {
                if parts.len() >= 2 {
                    let msg = parts[1..].join(" ");
                    return Some(ClientMessage::SendChat {
                        message: msg,
                        channel: bw_shared::messages::ChatChannel::Sector,
                    });
                }
            }
            "metrics" => {
                return Some(ClientMessage::SubscribeMetrics);
            }
            _ => {
                state.ui.add_notification(format!("Unknown command: {}", parts[0]));
            }
        }
        None
    }

    fn handle_modal(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        match code {
            KeyCode::Esc => {
                state.ui.modal = None;
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                if let Some(Modal::Confirm { on_confirm, .. }) = state.ui.modal.take() {
                    return match on_confirm {
                        crate::state::ConfirmAction::AbandonMission => {
                            if let Some(mission) = &state.game.active_mission {
                                Some(ClientMessage::abandon_mission(mission.id))
                            } else {
                                None
                            }
                        }
                        crate::state::ConfirmAction::Undock => Some(ClientMessage::undock()),
                        crate::state::ConfirmAction::Disengage => {
                            Some(ClientMessage::disengage_combat())
                        }
                    };
                }
            }
            KeyCode::Char('n') => {
                state.ui.modal = None;
            }
            _ => {}
        }
        None
    }

    fn handle_mission_choice(
        &mut self,
        code: KeyCode,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        // Esc to dismiss (if possible)
        if code == KeyCode::Esc {
            // Can't dismiss mission choices - they require a response
            state.ui.add_notification("Must select a choice".into());
            return None;
        }

        // Number keys 1-9 to select choice
        if let KeyCode::Char(c) = code {
            if let Some(digit) = c.to_digit(10) {
                if digit >= 1 {
                    let idx = (digit - 1) as usize;
                    if let Some(ref choice) = state.game.mission_choice {
                        if let Some(selected) = choice.choices.get(idx) {
                            if selected.is_available {
                                let mission_id = choice.mission_id;
                                let choice_id = selected.id.clone();
                                // Clear the choice dialog
                                state.game.mission_choice = None;
                                return Some(ClientMessage::mission_choice(mission_id, choice_id));
                            } else {
                                state.ui.add_notification("That choice is not available".into());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn handle_map_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        // Navigation with hjkl or arrows
        let step = 50.0;
        match code {
            KeyCode::Up | KeyCode::Char('k') if modifiers.is_empty() => {
                let (x, y) = state.ui.map_cursor.unwrap_or(state.game.position);
                state.ui.map_cursor = Some((x, y - step));
            }
            KeyCode::Down | KeyCode::Char('j') if modifiers.is_empty() => {
                let (x, y) = state.ui.map_cursor.unwrap_or(state.game.position);
                state.ui.map_cursor = Some((x, y + step));
            }
            KeyCode::Left | KeyCode::Char('h') if modifiers.is_empty() => {
                let (x, y) = state.ui.map_cursor.unwrap_or(state.game.position);
                state.ui.map_cursor = Some((x - step, y));
            }
            KeyCode::Right | KeyCode::Char('l') if modifiers.is_empty() => {
                let (x, y) = state.ui.map_cursor.unwrap_or(state.game.position);
                state.ui.map_cursor = Some((x + step, y));
            }
            _ => {}
        }

        // Actions
        if SELECT.matches(code, modifiers) {
            // Move to cursor position
            if let Some((x, y)) = state.ui.map_cursor {
                return Some(ClientMessage::move_to_position(x, y, 0.0));
            }
        }

        if DOCK.matches(code, modifiers) {
            // Find nearest station
            if let Some(station) = state
                .game
                .locations
                .iter()
                .find(|l| l.location_type == "Station")
            {
                return Some(ClientMessage::dock(station.id));
            }
            state.ui.add_notification("No station nearby".into());
        }

        if UNDOCK.matches(code, modifiers) {
            return Some(ClientMessage::undock());
        }

        if STOP.matches(code, modifiers) {
            return Some(ClientMessage::stop_movement());
        }

        if ENGAGE.matches(code, modifiers) {
            // Engage selected ship
            if let Some(ship) = state.game.ships.get(state.ui.ship_selected) {
                return Some(ClientMessage::engage_target(ship.id));
            }
        }

        if DISENGAGE.matches(code, modifiers) {
            return Some(ClientMessage::disengage_combat());
        }

        if CHAT.matches(code, modifiers) || INSERT.matches(code, modifiers) {
            state.ui.input_mode = InputMode::Insert;
            state.ui.input_buffer.clear();
        }

        // Jump to adjacent sector (g key)
        if JUMP.matches(code, modifiers) {
            // Check if near a jumpgate
            let near_jumpgate = state.game.locations.iter().find(|l| {
                l.location_type == "Jumpgate"
                    && (state.game.position.0 - l.position.0).abs() < 100.0
                    && (state.game.position.1 - l.position.1).abs() < 100.0
            });

            if near_jumpgate.is_some() {
                // Jump to first adjacent sector
                if let Some(sector) = state.game.adjacent_sectors.first() {
                    state.ui.add_notification(format!("Jumping to {}...", sector.name));
                    return Some(ClientMessage::move_to_sector(sector.id));
                } else {
                    state.ui.add_notification("No adjacent sectors".into());
                }
            } else {
                // Show adjacent sectors info
                if state.game.adjacent_sectors.is_empty() {
                    state.ui.add_notification("No adjacent sectors".into());
                } else {
                    let sectors: Vec<_> = state
                        .game
                        .adjacent_sectors
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect();
                    state.ui.add_notification(format!("Adjacent: {}", sectors.join(", ")));
                }
            }
        }

        // Hail selected ship (y key)
        if HAIL.matches(code, modifiers) {
            if let Some(ship) = state.game.ships.get(state.ui.ship_selected) {
                state.ui.add_notification(format!("Hailing {}...", ship.name));
                return Some(ClientMessage::hail(ship.id));
            } else {
                state.ui.add_notification("No target selected".into());
            }
        }

        // Number keys for quick ship selection
        if let KeyCode::Char(c) = code {
            if let Some(digit) = c.to_digit(10) {
                if digit > 0 {
                    let idx = (digit - 1) as usize;
                    if idx < state.game.ships.len() {
                        state.ui.ship_selected = idx;
                    }
                }
            }
        }

        None
    }

    fn handle_left_sidebar(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        // Tab switching
        if code == KeyCode::Char('1') {
            state.ui.left_tab = crate::state::LeftTab::Missions;
            return None;
        }
        if code == KeyCode::Char('2') {
            state.ui.left_tab = crate::state::LeftTab::Squadron;
            return None;
        }

        // Navigation
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if state.ui.mission_selected > 0 {
                    state.ui.mission_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.ui.mission_selected < state.game.available_missions.len().saturating_sub(1)
                {
                    state.ui.mission_selected += 1;
                }
            }
            _ => {}
        }

        // Mission actions
        if ACCEPT.matches(code, modifiers) {
            if let Some(mission) = state.game.available_missions.get(state.ui.mission_selected) {
                if mission.can_accept {
                    return Some(ClientMessage::accept_mission(mission.id));
                }
            }
        }

        if ABANDON.matches(code, modifiers) {
            if state.game.active_mission.is_some() {
                state.ui.modal = Some(Modal::Confirm {
                    title: "Abandon Mission".into(),
                    message: "Are you sure you want to abandon the current mission?".into(),
                    on_confirm: crate::state::ConfirmAction::AbandonMission,
                });
            }
        }

        None
    }

    fn handle_right_sidebar(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        state: &mut AppState,
    ) -> Option<ClientMessage> {
        // Tab switching
        if code == KeyCode::Char('1') {
            state.ui.right_tab = crate::state::RightTab::Ship;
            return None;
        }
        if code == KeyCode::Char('2') {
            state.ui.right_tab = crate::state::RightTab::Comms;
            return None;
        }
        if code == KeyCode::Char('3') {
            state.ui.right_tab = crate::state::RightTab::Combat;
            return None;
        }

        // Chat input shortcut
        if code == KeyCode::Char('c') || code == KeyCode::Char('i') {
            state.ui.input_mode = InputMode::Insert;
            state.ui.input_buffer.clear();
        }

        None
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
