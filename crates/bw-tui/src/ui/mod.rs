//! UI rendering

mod layout;
pub mod widgets;
pub mod debug;

use ratatui::Frame;

use crate::state::AppState;

/// Main draw function
pub fn draw(frame: &mut Frame, state: &AppState) {
    layout::draw(frame, state);
}
