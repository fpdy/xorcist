use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::actions::{
    ActionSpec, DIFF_ACTIONS, GENERAL_ACTIONS, JJ_COMMAND_ACTIONS, NAVIGATION_ACTIONS, action_by_id,
};
use crate::app::{App, InputMode, ModalState};

/// Render the help modal.
pub(super) fn render_help(frame: &mut Frame) {
    let area = centered_rect(frame.area(), 50, 80);

    // Clear the area first to avoid background bleed-through
    frame.render_widget(Clear, area);

    let help_lines = build_help_lines();

    let help_widget = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help "),
    );

    frame.render_widget(help_widget, area);
}

fn build_help_lines() -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled(
        "─── Keyboard Shortcuts ───",
        Style::default().fg(Color::Cyan).bold(),
    )];

    push_section(&mut lines, "Navigation", NAVIGATION_ACTIONS);
    push_section(&mut lines, "jj Commands", JJ_COMMAND_ACTIONS);

    lines.push(Line::raw(""));
    lines.push(Line::styled("  Detail View", Style::default().bold()));
    push_action(
        &mut lines,
        action_by_id("detail.open_diff").expect("known action"),
    );
    push_action(
        &mut lines,
        action_by_id("view.page_down").expect("known action"),
    );
    push_action(
        &mut lines,
        action_by_id("view.page_up").expect("known action"),
    );
    push_action(
        &mut lines,
        action_by_id("detail.back").expect("known action"),
    );

    push_section(&mut lines, "Diff View", DIFF_ACTIONS);
    push_section(&mut lines, "General", GENERAL_ACTIONS);

    lines
}

fn push_section(lines: &mut Vec<Line<'static>>, title: &'static str, actions: &[ActionSpec]) {
    lines.push(Line::raw(""));
    lines.push(Line::styled(format!("  {title}"), Style::default().bold()));
    for action in actions {
        push_action(lines, action);
    }
}

fn push_action(lines: &mut Vec<Line<'static>>, action: &ActionSpec) {
    debug_assert!(action.has_valid_metadata());
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<20}", action.key_label),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(action.help),
    ]));
}

/// Calculate a centered rectangle with given percentage of width and height.
fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// Render the modal overlay for confirmation dialogs.
pub(super) fn render_modal_overlay(frame: &mut Frame, app: &App) {
    let ModalState::Confirm(action) = &app.modal else {
        return;
    };

    let message = action.confirm_message();

    // Calculate centered area for modal box
    let area = frame.area();
    let width = (message.len() as u16 + 6)
        .max(30)
        .min(area.width.saturating_sub(4));
    let height = 5.min(area.height);
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

/// Render the input overlay for text entry.
pub(super) fn render_input_overlay(frame: &mut Frame, app: &App) {
    let Some(mode) = &app.input_mode else {
        return;
    };

    // Calculate centered area for input box
    let area = frame.area();
    let width = (area.width.saturating_mul(60) / 100)
        .max(40)
        .min(area.width.saturating_sub(4));
    let height = 3.min(area.height);
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
        InputMode::RebaseDestination => " Rebase to ",
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
        let cursor_x = (cursor_x as u16).min(inner_area.width.saturating_sub(1));
        frame.set_cursor_position(Position::new(
            inner_area.x.saturating_add(cursor_x),
            inner_area.y,
        ));
    }
}
