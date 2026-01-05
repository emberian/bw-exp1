//! ASCII sector map widget

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{AppState, PanelFocus};

/// Sector size in game units
const SECTOR_SIZE: f64 = 1000.0;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.ui.focus == PanelFocus::SectorMap;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = state
        .game
        .sector
        .as_ref()
        .map(|s| format!(" {} ", s.name))
        .unwrap_or_else(|| " Sector Map ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into map area and info area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(inner);

    let map_area = chunks[0];
    let info_area = chunks[1];

    // Create the map grid
    let width = map_area.width as usize;
    let height = map_area.height as usize;

    if width < 3 || height < 3 {
        return;
    }

    // Initialize grid with sparse dots (every 4th cell) for a cleaner look
    let mut grid: Vec<Vec<(char, Style)>> = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    if x % 4 == 0 && y % 2 == 0 {
                        ('.', Style::default().fg(Color::Rgb(40, 40, 40)))
                    } else {
                        (' ', Style::default())
                    }
                })
                .collect()
        })
        .collect();

    // Helper to convert game coords to screen coords
    // Sector bounds are centered at origin (e.g., -500 to 500), so offset by half
    let to_screen = |x: f64, y: f64| -> Option<(usize, usize)> {
        let sx = (((x + SECTOR_SIZE / 2.0) / SECTOR_SIZE) * width as f64) as isize;
        let sy = (((y + SECTOR_SIZE / 2.0) / SECTOR_SIZE) * height as f64) as isize;
        if sx >= 0 && sx < width as isize && sy >= 0 && sy < height as isize {
            Some((sx as usize, sy as usize))
        } else {
            None
        }
    };

    // Draw locations
    for location in &state.game.locations {
        if let Some((sx, sy)) = to_screen(location.position.0, location.position.1) {
            let (ch, style) = match location.location_type.as_str() {
                "Station" => ('S', Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                "Jumpgate" => ('J', Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                "MiningSite" => ('M', Style::default().fg(Color::Yellow)),
                "Debris" => ('#', Style::default().fg(Color::DarkGray)),
                "AsteroidField" => ('A', Style::default().fg(Color::Gray)),
                _ => ('?', Style::default().fg(Color::White)),
            };
            grid[sy][sx] = (ch, style);
        }
    }

    // Draw other ships with number labels
    for (idx, ship) in state.game.ships.iter().enumerate() {
        if let Some((sx, sy)) = to_screen(ship.position.0, ship.position.1) {
            let is_selected = idx == state.ui.ship_selected;
            let style = if ship.is_hostile {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            // Use number for quick selection (1-9)
            let ch = if idx < 9 {
                char::from_digit((idx + 1) as u32, 10).unwrap_or('*')
            } else {
                '*'
            };
            grid[sy][sx] = (ch, style);
        }
    }

    // Draw player ship
    if let Some((sx, sy)) = to_screen(state.game.position.0, state.game.position.1) {
        grid[sy][sx] = (
            '@',
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
    }

    // Draw cursor if active
    if let Some((cx, cy)) = state.ui.map_cursor {
        if let Some((sx, sy)) = to_screen(cx, cy) {
            let (ch, mut style) = grid[sy][sx];
            // Show cursor as 'X' if on empty space
            let ch = if ch == ' ' || ch == '.' { 'X' } else { ch };
            style = style.bg(Color::DarkGray);
            grid[sy][sx] = (ch, style);
        }
    }

    // Render grid
    let lines: Vec<Line> = grid
        .into_iter()
        .map(|row| {
            Line::from(
                row.into_iter()
                    .map(|(ch, style)| Span::styled(ch.to_string(), style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    let map = Paragraph::new(lines);
    frame.render_widget(map, map_area);

    // Draw info/legend area
    draw_info_area(frame, info_area, state);
}

fn draw_info_area(frame: &mut Frame, area: Rect, state: &AppState) {
    // First line: Legend
    let legend = Line::from(vec![
        Span::styled("@", Style::default().fg(Color::Green)),
        Span::styled("You ", Style::default().fg(Color::DarkGray)),
        Span::styled("S", Style::default().fg(Color::Blue)),
        Span::styled("Station ", Style::default().fg(Color::DarkGray)),
        Span::styled("J", Style::default().fg(Color::Magenta)),
        Span::styled("Jump ", Style::default().fg(Color::DarkGray)),
        Span::styled("1-9", Style::default().fg(Color::White)),
        Span::styled("Ships", Style::default().fg(Color::DarkGray)),
    ]);

    // Second line: Selected target info or cursor position
    let info_line = if let Some(ship) = state.game.ships.get(state.ui.ship_selected) {
        let status_color = if ship.is_hostile {
            Color::Red
        } else {
            Color::White
        };
        Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&ship.name, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(
                format!("Hull:{:.0}%", ship.hull_percent),
                Style::default().fg(if ship.hull_percent < 30.0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
            Span::raw(" "),
            Span::styled(&ship.status, Style::default().fg(Color::Yellow)),
        ])
    } else if let Some((cx, cy)) = state.ui.map_cursor {
        Line::from(vec![
            Span::styled("Cursor: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("({:.0}, {:.0})", cx, cy),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" "),
            Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
            Span::styled(" to move", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Pos: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("({:.0}, {:.0})", state.game.position.0, state.game.position.1),
                Style::default().fg(Color::White),
            ),
            Span::raw(" "),
            Span::styled("[hjkl]", Style::default().fg(Color::Cyan)),
            Span::styled(" cursor", Style::default().fg(Color::DarkGray)),
        ])
    };

    let info = Paragraph::new(vec![legend, Line::default(), info_line]);
    frame.render_widget(info, area);
}
