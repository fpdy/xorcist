use ansi_to_tui::IntoText;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app::App;

/// Render the log view.
pub(super) fn render_log_view(frame: &mut Frame, app: &mut App) {
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

/// Render the log list with ANSI graph output.
fn render_log_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let viewport_height = area.height as usize;

    // Ensure selected line is visible
    app.ensure_selected_visible(viewport_height);

    // Get the selected line index for highlighting
    let selected_line_idx = app.selected_line_index();

    // Build text from graph lines
    let mut lines: Vec<Line> = Vec::new();

    for (idx, graph_line) in app.graph_log.lines.iter().enumerate() {
        // Parse ANSI codes to ratatui Line
        let text = graph_line
            .raw
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::raw(&graph_line.raw));

        // Get the first line (should only be one line per graph_line)
        let mut line = if text.lines.is_empty() {
            Line::raw("")
        } else {
            text.lines.into_iter().next().unwrap()
        };

        // Transform description for commit lines
        if let Some(ref desc) = graph_line.description {
            line = transform_line_description(line, &graph_line.plain, desc);
        }

        // Highlight selected line
        if Some(idx) == selected_line_idx {
            // Apply background color to indicate selection
            line = line.bg(Color::Indexed(236)).bold();
        }

        lines.push(line);
    }

    let paragraph = Paragraph::new(lines).scroll((app.scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);

    // Scrollbar
    let total_lines = app.line_count();
    if total_lines > viewport_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(viewport_height))
            .position(app.scroll_offset);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Transform a line's description part.
///
/// - Empty description: replace with "(no desc)" in DarkGray italic
/// - Conventional commits: convert to emoji format
fn transform_line_description<'a>(line: Line<'a>, plain: &str, description: &str) -> Line<'a> {
    // Handle empty description: keep all original spans and append "(no desc)"
    if description.is_empty() {
        let mut spans: Vec<Span<'a>> = line.spans.into_iter().collect();
        // Prevent selection highlight from filling this separator space.
        // (Spans with explicit bg=Reset override the line background.)
        spans.push(Span::styled(" ", Style::default().bg(Color::Reset)));
        spans.push(Span::styled(
            "(no desc)",
            Style::default().fg(Color::DarkGray).italic(),
        ));
        return Line::from(spans);
    }

    // Find the position of description in the plain text
    let desc_start = match plain.find(description) {
        Some(pos) => pos,
        None => return line, // Description not found, return unchanged
    };

    // Calculate prefix (everything before description)
    let prefix_plain = &plain[..desc_start];

    // Find how many characters to keep from the original spans
    let mut prefix_char_count = 0;
    let mut prefix_spans: Vec<Span<'a>> = Vec::new();

    for span in line.spans.into_iter() {
        let span_len = span.content.chars().count();
        let prefix_remaining = prefix_plain
            .chars()
            .count()
            .saturating_sub(prefix_char_count);

        if prefix_remaining == 0 {
            // We've collected all prefix spans, skip the rest (description part)
            break;
        } else if span_len <= prefix_remaining {
            // This entire span is part of the prefix
            prefix_char_count += span_len;
            prefix_spans.push(span);
        } else {
            // This span partially overlaps - split it
            let chars: String = span.content.chars().take(prefix_remaining).collect();
            prefix_spans.push(Span::styled(chars, span.style));
            break;
        }
    }

    // Apply conventional commits emoji transformation
    let transformed = crate::conventional::format_commit_message(description);
    if transformed != description {
        // Conventional commit was transformed - use default style for emoji text
        prefix_spans.push(Span::raw(transformed));
    } else {
        // Not a conventional commit - keep original style (just use raw text)
        prefix_spans.push(Span::raw(description.to_string()));
    }

    Line::from(prefix_spans)
}

/// Render the status bar for log view.
fn render_log_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    // Show command result if available, otherwise show help text
    let (text, style) = if app.is_loading_more {
        (
            " Loading more entries... ".to_string(),
            Style::default().bg(Color::DarkGray).fg(Color::Yellow),
        )
    } else if let Some(result) = &app.last_command_result {
        let color = if result.success {
            Color::Green
        } else {
            Color::Red
        };
        let prefix = if result.success { "✓" } else { "✗" };
        let msg = format!(
            " {prefix} {} ",
            truncate_message(&result.message, (area.width as usize).saturating_sub(4))
        );
        (msg, Style::default().bg(Color::DarkGray).fg(color))
    } else {
        // Build help text with entry count info
        let count_info = if app.has_more_entries {
            format!("[{}+ commits] ", app.commit_count())
        } else {
            format!("[{} commits] ", app.commit_count())
        };
        let help = format!(
            " {count_info}n: new  e: edit  d: describe  b: bookmark  r: rebase  Enter: show  ?: help "
        );
        (help, Style::default().bg(Color::DarkGray).fg(Color::White))
    };

    let status_bar = Paragraph::new(text).style(style);
    frame.render_widget(status_bar, area);
}

/// Truncate a message (first line only) to fit within the given display width.
fn truncate_message(msg: &str, max_width: usize) -> String {
    let first_line = msg.lines().next().unwrap_or(msg);
    crate::text::truncate_str(first_line, max_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_message_ascii() {
        assert_eq!(truncate_message("hello world", 8), "hello...");
        assert_eq!(truncate_message("short", 10), "short");
    }

    #[test]
    fn test_truncate_message_japanese() {
        // Japanese characters: 1 char = 2 width
        assert_eq!(truncate_message("日本語", 10), "日本語"); // 6 width
        assert_eq!(truncate_message("日本語テスト", 10), "日本語..."); // 12 width -> truncate
    }

    #[test]
    fn test_truncate_message_multiline() {
        // Should only use the first line
        assert_eq!(truncate_message("first\nsecond", 20), "first");
        assert_eq!(truncate_message("日本語\n英語", 20), "日本語");
    }

    #[test]
    fn test_truncate_message_mixed() {
        assert_eq!(truncate_message("Hello世界", 10), "Hello世界"); // 5 + 4 = 9 width
        // "Error: 失敗しました" = 7 + 10 = 17 width, truncate to 15 → "Error: 失敗..." (7 + 4 + 3 = 14)
        assert_eq!(
            truncate_message("Error: 失敗しました", 15),
            "Error: 失敗..."
        );
    }
}
