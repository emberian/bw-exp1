//! Squadron panel widget

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    // Split into squadron info and invites/proposals
    let has_invites = !state.game.squadron_invites.is_empty();
    let has_proposals = !state.game.alliance_proposals.is_empty();
    let needs_split = has_invites || has_proposals;

    let (info_area, notif_area) = if needs_split {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(4)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Squadron info
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
                Line::from(vec![Span::styled(
                    format!("\"{}\"", motto),
                    Style::default().fg(Color::DarkGray),
                )])
            } else {
                Line::default()
            },
            Line::default(),
            Line::from(vec![
                Span::styled(":sq leave", Style::default().fg(Color::DarkGray)),
                Span::styled(" to leave", Style::default().fg(Color::Rgb(80, 80, 80))),
            ]),
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
            Line::default(),
            Line::from(vec![
                Span::styled(":sq create ", Style::default().fg(Color::DarkGray)),
                Span::styled("<name> <tag>", Style::default().fg(Color::Rgb(80, 80, 80))),
            ]),
        ]
    };

    let para = Paragraph::new(content);
    frame.render_widget(para, info_area);

    // Draw invites/proposals if any
    if let Some(notif_area) = notif_area {
        let mut lines = Vec::new();

        // Squadron invites
        if let Some(invite) = state.game.squadron_invites.first() {
            lines.push(Line::from(vec![
                Span::styled("Invite: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("[{}] {}", invite.squadron_tag, invite.squadron_name),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  from ", Style::default().fg(Color::DarkGray)),
                Span::styled(&invite.inviter_name, Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("[y]", Style::default().fg(Color::Green)),
                Span::styled("Accept ", Style::default().fg(Color::DarkGray)),
                Span::styled("[n]", Style::default().fg(Color::Red)),
                Span::styled("Decline", Style::default().fg(Color::DarkGray)),
            ]));
        }
        // Alliance proposals (for squadron leaders)
        else if let Some(proposal) = state.game.alliance_proposals.first() {
            lines.push(Line::from(vec![
                Span::styled("Alliance: ", Style::default().fg(Color::Magenta)),
                Span::styled(
                    format!("[{}] {}", proposal.from_squadron_tag, proposal.from_squadron_name),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("[y]", Style::default().fg(Color::Green)),
                Span::styled("Accept ", Style::default().fg(Color::DarkGray)),
                Span::styled("[n]", Style::default().fg(Color::Red)),
                Span::styled("Decline", Style::default().fg(Color::DarkGray)),
            ]));
        }

        let notif_para = Paragraph::new(lines);
        frame.render_widget(notif_para, notif_area);
    }
}
