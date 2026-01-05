//! Debug panel widgets

mod message_log;
mod metrics_panel;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Tabs},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Debug Panel (F12 to hide) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Tab bar
    let tabs_titles = vec!["Metrics", "Messages", "State"];
    let tabs = Tabs::new(tabs_titles)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(if state.debug.show_metrics {
            0
        } else if state.debug.show_messages {
            1
        } else {
            2
        });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(inner);

    frame.render_widget(tabs, chunks[0]);

    // Content based on selected tab
    if state.debug.show_metrics {
        metrics_panel::draw(frame, chunks[1], state);
    } else if state.debug.show_messages {
        message_log::draw(frame, chunks[1], state);
    } else {
        draw_state_inspector(frame, chunks[1], state);
    }
}

fn draw_state_inspector(frame: &mut Frame, area: Rect, state: &AppState) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let lines = vec![
        Line::from(vec![
            Span::styled("Player: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "{} ({})",
                state.game.username,
                state
                    .game
                    .player_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".into())
            )),
        ]),
        Line::from(vec![
            Span::styled("Ship: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "{} ({})",
                state.game.ship_name,
                state
                    .game
                    .ship_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".into())
            )),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "({:.1}, {:.1})",
                state.game.position.0, state.game.position.1
            )),
        ]),
        Line::from(vec![
            Span::styled("Sector: ", Style::default().fg(Color::DarkGray)),
            Span::raw(
                state
                    .game
                    .sector
                    .as_ref()
                    .map(|s| format!("{} ({})", s.name, s.id))
                    .unwrap_or_else(|| "none".into()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Ships in sector: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", state.game.ships.len())),
        ]),
        Line::from(vec![
            Span::styled("Locations: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", state.game.locations.len())),
        ]),
        Line::from(vec![
            Span::styled("Missions: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "{} available, {} active",
                state.game.available_missions.len(),
                if state.game.active_mission.is_some() {
                    1
                } else {
                    0
                }
            )),
        ]),
        Line::from(vec![
            Span::styled("Combat: ", Style::default().fg(Color::DarkGray)),
            Span::raw(if state.game.combat.is_some() {
                "Active"
            } else {
                "None"
            }),
        ]),
        Line::from(vec![
            Span::styled("Admin: ", Style::default().fg(Color::DarkGray)),
            Span::raw(if state.game.is_admin { "Yes" } else { "No" }),
        ]),
    ];

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}
