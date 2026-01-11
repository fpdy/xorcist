//! UI rendering with ratatui.

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};

use crate::app::{App, InputMode, ModalState, View};
use crate::jj::{DiffStatus, LogEntry, ShowOutput};

/// Render the entire UI based on current view.
pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Log => render_log_view(frame, app),
        View::Detail => render_detail_view(frame, app),
    }

    // Render input overlay if in input mode
    if app.is_input_mode() {
        render_input_overlay(frame, app);
    }

    // Render help modal on top if visible
    if app.show_help {
        render_help(frame);
    }

    // Render modal dialog if open
    if app.is_modal_open() {
        render_modal_overlay(frame, app);
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
    render_log_status_bar(frame, chunks[2], app);
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
        // Shortest unique prefix: bright magenta + bold
        Span::styled(
            &entry.change_id_prefix,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        // Rest of change ID: dim/dark gray
        Span::styled(&entry.change_id_rest, Style::default().fg(Color::DarkGray)),
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
fn render_log_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    // Show command result if available, otherwise show help text
    let (text, style) = if let Some(result) = &app.last_command_result {
        let color = if result.success {
            Color::Green
        } else {
            Color::Red
        };
        let prefix = if result.success { "✓" } else { "✗" };
        let msg = format!(
            " {prefix} {} ",
            truncate_message(&result.message, area.width as usize - 4)
        );
        (msg, Style::default().bg(Color::DarkGray).fg(color))
    } else {
        let help = " n: new  e: edit  d: describe  b: bookmark  Enter: show  q: quit  ?: help ";
        (
            help.to_string(),
            Style::default().bg(Color::DarkGray).fg(Color::White),
        )
    };

    let status_bar = Paragraph::new(text).style(style);
    frame.render_widget(status_bar, area);
}

/// Truncate a message to fit within the given width.
fn truncate_message(msg: &str, max_width: usize) -> String {
    // Take only the first line
    let first_line = msg.lines().next().unwrap_or(msg);
    if first_line.len() <= max_width {
        first_line.to_string()
    } else {
        format!("{}...", &first_line[..max_width.saturating_sub(3)])
    }
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
            // Shortest unique prefix: bright magenta + bold
            Span::styled(
                output.change_id_prefix.clone(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            // Rest of change ID: dim/dark gray
            Span::styled(
                output.change_id_rest.clone(),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Commit ID: ", Style::default().bold()),
            // Shortest unique prefix: bright yellow + bold
            Span::styled(
                output.commit_id_prefix.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            // Rest of commit ID: dim/dark gray
            Span::styled(
                output.commit_id_rest.clone(),
                Style::default().fg(Color::DarkGray),
            ),
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
    let help_text = " j/k: scroll  Ctrl+d/u: page  q/Esc: back  ?: help ";
    let status_bar =
        Paragraph::new(help_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}

/// Render the help modal.
fn render_help(frame: &mut Frame) {
    let area = centered_rect(frame.area(), 50, 60);

    // Clear the area first to avoid background bleed-through
    frame.render_widget(Clear, area);

    let help_lines = vec![
        Line::styled(
            "─── Keyboard Shortcuts ───",
            Style::default().fg(Color::Cyan).bold(),
        ),
        Line::raw(""),
        Line::styled("  Navigation", Style::default().bold()),
        Line::from(vec![
            Span::styled("  j / ↓      ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑      ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("  g / Home   ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to top"),
        ]),
        Line::from(vec![
            Span::styled("  G / End    ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to bottom"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d     ", Style::default().fg(Color::Yellow)),
            Span::raw("Page down"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u     ", Style::default().fg(Color::Yellow)),
            Span::raw("Page up"),
        ]),
        Line::raw(""),
        Line::styled("  Actions", Style::default().bold()),
        Line::from(vec![
            Span::styled("  Enter      ", Style::default().fg(Color::Yellow)),
            Span::raw("Open detail view"),
        ]),
        Line::from(vec![
            Span::styled("  q          ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit / Close view"),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", Style::default().fg(Color::Yellow)),
            Span::raw("Close detail / help"),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
    ];

    let help_widget = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help "),
    );

    frame.render_widget(help_widget, area);
}

/// Calculate a centered rectangle with given percentage of width and height.
fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

// ─────────────────────────────────────────────────────────────────────────────
// Modal overlay (confirmation dialogs)
// ─────────────────────────────────────────────────────────────────────────────

/// Render the modal overlay for confirmation dialogs.
fn render_modal_overlay(frame: &mut Frame, app: &App) {
    let ModalState::Confirm(action) = &app.modal else {
        return;
    };

    let message = action.confirm_message();

    // Calculate centered area for modal box
    let area = frame.area();
    let width = (message.len() as u16 + 6).max(30).min(area.width - 4);
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    // Clear the area behind the modal box
    frame.render_widget(Clear, modal_area);

    // Build the modal box
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm ")
        .title_style(Style::default().fg(Color::Yellow).bold());

    let inner_area = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Split inner area for message and buttons
    let chunks = Layout::vertical([
        Constraint::Length(1), // Message
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Buttons
    ])
    .split(inner_area);

    // Render message (centered)
    let message_paragraph = Paragraph::new(message).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(message_paragraph, chunks[0]);

    // Render buttons
    let buttons = Line::from(vec![
        Span::styled(" [Y]es ", Style::default().fg(Color::Green).bold()),
        Span::raw("  "),
        Span::styled(" [N]o ", Style::default().fg(Color::Red).bold()),
    ]);
    let buttons_paragraph = Paragraph::new(buttons).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(buttons_paragraph, chunks[2]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Input overlay
// ─────────────────────────────────────────────────────────────────────────────

/// Render the input overlay for text entry.
fn render_input_overlay(frame: &mut Frame, app: &App) {
    let Some(mode) = &app.input_mode else {
        return;
    };

    // Calculate centered area for input box
    let area = frame.area();
    let width = (area.width * 60 / 100).max(40).min(area.width - 4);
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let input_area = Rect::new(x, y, width, height);

    // Clear the area behind the input box
    frame.render_widget(Clear, input_area);

    // Build the input box
    let title = match mode {
        InputMode::Describe => " Describe ",
        InputMode::BookmarkSet => " Set Bookmark ",
        InputMode::NewWithMessage => " New Change ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold());

    let inner_area = block.inner(input_area);
    frame.render_widget(block, input_area);

    // Render the input text
    let input_value = app.input.value();
    let display_text = if input_value.is_empty() {
        Span::styled(mode.placeholder(), Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(input_value)
    };

    // Calculate scroll for long input
    let scroll = app.input.visual_scroll(inner_area.width as usize);
    let input_paragraph = Paragraph::new(Line::from(display_text)).scroll((0, scroll as u16));
    frame.render_widget(input_paragraph, inner_area);

    // Set cursor position
    if !input_value.is_empty() || app.is_input_mode() {
        let cursor_x = app.input.visual_cursor().saturating_sub(scroll);
        frame.set_cursor_position(Position::new(inner_area.x + cursor_x as u16, inner_area.y));
    }
}
