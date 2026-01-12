//! xorcist - A TUI client for jj (Jujutsu VCS).

mod app;
mod conventional;
mod error;
mod jj;
mod ui;

use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

use app::{App, InputMode, View};
use error::XorcistError;
use jj::{JjRunner, fetch_log, find_jj_repo};

/// A TUI client for jj (Jujutsu VCS).
#[derive(Parser, Debug)]
#[command(name = "xor", version, about)]
struct Args {
    /// Maximum number of log entries to load.
    /// Use --all to load the entire history.
    #[arg(short = 'n', long, default_value = "500")]
    limit: usize,

    /// Load all history (may be slow on large repositories).
    #[arg(long)]
    all: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Find jj repository
    let current_dir = env::current_dir().context("failed to get current directory")?;
    let repo = find_jj_repo(&current_dir).ok_or(XorcistError::NotInRepo)?;

    // Create runner and fetch log
    let runner = JjRunner::new().with_work_dir(&repo.root);

    // Check if jj is available
    if !runner.is_available() {
        return Err(XorcistError::JjNotFound.into());
    }

    // Determine limit: --all overrides --limit
    let limit = if args.all { None } else { Some(args.limit) };

    // Fetch log entries
    let entries = fetch_log(&runner, limit).context("failed to fetch jj log")?;

    // Create app state
    let repo_root_display = repo
        .root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| repo.root.to_string_lossy().to_string());

    let mut app = App::new(entries, repo_root_display, runner);
    app.set_log_limit(limit);

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

        // Check if we need to load more entries (after drawing, so "Loading..." is visible)
        if app.should_load_more() {
            app.start_loading();
            // Redraw to show "Loading..." status
            terminal.draw(|frame| {
                ui::render(frame, app);
            })?;
            // Now perform the actual load
            app.load_more_entries()
                .context("failed to load more entries")?;
        }

        // Handle events
        let event = event::read()?;
        if let Event::Key(key) = &event
            && key.kind == KeyEventKind::Press
        {
            // Handle ? key globally for help toggle
            if key.code == KeyCode::Char('?') {
                app.toggle_help();
                continue;
            }

            // If help is showing, close it and execute the command
            if app.show_help {
                if key.code == KeyCode::Esc {
                    app.close_help();
                    continue;
                }
                // Close help and fall through to execute the command
                app.close_help();
            }

            // Modal dialog takes highest priority
            if app.is_modal_open() {
                handle_modal_keys(app, *key)?;
            } else if app.is_input_mode() {
                // Input mode takes second priority
                handle_input_keys(app, *key, &event)?;
            } else {
                match app.view {
                    View::Log => handle_log_keys(app, *key)?,
                    View::Detail => handle_detail_keys(app, *key),
                }
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
fn handle_input_keys(app: &mut App, key: KeyEvent, event: &Event) -> Result<()> {
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
fn handle_modal_keys(app: &mut App, key: KeyEvent) -> Result<()> {
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
