//! Keybinding definitions
//!
//! This module defines all keybindings used in the TUI.
//! Some keybindings may not be directly referenced but are kept for documentation
//! and future use.

#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyModifiers};

/// A keybinding definition
#[derive(Debug, Clone)]
pub struct Keybinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub description: &'static str,
}

impl Keybinding {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers, description: &'static str) -> Self {
        Self {
            code,
            modifiers,
            description,
        }
    }

    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && self.modifiers == modifiers
    }
}

// Global keybindings
pub const QUIT: Keybinding = Keybinding::new(KeyCode::Char('q'), KeyModifiers::NONE, "Quit");
pub const HELP: Keybinding = Keybinding::new(KeyCode::Char('?'), KeyModifiers::NONE, "Show help");
pub const TAB_NEXT: Keybinding =
    Keybinding::new(KeyCode::Tab, KeyModifiers::NONE, "Next panel");
pub const TAB_PREV: Keybinding =
    Keybinding::new(KeyCode::BackTab, KeyModifiers::SHIFT, "Previous panel");
pub const DEBUG_TOGGLE: Keybinding =
    Keybinding::new(KeyCode::F(12), KeyModifiers::NONE, "Toggle debug");
pub const ESCAPE: Keybinding =
    Keybinding::new(KeyCode::Esc, KeyModifiers::NONE, "Cancel/Close");
pub const COMMAND_MODE: Keybinding =
    Keybinding::new(KeyCode::Char(':'), KeyModifiers::NONE, "Command mode");

// Map navigation
pub const MOVE_UP: Keybinding = Keybinding::new(KeyCode::Char('k'), KeyModifiers::NONE, "Move up");
pub const MOVE_DOWN: Keybinding =
    Keybinding::new(KeyCode::Char('j'), KeyModifiers::NONE, "Move down");
pub const MOVE_LEFT: Keybinding =
    Keybinding::new(KeyCode::Char('h'), KeyModifiers::NONE, "Move left");
pub const MOVE_RIGHT: Keybinding =
    Keybinding::new(KeyCode::Char('l'), KeyModifiers::NONE, "Move right");
pub const SELECT: Keybinding =
    Keybinding::new(KeyCode::Enter, KeyModifiers::NONE, "Select/Confirm");

// Ship actions
pub const DOCK: Keybinding = Keybinding::new(KeyCode::Char('d'), KeyModifiers::NONE, "Dock");
pub const UNDOCK: Keybinding = Keybinding::new(KeyCode::Char('u'), KeyModifiers::NONE, "Undock");
pub const STOP: Keybinding = Keybinding::new(KeyCode::Char('s'), KeyModifiers::NONE, "Stop");
pub const ENGAGE: Keybinding = Keybinding::new(KeyCode::Char('e'), KeyModifiers::NONE, "Engage");
pub const DISENGAGE: Keybinding =
    Keybinding::new(KeyCode::Char('r'), KeyModifiers::NONE, "Retreat/Disengage");
pub const JUMP: Keybinding =
    Keybinding::new(KeyCode::Char('g'), KeyModifiers::NONE, "Go to/Jump");
pub const HAIL: Keybinding = Keybinding::new(KeyCode::Char('y'), KeyModifiers::NONE, "Hail");

// Mission actions
pub const ACCEPT: Keybinding =
    Keybinding::new(KeyCode::Char('a'), KeyModifiers::NONE, "Accept mission");
pub const ABANDON: Keybinding =
    Keybinding::new(KeyCode::Char('x'), KeyModifiers::NONE, "Abandon mission");

// Chat
pub const CHAT: Keybinding =
    Keybinding::new(KeyCode::Char('c'), KeyModifiers::NONE, "Chat input");
pub const INSERT: Keybinding =
    Keybinding::new(KeyCode::Char('i'), KeyModifiers::NONE, "Insert mode");

/// Get all keybindings for help display
pub fn all_keybindings() -> Vec<(&'static str, Vec<Keybinding>)> {
    vec![
        (
            "Global",
            vec![
                QUIT,
                HELP,
                TAB_NEXT,
                TAB_PREV,
                DEBUG_TOGGLE,
                ESCAPE,
                COMMAND_MODE,
            ],
        ),
        (
            "Navigation",
            vec![MOVE_UP, MOVE_DOWN, MOVE_LEFT, MOVE_RIGHT, SELECT],
        ),
        (
            "Ship Actions",
            vec![DOCK, UNDOCK, STOP, ENGAGE, DISENGAGE, JUMP, HAIL],
        ),
        ("Missions", vec![ACCEPT, ABANDON]),
        ("Chat", vec![CHAT, INSERT]),
    ]
}
