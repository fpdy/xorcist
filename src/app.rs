//! Application state management.

use tui_input::Input;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::error::XorcistError;
use crate::jj::{JjRunner, LogEntry, ShowOutput, fetch_log, fetch_show};

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

/// Truncate a string to fit within a maximum display width.
/// Uses unicode-width for correct handling of CJK and other wide characters.
fn truncate_str(s: &str, max_width: usize) -> String {
    let width = s.width();
    if width <= max_width {
        return s.to_string();
    }

    let target_width = max_width.saturating_sub(3); // Reserve space for "..."
    let mut current_width = 0;
    let mut end_idx = 0;

    for (idx, ch) in s.char_indices() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        current_width += ch_width;
        end_idx = idx + ch.len_utf8();
    }

    format!("{}...", &s[..end_idx])
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
    /// The command that was executed.
    pub command: String,
    /// Whether the command succeeded.
    pub success: bool,
    /// Output message (stdout or stderr).
    pub message: String,
}

/// Application state.
pub struct App {
    /// Log entries to display.
    pub entries: Vec<LogEntry>,
    /// Currently selected index.
    pub selected: usize,
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
}

impl App {
    /// Create a new App with the given log entries.
    pub fn new(entries: Vec<LogEntry>, repo_root: String, runner: JjRunner) -> Self {
        Self {
            entries,
            selected: 0,
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
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Jump to the first entry.
    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Jump to the last entry.
    pub fn select_last(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Page down (move by visible height).
    pub fn page_down(&mut self, page_size: usize) {
        if self.entries.is_empty() {
            return;
        }
        let new_selected = self.selected.saturating_add(page_size);
        self.selected = new_selected.min(self.entries.len() - 1);
    }

    /// Page up (move by visible height).
    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
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

    /// Open detail view for selected entry.
    pub fn open_detail(&mut self) -> Result<(), XorcistError> {
        if let Some(entry) = self.entries.get(self.selected) {
            let show_output = fetch_show(&self.runner, &entry.change_id)?;
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

    /// Get the currently selected entry.
    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.entries.get(self.selected)
    }

    /// Refresh log entries.
    pub fn refresh_log(&mut self) -> Result<(), XorcistError> {
        self.entries = fetch_log(&self.runner, Some(500))?;
        // Clamp selection to valid range
        if !self.entries.is_empty() && self.selected >= self.entries.len() {
            self.selected = self.entries.len() - 1;
        }
        Ok(())
    }

    /// Handle command result (store for status display).
    fn handle_command_result(&mut self, result: Result<CommandResult, XorcistError>) {
        match result {
            Ok(cmd_result) => {
                self.last_command_result = Some(cmd_result);
            }
            Err(e) => {
                self.last_command_result = Some(CommandResult {
                    command: "unknown".to_string(),
                    success: false,
                    message: e.to_string(),
                });
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Modal dialog management
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if a modal is currently shown.
    pub fn is_modal_open(&self) -> bool {
        !matches!(self.modal, ModalState::None)
    }

    /// Close the modal dialog without executing.
    pub fn close_modal(&mut self) {
        self.modal = ModalState::None;
    }

    /// Show confirmation dialog for abandon.
    pub fn show_abandon_confirm(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.modal = ModalState::Confirm(PendingAction::Abandon {
                change_id: entry.change_id.clone(),
                description: entry.description.clone(),
            });
        }
    }

    /// Show confirmation dialog for squash.
    pub fn show_squash_confirm(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.modal = ModalState::Confirm(PendingAction::Squash {
                change_id: entry.change_id.clone(),
                description: entry.description.clone(),
            });
        }
    }

    /// Show confirmation dialog for git push.
    pub fn show_push_confirm(&mut self) {
        self.modal = ModalState::Confirm(PendingAction::GitPush);
    }

    /// Show confirmation dialog for undo.
    pub fn show_undo_confirm(&mut self) {
        self.modal = ModalState::Confirm(PendingAction::Undo);
    }

    /// Confirm and execute the pending action.
    pub fn confirm_action(&mut self) -> Result<(), XorcistError> {
        let action = match std::mem::take(&mut self.modal) {
            ModalState::Confirm(action) => action,
            ModalState::None => return Ok(()),
        };

        match action {
            PendingAction::Abandon { change_id, .. } => {
                let result = self.runner.execute_abandon(&change_id);
                self.handle_command_result(result);
                self.refresh_log()?;
            }
            PendingAction::Squash { change_id, .. } => {
                let result = self.runner.execute_squash(&change_id);
                self.handle_command_result(result);
                self.refresh_log()?;
            }
            PendingAction::GitPush => {
                let result = self.runner.execute_git_push();
                self.handle_command_result(result);
                self.refresh_log()?;
            }
            PendingAction::Undo => {
                let result = self.runner.execute_undo();
                self.handle_command_result(result);
                self.refresh_log()?;
            }
        }

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Direct command execution (no confirmation)
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute `jj git fetch`.
    pub fn execute_git_fetch(&mut self) -> Result<(), XorcistError> {
        let result = self.runner.execute_git_fetch();
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Input mode management
    // ─────────────────────────────────────────────────────────────────────────

    /// Start input mode for text entry.
    pub fn start_input_mode(&mut self, mode: InputMode) {
        self.input_mode = Some(mode);
        self.input.reset();
    }

    /// Cancel input mode without executing.
    pub fn cancel_input_mode(&mut self) {
        self.input_mode = None;
        self.input.reset();
    }

    /// Check if currently in input mode.
    pub fn is_input_mode(&self) -> bool {
        self.input_mode.is_some()
    }

    /// Submit the current input and execute the corresponding command.
    pub fn submit_input(&mut self) -> Result<(), XorcistError> {
        let Some(mode) = self.input_mode.take() else {
            return Ok(());
        };
        let value = self.input.value().to_string();
        self.input.reset();

        match mode {
            InputMode::Describe => self.execute_describe(&value)?,
            InputMode::BookmarkSet => self.execute_bookmark_set(&value)?,
            InputMode::NewWithMessage => self.execute_new_with_message(&value)?,
        }
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Command execution (Phase1 commands)
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute `jj new` on the selected revision.
    pub fn execute_new(&mut self) -> Result<(), XorcistError> {
        let Some(entry) = self.selected_entry() else {
            return Ok(());
        };
        let change_id = entry.change_id.clone();
        let result = self.runner.execute_new(&change_id);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj new -m` with the given message.
    pub fn execute_new_with_message(&mut self, message: &str) -> Result<(), XorcistError> {
        let Some(entry) = self.selected_entry() else {
            return Ok(());
        };
        let change_id = entry.change_id.clone();
        let result = if message.is_empty() {
            self.runner.execute_new(&change_id)
        } else {
            self.runner.execute_new_with_message(&change_id, message)
        };
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj edit` on the selected revision.
    pub fn execute_edit(&mut self) -> Result<(), XorcistError> {
        let Some(entry) = self.selected_entry() else {
            return Ok(());
        };
        let change_id = entry.change_id.clone();
        let result = self.runner.execute_edit(&change_id);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj describe -m` on the selected revision.
    pub fn execute_describe(&mut self, message: &str) -> Result<(), XorcistError> {
        let Some(entry) = self.selected_entry() else {
            return Ok(());
        };
        let change_id = entry.change_id.clone();
        let result = self.runner.execute_describe(&change_id, message);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj bookmark set` on the selected revision.
    pub fn execute_bookmark_set(&mut self, name: &str) -> Result<(), XorcistError> {
        if name.is_empty() {
            self.last_command_result = Some(CommandResult {
                command: "jj bookmark set".to_string(),
                success: false,
                message: "Bookmark name cannot be empty".to_string(),
            });
            return Ok(());
        }
        let Some(entry) = self.selected_entry() else {
            return Ok(());
        };
        let change_id = entry.change_id.clone();
        let result = self.runner.execute_bookmark_set(name, &change_id);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn make_entry(id: &str) -> LogEntry {
        LogEntry {
            change_id: id.to_string(),
            change_id_prefix: id.to_string(),
            change_id_rest: String::new(),
            commit_id: format!("commit_{id}"),
            commit_id_prefix: format!("commit_{id}"),
            commit_id_rest: String::new(),
            author: "Test".to_string(),
            timestamp: "now".to_string(),
            description: format!("Entry {id}"),
            is_working_copy: false,
            is_immutable: false,
            is_empty: false,
            bookmarks: vec![],
        }
    }

    fn make_runner() -> JjRunner {
        JjRunner::new().with_work_dir(Path::new("/tmp"))
    }

    #[test]
    fn test_navigation() {
        let entries = vec![make_entry("1"), make_entry("2"), make_entry("3")];
        let mut app = App::new(entries, "/repo".to_string(), make_runner());

        assert_eq!(app.selected, 0);

        app.select_next();
        assert_eq!(app.selected, 1);

        app.select_next();
        assert_eq!(app.selected, 2);

        // Should not go past the end
        app.select_next();
        assert_eq!(app.selected, 2);

        app.select_previous();
        assert_eq!(app.selected, 1);

        app.select_first();
        assert_eq!(app.selected, 0);

        app.select_last();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_page_navigation() {
        let entries: Vec<_> = (0..20).map(|i| make_entry(&i.to_string())).collect();
        let mut app = App::new(entries, "/repo".to_string(), make_runner());

        app.page_down(5);
        assert_eq!(app.selected, 5);

        app.page_down(5);
        assert_eq!(app.selected, 10);

        app.page_up(3);
        assert_eq!(app.selected, 7);

        // Page down past the end
        app.page_down(100);
        assert_eq!(app.selected, 19);

        // Page up past the beginning
        app.page_up(100);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_empty_entries() {
        let mut app = App::new(vec![], "/repo".to_string(), make_runner());

        // Should not panic on empty list
        app.select_next();
        app.select_previous();
        app.select_first();
        app.select_last();
        app.page_down(5);
        app.page_up(5);

        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_view_transitions() {
        let entries = vec![make_entry("1")];
        let mut app = App::new(entries, "/repo".to_string(), make_runner());

        assert_eq!(app.view, View::Log);
        assert!(app.detail_state.is_none());

        // Note: open_detail would fail without a real jj repo
        // We just test close_detail here
        app.view = View::Detail;
        app.detail_state = Some(DetailState {
            show_output: ShowOutput {
                change_id: "abc123".to_string(),
                change_id_prefix: "abc".to_string(),
                change_id_rest: "123".to_string(),
                commit_id: "def456".to_string(),
                commit_id_prefix: "def".to_string(),
                commit_id_rest: "456".to_string(),
                author: "Test".to_string(),
                timestamp: "now".to_string(),
                description: "Test".to_string(),
                bookmarks: vec![],
                diff_summary: vec![],
            },
            scroll: 5,
            content_height: 20,
        });

        app.close_detail();
        assert_eq!(app.view, View::Log);
        assert!(app.detail_state.is_none());
    }

    #[test]
    fn test_detail_scroll() {
        let mut app = App::new(vec![], "/repo".to_string(), make_runner());
        app.detail_state = Some(DetailState {
            show_output: ShowOutput {
                change_id: "abc123".to_string(),
                change_id_prefix: "abc".to_string(),
                change_id_rest: "123".to_string(),
                commit_id: "def456".to_string(),
                commit_id_prefix: "def".to_string(),
                commit_id_rest: "456".to_string(),
                author: "Test".to_string(),
                timestamp: "now".to_string(),
                description: "Test".to_string(),
                bookmarks: vec![],
                diff_summary: vec![],
            },
            scroll: 5,
            content_height: 20,
        });

        app.detail_scroll_down(3);
        assert_eq!(app.detail_state.as_ref().unwrap().scroll, 8);

        app.detail_scroll_up(2);
        assert_eq!(app.detail_state.as_ref().unwrap().scroll, 6);

        // Scroll up past beginning
        app.detail_scroll_up(100);
        assert_eq!(app.detail_state.as_ref().unwrap().scroll, 0);
    }

    #[test]
    fn test_set_detail_content_height() {
        let mut app = App::new(vec![], "/repo".to_string(), make_runner());
        app.detail_state = Some(DetailState {
            show_output: ShowOutput {
                change_id: "abc123".to_string(),
                change_id_prefix: "abc".to_string(),
                change_id_rest: "123".to_string(),
                commit_id: "def456".to_string(),
                commit_id_prefix: "def".to_string(),
                commit_id_rest: "456".to_string(),
                author: "Test".to_string(),
                timestamp: "now".to_string(),
                description: "Test".to_string(),
                bookmarks: vec![],
                diff_summary: vec![],
            },
            scroll: 50,
            content_height: 0,
        });

        // Setting height should clamp scroll
        app.set_detail_content_height(20);
        assert_eq!(app.detail_state.as_ref().unwrap().content_height, 20);
        assert_eq!(app.detail_state.as_ref().unwrap().scroll, 19);
    }

    #[test]
    fn test_truncate_str_ascii() {
        // ASCII strings: 1 char = 1 width
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("exact len", 9), "exact len");
    }

    #[test]
    fn test_truncate_str_japanese() {
        // Japanese characters: 1 char = 2 width
        assert_eq!(truncate_str("日本語", 10), "日本語"); // 6 width, fits
        assert_eq!(truncate_str("日本語テスト", 10), "日本語..."); // 12 width -> truncate to 7 + "..."
    }

    #[test]
    fn test_truncate_str_mixed() {
        // Mixed ASCII and CJK
        assert_eq!(truncate_str("Hello世界", 10), "Hello世界"); // 5 + 4 = 9 width, fits
        assert_eq!(truncate_str("Hello世界!", 10), "Hello世界!"); // 5 + 4 + 1 = 10 width, fits exactly
        assert_eq!(truncate_str("Hello世界!!", 10), "Hello世..."); // 5 + 4 + 2 = 11 width, truncate
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 10), "");
        assert_eq!(truncate_str("", 0), "");
    }

    #[test]
    fn test_truncate_str_small_max() {
        // When max_width is very small
        assert_eq!(truncate_str("hello", 3), "...");
        assert_eq!(truncate_str("hello", 4), "h...");
    }
}
