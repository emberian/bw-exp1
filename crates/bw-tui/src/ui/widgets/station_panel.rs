//! Station services panel (shown when docked)

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Station Services ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Find current station from docked status
    let station = state.game.locations.iter().find(|l| {
        l.location_type == "Station"
            && (state.game.position.0 - l.position.0).abs() < 50.0
            && (state.game.position.1 - l.position.1).abs() < 50.0
    });

    if let Some(station) = station {
        // Split into header and services list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(inner);

        // Header with station name
        let header_lines = vec![
            Line::from(vec![Span::styled(
                &station.name,
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::default(),
            Line::from(Span::styled(
                "Available Services:",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let header = Paragraph::new(header_lines);
        frame.render_widget(header, chunks[0]);

        // Services list
        if station.services.is_empty() {
            let empty = Paragraph::new("  No services available")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = station
                .services
                .iter()
                .enumerate()
                .map(|(idx, service)| {
                    let icon = match service.as_str() {
                        "Repair" => "[R]",
                        "Refuel" => "[F]",
                        "Rearm" => "[A]",
                        "Trade" => "[T]",
                        "Mission" => "[M]",
                        "Hangar" => "[H]",
                        _ => "[*]",
                    };

                    let key_hint = format!("{}", idx + 1);

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {} ", key_hint),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(icon, Style::default().fg(Color::Cyan)),
                        Span::raw(" "),
                        Span::styled(service, Style::default().fg(Color::White)),
                    ]))
                })
                .collect();

            let list = List::new(items).highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );
            frame.render_widget(list, chunks[1]);
        }
    } else {
        // Not at a station
        let lines = vec![
            Line::from(Span::styled(
                "Not docked at a station",
                Style::default().fg(Color::DarkGray),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Press 'd' near a station to dock",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let para = Paragraph::new(lines);
        frame.render_widget(para, inner);
    }
}
