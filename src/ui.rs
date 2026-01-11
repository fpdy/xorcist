//! UI rendering with ratatui.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::App;
use crate::jj::LogEntry;

/// Render the entire UI.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Title bar
        Constraint::Min(3),    // Log list
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_title_bar(frame, chunks[0], app);
    render_log_list(frame, chunks[1], app);
    render_status_bar(frame, chunks[2]);
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

/// Render the status bar.
fn render_status_bar(frame: &mut Frame, area: Rect) {
    let help_text = " j/k: navigate  g/G: top/bottom  q: quit ";
    let status_bar =
        Paragraph::new(help_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
