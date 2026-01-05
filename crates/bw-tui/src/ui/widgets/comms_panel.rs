//! Communications/chat panel

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use bw_shared::messages::ChatChannel;

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.game.chat_messages.is_empty() {
        let empty = Paragraph::new("No messages")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
        return;
    }

    // Show most recent messages first
    let items: Vec<ListItem> = state
        .game
        .chat_messages
        .iter()
        .rev()
        .take(area.height as usize)
        .map(|msg| {
            let channel_style = match msg.channel {
                ChatChannel::Sector => Style::default().fg(Color::White),
                ChatChannel::Squadron => Style::default().fg(Color::Cyan),
                ChatChannel::Direct => Style::default().fg(Color::Magenta),
                ChatChannel::System => Style::default().fg(Color::Yellow),
            };

            let channel_prefix = match msg.channel {
                ChatChannel::Sector => "[S]",
                ChatChannel::Squadron => "[Q]",
                ChatChannel::Direct => "[D]",
                ChatChannel::System => "[!]",
            };

            let line = Line::from(vec![
                Span::styled(channel_prefix, channel_style),
                Span::raw(" "),
                Span::styled(
                    format!("{}:", msg.sender_name),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" "),
                Span::raw(&msg.message),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}
