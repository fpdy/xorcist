//! xorcist - A TUI client for jj (Jujutsu VCS).

mod app;
mod conventional;
mod error;
mod jj;
mod keys;
mod text;
mod ui;

use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};

use app::App;
use error::XorcistError;
use jj::{JjRunner, fetch_graph_log, find_jj_repo};

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

    // Fetch graph log
    let graph_log = fetch_graph_log(&runner, limit).context("failed to fetch jj log")?;

    // Create app state
    let repo_root_display = repo
        .root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| repo.root.to_string_lossy().to_string());

    let mut app = App::new(graph_log, repo_root_display, runner);
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
            && keys::dispatch_key_event(app, *key, &event)?
        {
            continue;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
