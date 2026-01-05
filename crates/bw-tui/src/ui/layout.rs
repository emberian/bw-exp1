//! Main layout composition

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::{AppState, InputMode, Modal, PanelFocus};

use super::{debug, widgets};

/// Draw the main UI layout
pub fn draw(frame: &mut Frame, state: &AppState) {
    let size = frame.area();

    // Main vertical layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header/resource bar
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(size);

    // Draw header
    widgets::resource_bar::draw(frame, chunks[0], state);

    // Main content area
    let main_area = if state.debug.show_debug {
        // Split for debug panel
        let debug_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);

        // Draw debug panel
        debug::draw(frame, debug_chunks[1], state);

        debug_chunks[0]
    } else {
        chunks[1]
    };

    // Three-column layout
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Left sidebar
            Constraint::Percentage(50), // Center (map)
            Constraint::Percentage(25), // Right sidebar
        ])
        .split(main_area);

    // Draw panels
    draw_left_sidebar(frame, columns[0], state);
    widgets::sector_map::draw(frame, columns[1], state);
    draw_right_sidebar(frame, columns[2], state);

    // Draw status bar
    draw_status_bar(frame, chunks[2], state);

    // Draw overlays (help, modals)
    if state.ui.show_help {
        draw_help_overlay(frame, size);
    }

    if let Some(ref modal) = state.ui.modal {
        draw_modal(frame, size, modal, state);
    }

    // Draw mission choice dialog if active (takes priority)
    if state.game.mission_choice.is_some() {
        let choice_area = centered_rect(60, 50, size);
        frame.render_widget(Clear, choice_area);
        widgets::dialogs::draw_mission_choice(frame, choice_area, state);
    }

    // Draw input line if in input mode
    if state.ui.input_mode != InputMode::Normal {
        draw_input_line(frame, size, state);
    }

    // Draw notifications
    widgets::notifications::draw(frame, size, state);
}

fn draw_left_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.ui.focus == PanelFocus::LeftSidebar;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(format!(
            " {} ",
            match state.ui.left_tab {
                crate::state::LeftTab::Missions => "[1]Missions [2]Squad",
                crate::state::LeftTab::Squadron => "[1]Missions [2]Squad",
            }
        ))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match state.ui.left_tab {
        crate::state::LeftTab::Missions => {
            widgets::mission_panel::draw(frame, inner, state);
        }
        crate::state::LeftTab::Squadron => {
            widgets::squadron_panel::draw(frame, inner, state);
        }
    }
}

fn draw_right_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.ui.focus == PanelFocus::RightSidebar;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(format!(
            " {} ",
            match state.ui.right_tab {
                crate::state::RightTab::Ship => "[1]Ship [2]Comms [3]Combat",
                crate::state::RightTab::Comms => "[1]Ship [2]Comms [3]Combat",
                crate::state::RightTab::Combat => "[1]Ship [2]Comms [3]Combat",
            }
        ))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match state.ui.right_tab {
        crate::state::RightTab::Ship => {
            widgets::ship_status::draw(frame, inner, state);
        }
        crate::state::RightTab::Comms => {
            widgets::comms_panel::draw(frame, inner, state);
        }
        crate::state::RightTab::Combat => {
            widgets::combat_log::draw(frame, inner, state);
        }
    }
}

fn draw_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build context-sensitive key hints
    let key_hints = build_key_hints(state);

    // Split into hints and status
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(25)])
        .split(inner);

    // Key hints (left side)
    let hints = Paragraph::new(Line::from(key_hints));
    frame.render_widget(hints, chunks[0]);

    // Connection status (right side)
    let status_spans = vec![
        if state.game.connected {
            Span::styled("Connected", Style::default().fg(Color::Green))
        } else {
            Span::styled("Disconnected", Style::default().fg(Color::Red))
        },
        Span::styled(
            format!(" Tick:{}", state.game.server_tick),
            Style::default().fg(Color::DarkGray),
        ),
    ];
    let status = Paragraph::new(Line::from(status_spans)).right_aligned();
    frame.render_widget(status, chunks[1]);
}

/// Build context-sensitive key hints based on current state
fn build_key_hints(state: &AppState) -> Vec<Span<'static>> {
    let mut hints = Vec::new();
    let key_style = Style::default().fg(Color::Cyan);
    let sep_style = Style::default().fg(Color::DarkGray);

    // Always show these
    hints.push(Span::styled("?", key_style));
    hints.push(Span::styled("Help ", sep_style));

    // Context-sensitive hints based on focus and state
    match state.ui.focus {
        PanelFocus::SectorMap => {
            hints.push(Span::styled("hjkl", key_style));
            hints.push(Span::styled("Move ", sep_style));
            hints.push(Span::styled("Enter", key_style));
            hints.push(Span::styled("Go ", sep_style));

            // Ship actions
            if state.game.ship_status == "Docked" {
                hints.push(Span::styled("u", key_style));
                hints.push(Span::styled("Undock ", sep_style));
            } else {
                hints.push(Span::styled("d", key_style));
                hints.push(Span::styled("Dock ", sep_style));
                hints.push(Span::styled("s", key_style));
                hints.push(Span::styled("Stop ", sep_style));
            }

            // Combat
            if state.game.combat.is_some() {
                hints.push(Span::styled("r", key_style));
                hints.push(Span::styled("Retreat ", sep_style));
            } else if !state.game.ships.is_empty() {
                hints.push(Span::styled("e", key_style));
                hints.push(Span::styled("Engage ", sep_style));
                hints.push(Span::styled("1-9", key_style));
                hints.push(Span::styled("Select ", sep_style));
            }

            // Jump
            hints.push(Span::styled("g", key_style));
            hints.push(Span::styled("Jump ", sep_style));
        }
        PanelFocus::LeftSidebar => {
            hints.push(Span::styled("jk", key_style));
            hints.push(Span::styled("Navigate ", sep_style));
            hints.push(Span::styled("a", key_style));
            hints.push(Span::styled("Accept ", sep_style));
            if state.game.active_mission.is_some() {
                hints.push(Span::styled("x", key_style));
                hints.push(Span::styled("Abandon ", sep_style));
            }
            hints.push(Span::styled("1/2", key_style));
            hints.push(Span::styled("Tabs ", sep_style));
        }
        PanelFocus::RightSidebar => {
            hints.push(Span::styled("1/2/3", key_style));
            hints.push(Span::styled("Tabs ", sep_style));
            hints.push(Span::styled("c", key_style));
            hints.push(Span::styled("Chat ", sep_style));
        }
        _ => {}
    }

    // Global hints
    hints.push(Span::styled("Tab", key_style));
    hints.push(Span::styled("Focus ", sep_style));
    hints.push(Span::styled(":", key_style));
    hints.push(Span::styled("Cmd ", sep_style));
    hints.push(Span::styled("q", key_style));
    hints.push(Span::styled("Quit", sep_style));

    hints
}

fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    // Center the help panel
    let help_area = centered_rect(60, 70, area);

    frame.render_widget(Clear, help_area);

    let help_text = r#"
BLACKWING TUI - Help

GLOBAL
  q         Quit
  ?         Toggle this help
  Tab       Cycle panel focus
  F12       Toggle debug panel
  :         Command mode
  Esc       Cancel/Close

NAVIGATION (Map)
  h/j/k/l   Move cursor (vim-style)
  Arrows    Move cursor
  Enter     Move ship to cursor
  1-9       Quick-select ship

ACTIONS
  d         Dock at station
  u         Undock
  s         Stop movement
  e         Engage selected target
  r         Retreat/Disengage

MISSIONS
  a         Accept selected mission
  x         Abandon active mission

CHAT
  c/i       Enter chat mode
  Enter     Send message
  Esc       Cancel input

COMMANDS (: mode)
  :dock     Dock at nearest station
  :undock   Undock
  :stop     Stop movement
  :goto x y Move to position
  :chat msg Send chat message
  :debug    Toggle debug panel
  :help     Show this help
  :q        Quit
"#;

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(" Help (press ? to close) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(help, help_area);
}

fn draw_modal(frame: &mut Frame, area: Rect, modal: &Modal, _state: &AppState) {
    let modal_area = centered_rect(50, 30, area);
    frame.render_widget(Clear, modal_area);

    match modal {
        Modal::Help => {
            draw_help_overlay(frame, area);
        }
        Modal::Confirm { title, message, .. } => {
            let text = format!("{}\n\n[y] Yes  [n] No  [Esc] Cancel", message);
            let confirm = Paragraph::new(text).block(
                Block::default()
                    .title(format!(" {} ", title))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            frame.render_widget(confirm, modal_area);
        }
        Modal::MissionChoice => {
            // Handled by mission choice dialog
            widgets::dialogs::draw_mission_choice(frame, modal_area, _state);
        }
        Modal::StationServices => {
            widgets::station_panel::draw(frame, modal_area, _state);
        }
    }
}

fn draw_input_line(frame: &mut Frame, area: Rect, state: &AppState) {
    let input_area = Rect {
        x: area.x,
        y: area.y + area.height - 3,
        width: area.width,
        height: 3,
    };

    frame.render_widget(Clear, input_area);

    let (prefix, buffer) = match state.ui.input_mode {
        InputMode::Insert => ("> ", &state.ui.input_buffer),
        InputMode::Command => (":", &state.ui.command_buffer),
        InputMode::Normal => return,
    };

    let input = Paragraph::new(format!("{}{}_", prefix, buffer))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    frame.render_widget(input, input_area);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
