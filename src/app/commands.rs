//! jj command execution methods for App.

use crate::error::XorcistError;
use crate::jj::{fetch_diff_file, fetch_graph_log, parse_diff_summary};

use super::{App, CommandResult, DiffState, ModalState, PendingAction, View};

impl App {
    /// Refresh log entries.
    pub fn refresh_log(&mut self) -> Result<(), XorcistError> {
        self.graph_log = fetch_graph_log(&self.runner, self.log_limit)?;
        // Clamp selection to valid range
        let count = self.commit_count();
        if count > 0 && self.selected >= count {
            self.selected = count - 1;
        }
        Ok(())
    }

    /// Handle command result (store for status display).
    pub(super) fn handle_command_result(&mut self, result: Result<CommandResult, XorcistError>) {
        match result {
            Ok(cmd_result) => {
                self.last_command_result = Some(cmd_result);
            }
            Err(e) => {
                self.last_command_result = Some(CommandResult {
                    success: false,
                    message: e.to_string(),
                });
            }
        }
    }

    /// Show confirmation dialog for abandon.
    pub fn show_abandon_confirm(&mut self) {
        if let Some(change_id) = self.selected_change_id() {
            let description = self.selected_description().unwrap_or_default();
            self.modal = ModalState::Confirm(PendingAction::Abandon {
                change_id: change_id.to_string(),
                description,
            });
        }
    }

    /// Show confirmation dialog for squash.
    pub fn show_squash_confirm(&mut self) {
        if let Some(change_id) = self.selected_change_id() {
            let description = self.selected_description().unwrap_or_default();
            self.modal = ModalState::Confirm(PendingAction::Squash {
                change_id: change_id.to_string(),
                description,
            });
        }
    }

    /// Get the description of the selected commit (parsed from plain text).
    fn selected_description(&self) -> Option<String> {
        let line_idx = self.selected_line_index()?;
        let line = &self.graph_log.lines[line_idx];
        // The description is the last part of the line after change_id, author, timestamp
        // Format: "change_id author timestamp description..."
        // We'll extract everything after the third space-separated token
        let parts: Vec<&str> = line.plain.split_whitespace().collect();
        if parts.len() > 3 {
            Some(parts[3..].join(" "))
        } else {
            None
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

    /// Execute `jj git fetch`.
    pub fn execute_git_fetch(&mut self) -> Result<(), XorcistError> {
        let result = self.runner.execute_git_fetch();
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj new` on the selected revision.
    pub fn execute_new(&mut self) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let result = self.runner.execute_new(&change_id);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj new -m` with the given message.
    pub fn execute_new_with_message(&mut self, message: &str) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
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
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let result = self.runner.execute_edit(&change_id);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj describe -m` on the selected revision.
    pub fn execute_describe(&mut self, message: &str) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let result = self.runner.execute_describe(&change_id, message);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj bookmark set` on the selected revision.
    pub fn execute_bookmark_set(&mut self, name: &str) -> Result<(), XorcistError> {
        if name.is_empty() {
            self.last_command_result = Some(CommandResult {
                success: false,
                message: "Bookmark name cannot be empty".to_string(),
            });
            return Ok(());
        }
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let result = self.runner.execute_bookmark_set(name, &change_id);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Execute `jj rebase -d` on the selected revision.
    pub fn execute_rebase(&mut self, destination: &str) -> Result<(), XorcistError> {
        let destination = destination.trim();
        if destination.is_empty() {
            self.last_command_result = Some(CommandResult {
                success: false,
                message: "Destination cannot be empty".to_string(),
            });
            return Ok(());
        }
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let result = self.runner.execute_rebase(&change_id, destination);
        self.handle_command_result(result);
        self.refresh_log()?;
        Ok(())
    }

    /// Open diff view for the current detail state.
    pub fn open_diff_view(&mut self) -> Result<(), XorcistError> {
        let Some(detail) = &self.detail_state else {
            return Ok(());
        };
        let change_id = detail.show_output.change_id.clone();

        // Fetch diff summary
        let summary_output =
            self.runner
                .run_capture(&["diff", "-r", &change_id, "--color=never", "--summary"])?;
        let files = parse_diff_summary(&summary_output);

        self.diff_state = DiffState::new(change_id, files);

        // Fetch initial diff text if files exist
        if !self.diff_state.files.is_empty() {
            self.refresh_diff_text()?;
        }

        self.view = View::Diff;
        Ok(())
    }

    /// Refresh diff text for the currently selected file.
    pub fn refresh_diff_text(&mut self) -> Result<(), XorcistError> {
        let Some(file) = self.diff_state.selected_file() else {
            self.diff_state.diff_lines = Vec::new();
            return Ok(());
        };
        let path = file.path.clone();
        let output = fetch_diff_file(&self.runner, &self.diff_state.change_id, &path)?;
        self.diff_state.diff_lines = output.lines().map(|s| s.to_string()).collect();
        self.diff_state.diff_scroll = 0; // Reset vertical scroll on file change
        self.diff_state.diff_h_scroll = 0; // Reset horizontal scroll on file change
        Ok(())
    }
}
