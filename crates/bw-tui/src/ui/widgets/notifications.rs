//! Notification toast display

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.ui.notifications.is_empty() {
        return;
    }

    // Calculate notification area (top-right corner)
    let notif_width = 40.min(area.width.saturating_sub(2));
    let notif_height = state.ui.notifications.len().min(5) as u16 + 2;

    let notif_area = Rect {
        x: area.x + area.width.saturating_sub(notif_width + 1),
        y: area.y + 4, // Below header
        width: notif_width,
        height: notif_height,
    };

    // Clear the area
    frame.render_widget(Clear, notif_area);

    // Build notification lines
    let lines: Vec<Line> = state
        .ui
        .notifications
        .iter()
        .map(|n| {
            let age = n.created_at.elapsed().as_secs();
            let style = if age < 2 {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(&n.message, style))
        })
        .collect();

    let notifications = Paragraph::new(lines).block(
        Block::default()
            .title(" Notifications ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(notifications, notif_area);
}
