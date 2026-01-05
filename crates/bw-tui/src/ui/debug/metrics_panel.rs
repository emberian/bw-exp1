//! Tick metrics visualization

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{BarChart, Paragraph},
};

use crate::state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(3)])
        .split(area);

    // Current tick info
    draw_current_tick(frame, chunks[0], state);

    // Historical chart
    draw_history_chart(frame, chunks[1], state);
}

fn draw_current_tick(frame: &mut Frame, area: Rect, state: &AppState) {
    let lines = if let Some(ref metrics) = state.game.tick_metrics {
        let budget_pct = (metrics.total_us as f64 / metrics.budget_us as f64 * 100.0) as u32;
        let status_style = if metrics.over_budget {
            Style::default().fg(Color::Red)
        } else if budget_pct > 80 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        vec![
            Line::from(vec![
                Span::raw("Tick "),
                Span::styled(
                    format!("{}", metrics.tick),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" | "),
                Span::styled(
                    format!("{:.2}ms", metrics.total_us as f64 / 1000.0),
                    status_style,
                ),
                Span::raw(" / "),
                Span::styled(
                    format!("{:.0}ms budget", metrics.budget_us as f64 / 1000.0),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(format!(" ({}%)", budget_pct)),
            ]),
            Line::from(
                metrics
                    .phases
                    .iter()
                    .map(|p| {
                        Span::styled(
                            format!(" {}: {:.1}ms ", p.name, p.duration_us as f64 / 1000.0),
                            Style::default().fg(Color::White),
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
            Line::from(vec![
                Span::styled("Sectors: ", Style::default().fg(Color::DarkGray)),
                Span::raw(
                    metrics
                        .sectors
                        .iter()
                        .map(|s| format!("{}:{:.1}ms", s.sector_name, s.total_us as f64 / 1000.0))
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No metrics - use :metrics to subscribe",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn draw_history_chart(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(ref history) = state.game.tick_metrics_history {
        // Create bar chart data from recent ticks
        let data: Vec<(&str, u64)> = history
            .ticks
            .iter()
            .rev()
            .take(area.width as usize / 3)
            .map(|t| ("", t.total_us / 1000)) // Convert to ms
            .collect();

        if !data.is_empty() {
            let chart = BarChart::default()
                .data(&data)
                .bar_width(2)
                .bar_gap(1)
                .bar_style(Style::default().fg(Color::Cyan))
                .value_style(Style::default().fg(Color::White));

            frame.render_widget(chart, area);
        }

        // Summary stats at bottom
        let stats = Line::from(vec![
            Span::styled("Avg: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.1}ms", history.avg_duration_us as f64 / 1000.0)),
            Span::raw(" | "),
            Span::styled("P95: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.1}ms", history.p95_duration_us as f64 / 1000.0)),
            Span::raw(" | "),
            Span::styled("Max: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.1}ms", history.max_duration_us as f64 / 1000.0)),
            Span::raw(" | "),
            Span::styled("Over budget: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", history.over_budget_count),
                if history.over_budget_count > 0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]);

        let stats_area = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        frame.render_widget(Paragraph::new(stats), stats_area);
    } else {
        let empty = Paragraph::new("Subscribe to metrics with :metrics command")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, area);
    }
}
