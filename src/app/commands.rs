//! jj command execution methods for App.

use crate::error::XorcistError;
use crate::jj::fetch_graph_log;

use super::{App, CommandResult, ModalState, PendingAction};

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
}
