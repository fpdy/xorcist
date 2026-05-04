//! UI rendering with ratatui.

mod detail;
mod diff;
mod log;
mod overlays;

use ratatui::Frame;

use crate::app::{App, View};

/// Render the entire UI based on current view.
pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Log => log::render_log_view(frame, app),
        View::Detail => detail::render_detail_view(frame, app),
        View::Diff => diff::render_diff_view(frame, app),
    }

    // Render input overlay if in input mode
    if app.is_input_mode() {
        overlays::render_input_overlay(frame, app);
    }

    // Render help modal on top if visible
    if app.show_help {
        overlays::render_help(frame);
    }

    // Render modal dialog if open
    if app.is_modal_open() {
        overlays::render_modal_overlay(frame, app);
    }
}
