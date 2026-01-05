//! Mission panel widget

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Active mission
            Constraint::Min(5),    // Available missions
        ])
        .split(area);

    // Active mission section
    draw_active_mission(frame, chunks[0], state);

    // Available missions
    draw_available_missions(frame, chunks[1], state);
}

fn draw_active_mission(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = if let Some(ref mission) = state.game.active_mission {
        vec![
            Line::from(vec![
                Span::styled("Active: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &mission.title,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Progress: "),
                Span::styled(
                    format!("{:.0}%", mission.progress * 100.0),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No active mission",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_available_missions(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.game.available_missions.is_empty() {
        let empty = Paragraph::new("No missions available")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .game
        .available_missions
        .iter()
        .enumerate()
        .map(|(idx, mission)| {
            let is_selected = idx == state.ui.mission_selected;

            // Mission type icon
            let type_icon = match mission.mission_type.as_str() {
                "Combat" => "!",
                "Delivery" => ">",
                "Escort" => "#",
                "Patrol" => "~",
                "Investigation" => "?",
                _ => "*",
            };

            // Priority color
            let priority_style = match mission.priority.as_str() {
                "Critical" => Style::default().fg(Color::Red),
                "High" => Style::default().fg(Color::Yellow),
                "Normal" => Style::default().fg(Color::White),
                "Low" => Style::default().fg(Color::DarkGray),
                _ => Style::default(),
            };

            let mut spans = vec![
                Span::styled(
                    format!("[{}] ", type_icon),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(&mission.title, priority_style),
            ];

            // High profile badge
            if mission.is_high_profile {
                spans.push(Span::styled(" HP", Style::default().fg(Color::Magenta)));
            }

            // Expiry warning
            if let Some(expires) = mission.expires_in_seconds
                && expires < 300
            {
                spans.push(Span::styled(
                    format!(" ({}s)", expires),
                    Style::default().fg(Color::Red),
                ));
            }

            let style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}
