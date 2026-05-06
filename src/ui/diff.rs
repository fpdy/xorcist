use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app::App;
use crate::ui::diff_status_presentation;

/// Minimum width for horizontal (side-by-side) diff layout.
pub(super) const MIN_WIDTH_FOR_HORIZONTAL_DIFF: u16 = 120;

/// Render the diff view.
pub(super) fn render_diff_view(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Title bar
        Constraint::Min(3),    // Content
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Title bar
    let change_id_short = if app.diff_state.change_id.len() >= 8 {
        &app.diff_state.change_id[..8]
    } else {
        &app.diff_state.change_id
    };
    let title = format!(" Diff: {change_id_short} ");
    let title_bar = Paragraph::new(title).style(Style::default().bg(Color::Green).fg(Color::Black));
    frame.render_widget(title_bar, chunks[0]);

    // Content: responsive layout based on width
    // Ratio is 2:3 (file list : diff text)
    let content_area = chunks[1];
    if content_area.width >= MIN_WIDTH_FOR_HORIZONTAL_DIFF {
        // Horizontal (side-by-side) layout for wide terminals
        let content_chunks = Layout::horizontal([
            Constraint::Ratio(2, 5), // File list (2/5 = 40%)
            Constraint::Ratio(3, 5), // Diff text (3/5 = 60%)
        ])
        .split(content_area);

        render_diff_file_list(frame, content_chunks[0], app);
        render_diff_text(frame, content_chunks[1], app);
    } else {
        // Vertical (stacked) layout for narrow terminals
        let content_chunks = Layout::vertical([
            Constraint::Ratio(2, 5), // File list (2/5 = 40%)
            Constraint::Ratio(3, 5), // Diff text (3/5 = 60%)
        ])
        .split(content_area);

        render_diff_file_list(frame, content_chunks[0], app);
        render_diff_text(frame, content_chunks[1], app);
    }

    // Status bar
    render_diff_status_bar(frame, chunks[2]);
}

/// Render the file list in diff view.
fn render_diff_file_list(frame: &mut Frame, area: Rect, app: &App) {
    let state = &app.diff_state;
    let mut lines: Vec<Line> = Vec::new();

    for (idx, entry) in state.files.iter().enumerate() {
        let (symbol, color) = diff_status_presentation(entry.status);

        let is_selected = idx == state.selected;
        let path_style = if is_selected {
            Style::default().bg(Color::Indexed(236)).bold()
        } else {
            Style::default()
        };

        let line = Line::from(vec![
            Span::styled(format!(" {symbol} "), Style::default().fg(color).bold()),
            Span::styled(entry.path.clone(), path_style),
        ]);
        lines.push(if is_selected {
            line.bg(Color::Indexed(236))
        } else {
            line
        });
    }

    if state.files.is_empty() {
        lines.push(Line::styled(
            "  (no changes)",
            Style::default().fg(Color::DarkGray).italic(),
        ));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::RIGHT).title(" Files "))
        .scroll((state.file_scroll as u16, 0));
    frame.render_widget(paragraph, area);
}

/// Render the diff text in diff view.
fn render_diff_text(frame: &mut Frame, area: Rect, app: &App) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    let state = &app.diff_state;
    let h_scroll = state.diff_h_scroll;
    let v_scroll = state.diff_scroll;

    let lines: Vec<Line> = state
        .diff_lines
        .iter()
        .map(|line| {
            let style = if line.starts_with('+') && !line.starts_with("+++") {
                Style::default().fg(Color::Green)
            } else if line.starts_with('-') && !line.starts_with("---") {
                Style::default().fg(Color::Red)
            } else if line.starts_with("@@") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            Line::styled(line.clone(), style)
        })
        .collect();

    if lines.is_empty() {
        let empty_msg = Paragraph::new("  (select a file to view diff)")
            .style(Style::default().fg(Color::DarkGray).italic())
            .block(Block::default().borders(Borders::ALL).title(" Diff "));
        frame.render_widget(empty_msg, area);
        return;
    }

    // Build title with scroll indicator
    let title = if h_scroll > 0 {
        format!(" Diff (←{}) ", h_scroll)
    } else {
        " Diff ".to_string()
    };

    // Use Paragraph::scroll for both vertical and horizontal scrolling
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((v_scroll as u16, h_scroll as u16));
    frame.render_widget(paragraph, area);

    // Scrollbar
    let content_height = state.diff_lines.len();
    if content_height > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state =
            ScrollbarState::new(content_height.saturating_sub(visible_height)).position(v_scroll);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Render the status bar for diff view.
fn render_diff_status_bar(frame: &mut Frame, area: Rect) {
    let help_text = " j/k: select file  Ctrl+d/u: scroll  ←/→: pan  q/Esc: back  ?: help ";
    let status_bar =
        Paragraph::new(help_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
