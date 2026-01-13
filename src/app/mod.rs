//! Application state management.

mod commands;
mod input;
mod loading;
mod navigation;

#[cfg(test)]
mod tests;

use tui_input::Input;

use crate::error::XorcistError;
use crate::jj::{GraphLog, JjRunner, ShowOutput, fetch_show};
use crate::text::truncate_str;

/// Current view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Log,
    Detail,
}

/// Input mode for text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Entering description for `jj describe -m`.
    Describe,
    /// Entering bookmark name for `jj bookmark set`.
    BookmarkSet,
    /// Creating new change with message for `jj new -m`.
    NewWithMessage,
}

impl InputMode {
    /// Get the placeholder text for this input mode.
    pub fn placeholder(&self) -> &'static str {
        match self {
            InputMode::Describe => "Enter commit message...",
            InputMode::BookmarkSet => "Enter bookmark name...",
            InputMode::NewWithMessage => "Enter message (empty for no message)...",
        }
    }
}

/// State for detail view.
#[derive(Debug, Clone)]
pub struct DetailState {
    /// The ShowOutput being displayed.
    pub show_output: ShowOutput,
    /// Vertical scroll offset.
    pub scroll: usize,
    /// Total content height (for scroll calculation).
    pub content_height: usize,
}

/// Pending action for confirmation dialog.
#[derive(Debug, Clone)]
pub enum PendingAction {
    /// Abandon a change.
    Abandon {
        change_id: String,
        description: String,
    },
    /// Squash a change into its parent.
    Squash {
        change_id: String,
        description: String,
    },
    /// Push to remote.
    GitPush,
    /// Undo the last operation.
    Undo,
}

impl PendingAction {
    /// Get the confirmation message for this action.
    pub fn confirm_message(&self) -> String {
        match self {
            PendingAction::Abandon { description, .. } => {
                format!("Abandon change: \"{}\"?", truncate_str(description, 40))
            }
            PendingAction::Squash { description, .. } => {
                format!(
                    "Squash change: \"{}\" into parent?",
                    truncate_str(description, 40)
                )
            }
            PendingAction::GitPush => "Push to remote?".to_string(),
            PendingAction::Undo => "Undo last operation?".to_string(),
        }
    }
}

/// Modal dialog state.
#[derive(Debug, Clone, Default)]
pub enum ModalState {
    /// No modal is shown.
    #[default]
    None,
    /// Confirmation dialog for a pending action.
    Confirm(PendingAction),
}

/// Result of a command execution.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether the command succeeded.
    pub success: bool,
    /// Output message (stdout or stderr).
    pub message: String,
}

/// Default batch size for loading more entries.
const DEFAULT_BATCH_SIZE: usize = 500;

/// Threshold for triggering load more (entries from end).
const LOAD_MORE_THRESHOLD: usize = 50;

/// Application state.
pub struct App {
    /// Graph log with all lines and commit metadata.
    pub graph_log: GraphLog,
    /// Currently selected commit index (in commit_line_indices).
    pub selected: usize,
    /// Scroll offset for the log view (line-based).
    pub scroll_offset: usize,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Repository root path.
    pub repo_root: String,
    /// Current view mode.
    pub view: View,
    /// Detail view state.
    pub detail_state: Option<DetailState>,
    /// Whether the help modal is shown.
    pub show_help: bool,
    /// jj command runner.
    runner: JjRunner,
    /// Modal dialog state.
    pub modal: ModalState,
    /// Last command result for status display.
    pub last_command_result: Option<CommandResult>,
    /// Current input mode (if any).
    pub input_mode: Option<InputMode>,
    /// Text input buffer.
    pub input: Input,
    /// Log entry limit (None = no limit, i.e., all history).
    log_limit: Option<usize>,
    /// Whether there are more entries to load.
    pub has_more_entries: bool,
    /// Whether we are currently loading more entries.
    pub is_loading_more: bool,
    /// Whether a load-more check has been requested.
    pending_load_more: bool,
}

impl App {
    /// Create a new App with the given graph log.
    pub fn new(graph_log: GraphLog, repo_root: String, runner: JjRunner) -> Self {
        Self {
            graph_log,
            selected: 0,
            scroll_offset: 0,
            should_quit: false,
            repo_root,
            view: View::default(),
            detail_state: None,
            show_help: false,
            runner,
            modal: ModalState::default(),
            last_command_result: None,
            input_mode: None,
            input: Input::default(),
            log_limit: Some(DEFAULT_BATCH_SIZE),
            has_more_entries: false, // Will be set by set_log_limit
            is_loading_more: false,
            pending_load_more: false,
        }
    }

    /// Request application quit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Toggle help modal visibility.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Close help modal.
    pub fn close_help(&mut self) {
        self.show_help = false;
    }

    /// Check if a modal is currently shown.
    pub fn is_modal_open(&self) -> bool {
        !matches!(self.modal, ModalState::None)
    }

    /// Close the modal dialog without executing.
    pub fn close_modal(&mut self) {
        self.modal = ModalState::None;
    }

    /// Open detail view for selected entry.
    pub fn open_detail(&mut self) -> Result<(), XorcistError> {
        if let Some(change_id) = self.selected_change_id() {
            let show_output = fetch_show(&self.runner, change_id)?;
            self.detail_state = Some(DetailState {
                show_output,
                scroll: 0,
                content_height: 0, // Calculated during render
            });
            self.view = View::Detail;
        }
        Ok(())
    }

    /// Close detail view and return to log.
    pub fn close_detail(&mut self) {
        self.view = View::Log;
        self.detail_state = None;
    }

    /// Scroll detail view down.
    pub fn detail_scroll_down(&mut self, amount: usize) {
        if let Some(state) = &mut self.detail_state {
            state.scroll = state.scroll.saturating_add(amount);
        }
    }

    /// Scroll detail view up.
    pub fn detail_scroll_up(&mut self, amount: usize) {
        if let Some(state) = &mut self.detail_state {
            state.scroll = state.scroll.saturating_sub(amount);
        }
    }

    /// Set content height for detail view (called from render).
    pub fn set_detail_content_height(&mut self, height: usize) {
        if let Some(state) = &mut self.detail_state {
            state.content_height = height;
            // Clamp scroll to valid range
            if height > 0 && state.scroll >= height {
                state.scroll = height.saturating_sub(1);
            }
        }
    }
}
