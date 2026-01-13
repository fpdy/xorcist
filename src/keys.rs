//! Keyboard event handlers.

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

use crate::app::{App, InputMode, View};

/// Handle key events in log view.
pub fn handle_log_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    // Track if we need to check for loading more entries
    let mut check_load_more = false;

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
            check_load_more = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_previous();
        }
        KeyCode::Char('g') | KeyCode::Home => {
            app.select_first();
        }
        KeyCode::Char('G') | KeyCode::End => {
            app.select_last();
            check_load_more = true;
        }
        KeyCode::Enter => {
            app.open_detail().context("failed to open detail view")?;
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.page_down(10);
            check_load_more = true;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.page_up(10);
        }
        KeyCode::PageDown => {
            app.page_down(10);
            check_load_more = true;
        }
        KeyCode::PageUp => {
            app.page_up(10);
        }
        // jj commands with confirmation
        KeyCode::Char('a') => {
            // jj abandon (with confirmation)
            app.show_abandon_confirm();
        }
        KeyCode::Char('s') => {
            // jj squash (with confirmation)
            app.show_squash_confirm();
        }
        KeyCode::Char('f') => {
            // jj git fetch (no confirmation - read-only operation)
            app.execute_git_fetch()
                .context("failed to execute jj git fetch")?;
        }
        KeyCode::Char('p') => {
            // jj git push (with confirmation)
            app.show_push_confirm();
        }
        KeyCode::Char('u') => {
            // jj undo (with confirmation)
            app.show_undo_confirm();
        }
        // Phase1 jj command keys
        KeyCode::Char('n') => {
            // jj new (without message)
            app.execute_new().context("failed to execute jj new")?;
        }
        KeyCode::Char('N') => {
            // jj new -m (with message input)
            app.start_input_mode(InputMode::NewWithMessage);
        }
        KeyCode::Char('e') => {
            // jj edit
            app.execute_edit().context("failed to execute jj edit")?;
        }
        KeyCode::Char('d') => {
            // jj describe -m (input mode)
            app.start_input_mode(InputMode::Describe);
        }
        KeyCode::Char('b') => {
            // jj bookmark set (input mode)
            app.start_input_mode(InputMode::BookmarkSet);
        }
        _ => {}
    }

    // Mark that we should check for loading more entries
    if check_load_more {
        app.request_load_more_check();
    }

    Ok(())
}

/// Handle key events in input mode.
pub fn handle_input_keys(app: &mut App, key: KeyEvent, event: &Event) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            app.submit_input().context("failed to submit input")?;
        }
        KeyCode::Esc => {
            app.cancel_input_mode();
        }
        _ => {
            // Pass other keys to tui-input
            app.input.handle_event(event);
        }
    }
    Ok(())
}

/// Handle key events in modal dialog.
pub fn handle_modal_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_action().context("failed to execute action")?;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.close_modal();
        }
        _ => {}
    }
    Ok(())
}

/// Handle key events in detail view.
pub fn handle_detail_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.close_detail();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.detail_scroll_down(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.detail_scroll_up(1);
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.detail_scroll_down(10);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.detail_scroll_up(10);
        }
        KeyCode::PageDown => {
            app.detail_scroll_down(10);
        }
        KeyCode::PageUp => {
            app.detail_scroll_up(10);
        }
        _ => {}
    }
}

/// Dispatch key event to appropriate handler based on app state.
///
/// Returns `true` if the event was fully handled (e.g., help toggle),
/// meaning the caller should `continue` the event loop.
pub fn dispatch_key_event(app: &mut App, key: KeyEvent, event: &Event) -> Result<bool> {
    // Handle ? key globally for help toggle
    if key.code == KeyCode::Char('?') {
        app.toggle_help();
        return Ok(true);
    }

    // If help is showing, close it and execute the command
    if app.show_help {
        if key.code == KeyCode::Esc {
            app.close_help();
            return Ok(true);
        }
        // Close help and fall through to execute the command
        app.close_help();
    }

    // Modal dialog takes highest priority
    if app.is_modal_open() {
        handle_modal_keys(app, key)?;
    } else if app.is_input_mode() {
        // Input mode takes second priority
        handle_input_keys(app, key, event)?;
    } else {
        match app.view {
            View::Log => handle_log_keys(app, key)?,
            View::Detail => handle_detail_keys(app, key),
        }
    }

    Ok(false)
}
