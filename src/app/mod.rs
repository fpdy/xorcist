//! Application state management.

mod commands;
mod input;
mod loading;
mod navigation;

#[cfg(test)]
mod tests;

use std::sync::{Arc, mpsc};

use tui_input::Input;

use crate::error::XorcistError;
use crate::jj::{DiffEntry, GraphLog, JjBackend, ShowOutput};
use crate::text::truncate_str;

/// Current view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Log,
    Detail,
    Diff,
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
    /// Entering destination for `jj rebase -d`.
    RebaseDestination,
}

impl InputMode {
    /// Get the placeholder text for this input mode.
    pub fn placeholder(&self) -> &'static str {
        match self {
            InputMode::Describe => "Enter commit message...",
            InputMode::BookmarkSet => "Enter bookmark name...",
            InputMode::NewWithMessage => "Enter message (empty for no message)...",
            InputMode::RebaseDestination => "Enter destination (e.g., @-, main, abc123)...",
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

/// State for diff view.
#[derive(Debug, Clone, Default)]
pub struct DiffState {
    /// Target change ID.
    pub change_id: String,
    /// List of changed files.
    pub files: Vec<crate::jj::DiffEntry>,
    /// Currently selected file index.
    pub selected: usize,
    /// Scroll offset for file list.
    pub file_scroll: usize,
    /// Diff text lines for selected file.
    pub diff_lines: Vec<String>,
    /// Vertical scroll offset for diff text.
    pub diff_scroll: usize,
    /// Horizontal scroll offset for diff text.
    pub diff_h_scroll: usize,
}

impl DiffState {
    /// Create a new DiffState from change_id and files.
    pub fn new(change_id: String, files: Vec<crate::jj::DiffEntry>) -> Self {
        Self {
            change_id,
            files,
            selected: 0,
            file_scroll: 0,
            diff_lines: Vec::new(),
            diff_scroll: 0,
            diff_h_scroll: 0,
        }
    }

    /// Get the currently selected file, if any.
    pub fn selected_file(&self) -> Option<&crate::jj::DiffEntry> {
        self.files.get(self.selected)
    }
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

/// Background jj job currently owned by the App.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobKind {
    LoadMore,
    GitFetch,
    GitPush,
    MutatingCommand,
    OpenDetail,
    OpenDiff,
    RefreshDiff,
}

/// Result returned by background jj jobs.
pub enum JobResult {
    LoadMore {
        previous_selection: Option<String>,
        requested_limit: usize,
        result: Result<GraphLog, XorcistError>,
    },
    CommandAndRefresh {
        command: Result<CommandResult, XorcistError>,
        graph_log: Result<GraphLog, XorcistError>,
    },
    Detail(Result<ShowOutput, XorcistError>),
    Diff {
        change_id: String,
        files: Result<Vec<DiffEntry>, XorcistError>,
        initial_diff: Option<Result<String, XorcistError>>,
    },
    DiffText(Result<String, XorcistError>),
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
    /// Diff view state.
    pub diff_state: DiffState,
    /// Whether the help modal is shown.
    pub show_help: bool,
    /// jj command runner.
    runner: Arc<dyn JjBackend>,
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
    /// Current background jj job, if any.
    pub current_job: Option<JobKind>,
    /// Completion receiver for the current background job.
    job_rx: Option<mpsc::Receiver<JobResult>>,
}

impl App {
    /// Create a new App with the given graph log.
    pub fn new(graph_log: GraphLog, repo_root: String, runner: Arc<dyn JjBackend>) -> Self {
        Self {
            graph_log,
            selected: 0,
            scroll_offset: 0,
            should_quit: false,
            repo_root,
            view: View::default(),
            detail_state: None,
            diff_state: DiffState::default(),
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
            current_job: None,
            job_rx: None,
        }
    }

    /// Clone the configured jj backend for a background job.
    pub(crate) fn runner(&self) -> Arc<dyn JjBackend> {
        Arc::clone(&self.runner)
    }

    /// Whether a background jj job is running.
    pub fn is_busy(&self) -> bool {
        self.current_job.is_some()
    }

    pub(crate) fn start_job(&mut self, kind: JobKind, rx: mpsc::Receiver<JobResult>) -> bool {
        if self.is_busy() {
            self.last_command_result = Some(CommandResult {
                success: false,
                message: "Another jj command is still running".to_string(),
            });
            return false;
        }
        self.current_job = Some(kind);
        self.job_rx = Some(rx);
        true
    }

    pub(crate) fn clear_job(&mut self) {
        self.current_job = None;
        self.job_rx = None;
        self.is_loading_more = false;
    }

    /// Poll a background job once and apply its result if complete.
    pub fn poll_background_job(&mut self) -> Result<(), XorcistError> {
        let Some(rx) = self.job_rx.as_ref() else {
            return Ok(());
        };

        match rx.try_recv() {
            Ok(result) => {
                self.apply_job_result(result)?;
                self.clear_job();
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.last_command_result = Some(CommandResult {
                    success: false,
                    message: "Background jj command ended unexpectedly".to_string(),
                });
                self.clear_job();
            }
        }
        Ok(())
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
            let change_id = change_id.to_string();
            let runner = self.runner();
            let (tx, rx) = mpsc::channel();
            if !self.start_job(JobKind::OpenDetail, rx) {
                return Ok(());
            }
            std::thread::spawn(move || {
                let _ = tx.send(JobResult::Detail(crate::jj::fetch_show(
                    &*runner, &change_id,
                )));
            });
        }
        Ok(())
    }

    /// Close detail view and return to log.
    pub fn close_detail(&mut self) {
        self.view = View::Log;
        self.detail_state = None;
    }

    /// Close diff view and return to detail.
    pub fn close_diff(&mut self) {
        self.view = View::Detail;
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
