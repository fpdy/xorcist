//! UI rendering with ratatui.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

use crate::app::{App, View};
use crate::jj::{DiffStatus, LogEntry, ShowOutput};

/// Render the entire UI based on current view.
pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Log => render_log_view(frame, app),
        View::Detail => render_detail_view(frame, app),
    }
}

/// Render the log view.
fn render_log_view(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Title bar
        Constraint::Min(3),    // Log list
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_title_bar(frame, chunks[0], app);
    render_log_list(frame, chunks[1], app);
    render_log_status_bar(frame, chunks[2]);
}

/// Render the title bar.
fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(" xorcist - {} ", app.repo_root);
    let title_bar = Paragraph::new(title).style(Style::default().bg(Color::Blue).fg(Color::White));
    frame.render_widget(title_bar, area);
}

/// Render the log list.
fn render_log_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|entry| create_list_item(entry))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Create a list item from a log entry.
fn create_list_item(entry: &LogEntry) -> ListItem<'_> {
    let symbol = entry.graph_symbol();
    let symbol_style = if entry.is_working_copy {
        Style::default().fg(Color::Green).bold()
    } else if entry.is_immutable {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let mut spans = vec![
        Span::styled(format!("{symbol} "), symbol_style),
        Span::styled(&entry.change_id, Style::default().fg(Color::Magenta)),
        Span::raw(" "),
    ];

    // Add bookmarks if present
    if !entry.bookmarks.is_empty() {
        let bookmarks_str = entry.bookmarks.join(" ");
        spans.push(Span::styled(
            format!("[{bookmarks_str}] "),
            Style::default().fg(Color::Cyan),
        ));
    }

    // Description
    let desc_style = if entry.is_empty {
        Style::default().fg(Color::DarkGray).italic()
    } else {
        Style::default()
    };
    spans.push(Span::styled(&entry.description, desc_style));

    // Author and timestamp (right-aligned conceptually, but we just append)
    spans.push(Span::raw(" "));
    spans.push(Span::styled(
        format!("{} ", entry.author),
        Style::default().fg(Color::Cyan),
    ));
    spans.push(Span::styled(
        &entry.timestamp,
        Style::default().fg(Color::DarkGray),
    ));

    ListItem::new(Line::from(spans))
}

/// Render the status bar for log view.
fn render_log_status_bar(frame: &mut Frame, area: Rect) {
    let help_text = " j/k: navigate  g/G: top/bottom  Enter: show  q: quit ";
    let status_bar =
        Paragraph::new(help_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}

/// Render the detail view.
fn render_detail_view(frame: &mut Frame, app: &mut App) {
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
fn render_detail_content(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(state) = &app.detail_state else {
        return;
    };

    // Build content lines
    let lines = build_detail_lines(&state.show_output);
    let content_height = lines.len();

    // Update content height in app state
    app.set_detail_content_height(content_height);

    // Get current scroll position (re-borrow after mutation)
    let scroll = app.detail_state.as_ref().map(|s| s.scroll).unwrap_or(0);

    // Calculate visible height for scroll clamping
    let visible_height = area.height.saturating_sub(0) as usize;
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

/// Build lines for detail view content.
fn build_detail_lines(output: &ShowOutput) -> Vec<Line<'static>> {
    // Header section
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Change ID: ", Style::default().bold()),
            Span::styled(
                output.change_id.clone(),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("Commit ID: ", Style::default().bold()),
            Span::styled(output.commit_id.clone(), Style::default().fg(Color::Yellow)),
        ]),
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

    // Description
    lines.push(Line::styled(
        "─── Description ───",
        Style::default().fg(Color::DarkGray),
    ));
    for desc_line in output.description.lines() {
        lines.push(Line::raw(desc_line.to_string()));
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
        let (symbol, color) = match entry.status {
            DiffStatus::Added => ("+", Color::Green),
            DiffStatus::Modified => ("~", Color::Yellow),
            DiffStatus::Deleted => ("-", Color::Red),
            DiffStatus::Renamed => ("→", Color::Cyan),
            DiffStatus::Copied => ("⊕", Color::Blue),
        };
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

/// Render the status bar for detail view.
fn render_detail_status_bar(frame: &mut Frame, area: Rect) {
    let help_text = " j/k: scroll  Ctrl+d/u: page  q/Esc: back ";
    let status_bar =
        Paragraph::new(help_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
