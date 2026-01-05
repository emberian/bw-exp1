//! Ship status panel

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Gauge, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Ship name/class
            Constraint::Length(2), // Hull bar
            Constraint::Length(2), // Shield bar
            Constraint::Length(2), // Status
            Constraint::Length(2), // Position
            Constraint::Min(1),    // Remaining space
        ])
        .split(area);

    // Ship name and class
    let ship_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                &state.game.ship_name,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(&state.game.ship_class, Style::default().fg(Color::DarkGray)),
        ]),
    ]);
    frame.render_widget(ship_info, chunks[0]);

    // Hull bar
    let hull_pct = state.game.ship_hull / 100.0;
    let hull_color = if state.game.ship_hull < 25.0 {
        Color::Red
    } else if state.game.ship_hull < 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let hull_gauge = Gauge::default()
        .label(format!("Hull: {:.0}%", state.game.ship_hull))
        .ratio(hull_pct as f64)
        .gauge_style(Style::default().fg(hull_color));
    frame.render_widget(hull_gauge, chunks[1]);

    // Shield bar
    let shield_pct = state.game.ship_shields / 100.0;
    let shield_gauge = Gauge::default()
        .label(format!("Shield: {:.0}%", state.game.ship_shields))
        .ratio(shield_pct as f64)
        .gauge_style(Style::default().fg(Color::Cyan));
    frame.render_widget(shield_gauge, chunks[2]);

    // Status
    let status_color = match state.game.ship_status.as_str() {
        "Idle" => Color::White,
        "InTransit" | "Moving" => Color::Yellow,
        "Docked" => Color::Blue,
        "InCombat" => Color::Red,
        _ => Color::Gray,
    };

    let status = Paragraph::new(Line::from(vec![
        Span::raw("Status: "),
        Span::styled(&state.game.ship_status, Style::default().fg(status_color)),
    ]));
    frame.render_widget(status, chunks[3]);

    // Position
    let position = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("Pos: ({:.0}, {:.0})", state.game.position.0, state.game.position.1),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(position, chunks[4]);

    // Active combat info if in combat
    if let Some(ref combat) = state.game.combat {
        let combat_info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "IN COMBAT",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
                ),
            ]),
            Line::from(vec![
                Span::raw("Round: "),
                Span::styled(
                    format!("{}", combat.round),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        ]);
        frame.render_widget(combat_info, chunks[5]);
    }
}
