//! Raw WebSocket message log

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use crate::state::{AppState, MessageDirection};

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.debug.message_log.is_empty() {
        let empty = Paragraph::new("No messages logged yet")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .debug
        .message_log
        .iter()
        .take(area.height as usize)
        .map(|msg| {
            let direction_style = match msg.direction {
                MessageDirection::Sent => Style::default().fg(Color::Green),
                MessageDirection::Received => Style::default().fg(Color::Cyan),
            };

            let direction_char = match msg.direction {
                MessageDirection::Sent => ">",
                MessageDirection::Received => "<",
            };

            let time = msg.timestamp.format("%H:%M:%S%.3f");

            let line = Line::from(vec![
                Span::styled(
                    direction_char,
                    direction_style.add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {} ", time),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(&msg.message_type, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(&msg.summary, Style::default().fg(Color::White)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}
