//! Top resource/status bar

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into three sections
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(inner);

    // Left: Title and player info
    let left_spans = vec![
        Span::styled(
            "BLACKWING",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Rep: {}", state.game.reputation),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" "),
        Span::styled(
            format!("Fame: {}", state.game.fame),
            Style::default().fg(Color::Yellow),
        ),
    ];
    let left = Paragraph::new(Line::from(left_spans));
    frame.render_widget(left, chunks[0]);

    // Center: Sector info
    let sector_name = state
        .game
        .sector
        .as_ref()
        .map(|s| s.name.as_str())
        .unwrap_or("Unknown");
    let danger = state
        .game
        .sector
        .as_ref()
        .map(|s| s.danger_level.as_str())
        .unwrap_or("?");

    let danger_color = match danger {
        "Low" => Color::Green,
        "Moderate" => Color::Yellow,
        "High" => Color::Red,
        "Extreme" => Color::Magenta,
        _ => Color::White,
    };

    let center_spans = vec![
        Span::raw("Sector: "),
        Span::styled(sector_name, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" | Danger: "),
        Span::styled(danger, Style::default().fg(danger_color)),
    ];
    let center = Paragraph::new(Line::from(center_spans)).centered();
    frame.render_widget(center, chunks[1]);

    // Right: Resources and connection
    let fuel_color = if state.game.fuel < 20.0 {
        Color::Red
    } else if state.game.fuel < 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let right_spans = vec![
        Span::styled(
            format!("Fuel: {:.0}%", state.game.fuel),
            Style::default().fg(fuel_color),
        ),
        Span::raw(" "),
        Span::styled(
            format!("Ammo: {:.0}%", state.game.ammunition),
            Style::default().fg(Color::White),
        ),
        Span::raw(" | "),
        if state.game.connected {
            Span::styled("Connected", Style::default().fg(Color::Green))
        } else {
            Span::styled("Disconnected", Style::default().fg(Color::Red))
        },
    ];
    let right = Paragraph::new(Line::from(right_spans)).right_aligned();
    frame.render_widget(right, chunks[2]);
}
