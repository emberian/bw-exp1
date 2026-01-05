//! Combat log widget

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.game.combat_log.is_empty() {
        let empty = Paragraph::new("No combat events")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
        return;
    }

    // Show most recent events first
    let items: Vec<ListItem> = state
        .game
        .combat_log
        .iter()
        .rev()
        .take(area.height as usize)
        .map(|event| {
            let event_style = match event.event_type.as_str() {
                "Hit" | "CriticalHit" => Style::default().fg(Color::Red),
                "Miss" | "Dodge" => Style::default().fg(Color::Yellow),
                "ShieldHit" => Style::default().fg(Color::Cyan),
                "Destroyed" => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                "Retreat" => Style::default().fg(Color::Blue),
                _ => Style::default().fg(Color::White),
            };

            let damage_str = event
                .damage
                .map(|d| format!(" ({:.0} dmg)", d))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(&event.attacker_name, Style::default().fg(Color::Green)),
                Span::raw(" -> "),
                Span::styled(&event.target_name, Style::default().fg(Color::Red)),
                Span::raw(": "),
                Span::styled(&event.event_type, event_style),
                Span::styled(damage_str, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}
