//! UI rendering with ratatui.

mod detail;
mod diff;
mod log;
mod overlays;

use ratatui::{Frame, layout::Constraint, layout::Layout, style::Color};

use crate::{
    app::{App, View},
    jj::DiffStatus,
};

fn diff_status_presentation(status: DiffStatus) -> (&'static str, Color) {
    match status {
        DiffStatus::Added => ("+", Color::Green),
        DiffStatus::Modified => ("~", Color::Yellow),
        DiffStatus::Deleted => ("-", Color::Red),
        DiffStatus::Renamed => ("→", Color::Cyan),
        DiffStatus::Copied => ("⊕", Color::Blue),
    }
}

/// Render the entire UI based on current view.
pub fn render(frame: &mut Frame, app: &mut App) {
    update_layout_state(frame, app);

    match app.view {
        View::Log => log::render_log_view(frame, app),
        View::Detail => detail::render_detail_view(frame, app),
        View::Diff => diff::render_diff_view(frame, app),
    }

    // Render help with active-view priority, below modal and input overlays.
    if app.show_help {
        overlays::render_help(frame);
    }

    // Render modal above help, matching key priority.
    if app.is_modal_open() {
        overlays::render_modal_overlay(frame, app);
    }

    // Render input overlay last because input mode has highest key priority.
    if app.is_input_mode() {
        overlays::render_input_overlay(frame, app);
    }
}

fn update_layout_state(frame: &Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(frame.area());

    match app.view {
        View::Log => app.ensure_selected_visible(chunks[1].height as usize),
        View::Detail => {
            if let Some(height) = detail::content_height(app) {
                app.set_detail_content_height(height);
            }
        }
        View::Diff => {
            let content_area = chunks[1];
            let content_chunks = if content_area.width >= diff::MIN_WIDTH_FOR_HORIZONTAL_DIFF {
                Layout::horizontal([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                    .split(content_area)
            } else {
                Layout::vertical([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                    .split(content_area)
            };
            app.ensure_diff_file_visible(content_chunks[0].height.saturating_sub(1) as usize);
            app.clamp_diff_scroll(content_chunks[1].height.saturating_sub(2) as usize);
            app.clamp_diff_h_scroll(content_chunks[1].width.saturating_sub(2) as usize);
        }
    }
}
