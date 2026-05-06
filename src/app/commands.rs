//! jj command execution methods for App.

use std::sync::mpsc;
use std::thread;

use crate::error::XorcistError;
use crate::jj::{JjBackend, fetch_diff_file, fetch_graph_log, parse_diff_summary};

use super::{App, CommandResult, DiffState, JobKind, JobResult, ModalState, PendingAction, View};

impl App {
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

    fn start_command_and_refresh<F>(&mut self, kind: JobKind, command: F)
    where
        F: FnOnce(&dyn JjBackend) -> Result<CommandResult, XorcistError> + Send + 'static,
    {
        let runner = self.runner();
        let limit = self.log_limit;
        let (tx, rx) = mpsc::channel();
        if !self.start_job(kind, rx) {
            return;
        }
        thread::spawn(move || {
            let command_result = command(&*runner);
            let graph_result = fetch_graph_log(&*runner, limit);
            let _ = tx.send(JobResult::CommandAndRefresh {
                command: command_result,
                graph_log: graph_result,
            });
        });
    }

    pub(crate) fn apply_job_result(&mut self, result: JobResult) -> Result<(), XorcistError> {
        match result {
            JobResult::LoadMore {
                previous_selection,
                requested_limit,
                result,
            } => {
                let graph_log = result?;
                let loaded_count = graph_log.commit_count();
                self.graph_log = graph_log;
                self.log_limit = Some(requested_limit);
                self.has_more_entries = loaded_count >= requested_limit;
                self.restore_or_clamp_selection(previous_selection.as_deref());
            }
            JobResult::CommandAndRefresh { command, graph_log } => {
                self.handle_command_result(command);
                self.graph_log = graph_log?;
                self.restore_or_clamp_selection(None);
            }
            JobResult::Detail(result) => {
                let show_output = result?;
                self.detail_state = Some(super::DetailState {
                    show_output,
                    scroll: 0,
                    content_height: 0,
                });
                self.view = View::Detail;
            }
            JobResult::Diff {
                change_id,
                files,
                initial_diff,
            } => {
                let files = files?;
                self.diff_state = DiffState::new(change_id, files);
                if let Some(result) = initial_diff {
                    self.diff_state.diff_lines = result?.lines().map(|s| s.to_string()).collect();
                }
                self.view = View::Diff;
            }
            JobResult::DiffText(result) => {
                self.diff_state.diff_lines = result?.lines().map(|s| s.to_string()).collect();
                self.diff_state.diff_scroll = 0;
                self.diff_state.diff_h_scroll = 0;
            }
        }
        Ok(())
    }

    pub(crate) fn restore_or_clamp_selection(&mut self, change_id: Option<&str>) {
        if let Some(change_id) = change_id
            && let Some(selection) = self.graph_log.selection_for_change_id(change_id)
        {
            self.selected = selection;
            return;
        }

        let count = self.commit_count();
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
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

    /// Get the parsed description of the selected commit.
    fn selected_description(&self) -> Option<String> {
        let line_idx = self.selected_line_index()?;
        self.graph_log.lines[line_idx].description.clone()
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
                self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
                    runner.execute_abandon(&change_id)
                });
            }
            PendingAction::Squash { change_id, .. } => {
                self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
                    runner.execute_squash(&change_id)
                });
            }
            PendingAction::GitPush => {
                self.start_command_and_refresh(JobKind::GitPush, |runner| {
                    runner.execute_git_push()
                });
            }
            PendingAction::Undo => {
                self.start_command_and_refresh(JobKind::MutatingCommand, |runner| {
                    runner.execute_undo()
                });
            }
        }

        Ok(())
    }

    /// Execute `jj git fetch`.
    pub fn execute_git_fetch(&mut self) -> Result<(), XorcistError> {
        self.start_command_and_refresh(JobKind::GitFetch, |runner| runner.execute_git_fetch());
        Ok(())
    }

    /// Execute `jj new` on the selected revision.
    pub fn execute_new(&mut self) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
            runner.execute_new(&change_id)
        });
        Ok(())
    }

    /// Execute `jj new -m` with the given message.
    pub fn execute_new_with_message(&mut self, message: &str) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let message = message.to_string();
        self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
            if message.is_empty() {
                runner.execute_new(&change_id)
            } else {
                runner.execute_new_with_message(&change_id, &message)
            }
        });
        Ok(())
    }

    /// Execute `jj edit` on the selected revision.
    pub fn execute_edit(&mut self) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
            runner.execute_edit(&change_id)
        });
        Ok(())
    }

    /// Execute `jj describe -m` on the selected revision.
    pub fn execute_describe(&mut self, message: &str) -> Result<(), XorcistError> {
        let Some(change_id) = self.selected_change_id() else {
            return Ok(());
        };
        let change_id = change_id.to_string();
        let message = message.to_string();
        self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
            runner.execute_describe(&change_id, &message)
        });
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
        let name = name.to_string();
        self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
            runner.execute_bookmark_set(&name, &change_id)
        });
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
        let destination = destination.to_string();
        self.start_command_and_refresh(JobKind::MutatingCommand, move |runner| {
            runner.execute_rebase(&change_id, &destination)
        });
        Ok(())
    }

    /// Open diff view for the current detail state.
    pub fn open_diff_view(&mut self) -> Result<(), XorcistError> {
        let Some(detail) = &self.detail_state else {
            return Ok(());
        };
        let change_id = detail.show_output.change_id.clone();
        let runner = self.runner();
        let (tx, rx) = mpsc::channel();
        if !self.start_job(JobKind::OpenDiff, rx) {
            return Ok(());
        }
        thread::spawn(move || {
            let files_result = runner
                .run_capture(&["diff", "-r", &change_id, "--color=never", "--summary"])
                .map(|output| parse_diff_summary(&output));
            let initial_diff = files_result.as_ref().ok().and_then(|files| {
                files
                    .first()
                    .map(|file| fetch_diff_file(&*runner, &change_id, &file.path))
            });
            let _ = tx.send(JobResult::Diff {
                change_id,
                files: files_result,
                initial_diff,
            });
        });
        Ok(())
    }

    /// Refresh diff text for the currently selected file.
    pub fn refresh_diff_text(&mut self) -> Result<(), XorcistError> {
        let Some(file) = self.diff_state.selected_file() else {
            self.diff_state.diff_lines = Vec::new();
            return Ok(());
        };
        let path = file.path.clone();
        let change_id = self.diff_state.change_id.clone();
        let runner = self.runner();
        let (tx, rx) = mpsc::channel();
        if !self.start_job(JobKind::RefreshDiff, rx) {
            return Ok(());
        }
        thread::spawn(move || {
            let _ = tx.send(JobResult::DiffText(fetch_diff_file(
                &*runner, &change_id, &path,
            )));
        });
        Ok(())
    }
}
