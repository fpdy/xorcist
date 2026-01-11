//! xorcist - A TUI client for jj (Jujutsu VCS).

mod app;
mod error;
mod jj;
mod ui;

use std::env;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use app::{App, View};
use error::XorcistError;
use jj::{JjRunner, fetch_log, find_jj_repo};

fn main() -> Result<()> {
    // Find jj repository
    let current_dir = env::current_dir().context("failed to get current directory")?;
    let repo = find_jj_repo(&current_dir).ok_or(XorcistError::NotInRepo)?;

    // Create runner and fetch log
    let runner = JjRunner::new().with_work_dir(&repo.root);

    // Check if jj is available
    if !runner.is_available() {
        return Err(XorcistError::JjNotFound.into());
    }

    // Fetch log entries
    let entries = fetch_log(&runner, Some(500)).context("failed to fetch jj log")?;

    // Create app state
    let repo_root_display = repo
        .root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| repo.root.to_string_lossy().to_string());

    let app = App::new(entries, repo_root_display, runner);

    // Run TUI
    run_tui(app)
}

/// Run the TUI application.
fn run_tui(mut app: App) -> Result<()> {
    let mut terminal = ratatui::init();

    let result = run_event_loop(&mut terminal, &mut app);

    ratatui::restore();

    result
}

/// Main event loop.
fn run_event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| {
            ui::render(frame, app);
        })?;

        // Handle events
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.view {
                View::Log => handle_log_keys(app, key)?,
                View::Detail => handle_detail_keys(app, key),
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Handle key events in log view.
fn handle_log_keys(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_previous();
        }
        KeyCode::Char('g') | KeyCode::Home => {
            app.select_first();
        }
        KeyCode::Char('G') | KeyCode::End => {
            app.select_last();
        }
        KeyCode::Enter => {
            app.open_detail().context("failed to open detail view")?;
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.page_down(10);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.page_up(10);
        }
        KeyCode::PageDown => {
            app.page_down(10);
        }
        KeyCode::PageUp => {
            app.page_up(10);
        }
        _ => {}
    }
    Ok(())
}

/// Handle key events in detail view.
fn handle_detail_keys(app: &mut App, key: KeyEvent) {
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
