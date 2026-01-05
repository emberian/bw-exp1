//! UI-specific state (focus, tabs, modals, input)
//!
//! Contains all UI state including focus, tabs, modals, and input buffers.
//! Some variants and fields are reserved for future modal types.

#![allow(dead_code)]

use std::collections::VecDeque;

/// Which panel has keyboard focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelFocus {
    #[default]
    SectorMap,
    LeftSidebar,
    RightSidebar,
    StatusBar,
    Modal,
}

/// Input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal navigation mode
    #[default]
    Normal,
    /// Text input mode (chat)
    Insert,
    /// Command mode (vim-style :)
    Command,
}

/// Active tab in left sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftTab {
    #[default]
    Missions,
    Squadron,
}

/// Active tab in right sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightTab {
    #[default]
    Ship,
    Comms,
    Combat,
}

/// Modal dialog type
#[derive(Debug, Clone)]
pub enum Modal {
    Help,
    Confirm {
        title: String,
        message: String,
        on_confirm: ConfirmAction,
    },
    MissionChoice,
    StationServices,
}

/// Action to perform on confirm
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    AbandonMission,
    Undock,
    Disengage,
}

/// Notification displayed to user
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub created_at: std::time::Instant,
}

/// UI state
#[derive(Debug)]
pub struct UiState {
    /// Currently focused panel
    pub focus: PanelFocus,
    /// Active tab in left sidebar
    pub left_tab: LeftTab,
    /// Active tab in right sidebar
    pub right_tab: RightTab,
    /// Whether a modal dialog is active
    pub modal: Option<Modal>,
    /// Input mode
    pub input_mode: InputMode,
    /// Text input buffer
    pub input_buffer: String,
    /// Command buffer (for : commands)
    pub command_buffer: String,
    /// Show help overlay
    pub show_help: bool,
    /// Selected index in mission list
    pub mission_selected: usize,
    /// Selected index in ship list (for targeting)
    pub ship_selected: usize,
    /// Map cursor position (when navigating map)
    pub map_cursor: Option<(f64, f64)>,
    /// Scroll offset for combat log
    pub combat_log_scroll: usize,
    /// Scroll offset for chat
    pub chat_scroll: usize,
    /// Notifications queue
    pub notifications: VecDeque<Notification>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: PanelFocus::SectorMap,
            left_tab: LeftTab::Missions,
            right_tab: RightTab::Ship,
            modal: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            command_buffer: String::new(),
            show_help: false,
            mission_selected: 0,
            ship_selected: 0,
            map_cursor: None,
            combat_log_scroll: 0,
            chat_scroll: 0,
            notifications: VecDeque::new(),
        }
    }
}

impl UiState {
    /// Check if we're in any text input mode
    pub fn is_input_mode(&self) -> bool {
        matches!(self.input_mode, InputMode::Insert | InputMode::Command)
    }

    /// Add a notification
    pub fn add_notification(&mut self, message: String) {
        self.notifications.push_back(Notification {
            message,
            created_at: std::time::Instant::now(),
        });
        // Keep only last 5 notifications
        while self.notifications.len() > 5 {
            self.notifications.pop_front();
        }
    }

    /// Remove expired notifications (older than 5 seconds)
    pub fn cleanup_notifications(&mut self) {
        let now = std::time::Instant::now();
        self.notifications
            .retain(|n| now.duration_since(n.created_at).as_secs() < 5);
    }

    /// Cycle focus to next panel
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            PanelFocus::SectorMap => PanelFocus::LeftSidebar,
            PanelFocus::LeftSidebar => PanelFocus::RightSidebar,
            PanelFocus::RightSidebar => PanelFocus::SectorMap,
            PanelFocus::StatusBar => PanelFocus::SectorMap,
            PanelFocus::Modal => PanelFocus::Modal, // Stay on modal
        };
    }

    /// Cycle focus backwards
    pub fn cycle_focus_back(&mut self) {
        self.focus = match self.focus {
            PanelFocus::SectorMap => PanelFocus::RightSidebar,
            PanelFocus::LeftSidebar => PanelFocus::SectorMap,
            PanelFocus::RightSidebar => PanelFocus::LeftSidebar,
            PanelFocus::StatusBar => PanelFocus::SectorMap,
            PanelFocus::Modal => PanelFocus::Modal,
        };
    }
}
