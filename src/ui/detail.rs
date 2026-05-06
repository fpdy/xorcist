use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app::App;
use crate::{jj::ShowOutput, ui::diff_status_presentation};

/// Render the detail view.
pub(super) fn render_detail_view(frame: &mut Frame, app: &App) {
    let Some(state) = &app.detail_state else {
        return;
    };

    let chunks = Layout::vertical([
        Constraint::Length(1), // Title bar
        Constraint::Min(3),    // Content
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Title bar
    let change_id_short = &state.show_output.change_id[..8.min(state.show_output.change_id.len())];
    let title = format!(" Revision: {change_id_short} ");
    let title_bar =
        Paragraph::new(title).style(Style::default().bg(Color::Magenta).fg(Color::White));
    frame.render_widget(title_bar, chunks[0]);

    // Content area
    let content_area = chunks[1];
    render_detail_content(frame, content_area, app);

    // Status bar
    render_detail_status_bar(frame, chunks[2]);
}

/// Render the detail content with scrolling.
fn render_detail_content(frame: &mut Frame, area: Rect, app: &App) {
    let Some(state) = &app.detail_state else {
        return;
    };

    // Build content lines
    let lines = build_detail_lines(&state.show_output);
    let content_height = lines.len();

    // Get current scroll position.
    let scroll = app.detail_state.as_ref().map(|s| s.scroll).unwrap_or(0);

    let visible_height = area.height as usize;
    let max_scroll = content_height.saturating_sub(visible_height);
    let clamped_scroll = scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .scroll((clamped_scroll as u16, 0))
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
    frame.render_widget(paragraph, area);

    // Scrollbar
    if content_height > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state =
            ScrollbarState::new(content_height.saturating_sub(visible_height))
                .position(clamped_scroll);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn styled_id_line(label: &'static str, prefix: &str, rest: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(label, Style::default().bold()),
        Span::styled(
            prefix.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(rest.to_string(), Style::default().fg(Color::DarkGray)),
    ])
}

/// Build lines for detail view content.
fn build_detail_lines(output: &ShowOutput) -> Vec<Line<'static>> {
    let mut lines = vec![
        styled_id_line(
            "Change ID: ",
            &output.change_id_prefix,
            &output.change_id_rest,
            Color::Magenta,
        ),
        styled_id_line(
            "Commit ID: ",
            &output.commit_id_prefix,
            &output.commit_id_rest,
            Color::Yellow,
        ),
        Line::from(vec![
            Span::styled("Author:    ", Style::default().bold()),
            Span::styled(output.author.clone(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Date:      ", Style::default().bold()),
            Span::raw(output.timestamp.clone()),
        ]),
    ];

    if !output.bookmarks.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Bookmarks: ", Style::default().bold()),
            Span::styled(
                output.bookmarks.join(", "),
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    lines.push(Line::raw(""));

    // Description (first line gets emoji conversion)
    lines.push(Line::styled(
        "─── Description ───",
        Style::default().fg(Color::DarkGray),
    ));
    let mut desc_lines = output.description.lines();
    if let Some(first_line) = desc_lines.next() {
        // Apply conventional commits emoji to first line only
        let formatted = crate::conventional::format_commit_message(first_line);
        lines.push(Line::raw(formatted));
        // Remaining lines as-is
        for desc_line in desc_lines {
            lines.push(Line::raw(desc_line.to_string()));
        }
    }
    if output.description.is_empty() {
        lines.push(Line::styled(
            "(no description)",
            Style::default().fg(Color::DarkGray).italic(),
        ));
    }

    lines.push(Line::raw(""));

    // Diff summary
    lines.push(Line::styled(
        "─── Changed Files ───",
        Style::default().fg(Color::DarkGray),
    ));
    for entry in &output.diff_summary {
        let (symbol, color) = diff_status_presentation(entry.status);
        lines.push(Line::from(vec![
            Span::styled(format!(" {symbol} "), Style::default().fg(color).bold()),
            Span::raw(entry.path.clone()),
        ]));
    }

    if output.diff_summary.is_empty() {
        lines.push(Line::styled(
            "  (no changes)",
            Style::default().fg(Color::DarkGray).italic(),
        ));
    }

    lines
}

pub(super) fn content_height(app: &App) -> Option<usize> {
    app.detail_state
        .as_ref()
        .map(|state| build_detail_lines(&state.show_output).len())
}

/// Render the status bar for detail view.
fn render_detail_status_bar(frame: &mut Frame, area: Rect) {
    let help_text = " j/k: scroll  d: view diff  Ctrl+d/u: page  q/Esc: back  ?: help ";
    let status_bar =
        Paragraph::new(help_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
