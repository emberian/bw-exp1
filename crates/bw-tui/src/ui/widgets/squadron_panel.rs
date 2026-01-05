//! Squadron panel widget

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = if let Some(ref squadron) = state.game.squadron {
        vec![
            Line::from(vec![
                Span::styled(
                    format!("[{}] {}", squadron.tag, squadron.name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Leader: "),
                Span::styled(&squadron.leader_name, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("Members: "),
                Span::styled(
                    format!("{}", squadron.member_count),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::default(),
            if let Some(ref motto) = squadron.motto {
                Line::from(vec![
                    Span::styled(
                        format!("\"{}\"", motto),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            } else {
                Line::default()
            },
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Not in a squadron",
                Style::default().fg(Color::DarkGray),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Create or join a squadron",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "to coordinate with allies",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}
