//! Application state management

mod debug_state;
mod game_state;
mod ui_state;

pub use debug_state::{DebugState, MessageDirection};
pub use game_state::GameState;
pub use ui_state::{ConfirmAction, InputMode, LeftTab, Modal, PanelFocus, RightTab, UiState};

/// Combined application state
pub struct AppState {
    /// Game state from server
    pub game: GameState,
    /// UI-specific state (focus, tabs, modals)
    pub ui: UiState,
    /// Debug/GM state
    pub debug: DebugState,
}
