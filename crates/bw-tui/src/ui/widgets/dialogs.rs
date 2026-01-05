//! Dialog widgets (mission choice, confirm, etc.)

#![allow(dead_code)]

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::state::AppState;

pub fn draw_mission_choice(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" Mission Choice ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(ref choice) = state.game.mission_choice {
        // Description
        let desc = Paragraph::new(choice.description.clone())
            .style(Style::default().fg(Color::White));

        let desc_height = 3.min(inner.height / 2);
        let desc_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: desc_height,
        };
        frame.render_widget(desc, desc_area);

        // Choices
        let choices_area = Rect {
            x: inner.x,
            y: inner.y + desc_height + 1,
            width: inner.width,
            height: inner.height.saturating_sub(desc_height + 1),
        };

        let items: Vec<ListItem> = choice
            .choices
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let key = format!("[{}] ", idx + 1);
                let style = if c.is_available {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let mut spans = vec![
                    Span::styled(key, Style::default().fg(Color::Cyan)),
                    Span::styled(&c.text, style),
                ];

                if !c.is_available {
                    if let Some(ref req) = c.requirement_text {
                        spans.push(Span::styled(
                            format!(" ({})", req),
                            Style::default().fg(Color::Red),
                        ));
                    }
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, choices_area);
    } else {
        let empty = Paragraph::new("No active choice")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, inner);
    }
}

pub fn draw_confirm(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
) {
    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(message),
        Line::default(),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" Yes  "),
            Span::styled("[n]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" No  "),
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Cancel"),
        ]),
    ];

    let para = Paragraph::new(content);
    frame.render_widget(para, inner);
}
