//! jj command execution wrapper.

use std::fmt::Debug;
use std::path::Path;
use std::process::{Command, Output};

use crate::app::CommandResult;
use crate::error::XorcistError;

/// Backend interface for jj operations.
///
/// App code depends on this trait so tests can inject fakes without invoking a
/// real `jj` binary.
pub trait JjBackend: Debug + Send + Sync {
    /// Run a jj command and capture stdout as UTF-8.
    fn run_capture(&self, args: &[&str]) -> Result<String, XorcistError>;

    /// Check if jj is available.
    fn is_available(&self) -> bool;

    /// Execute `jj new` to create a new change.
    fn execute_new(&self, parent: &str) -> Result<CommandResult, XorcistError>;

    /// Execute `jj new -m` to create a new change with a message.
    fn execute_new_with_message(
        &self,
        parent: &str,
        message: &str,
    ) -> Result<CommandResult, XorcistError>;

    /// Execute `jj edit` to edit a revision.
    fn execute_edit(&self, revision: &str) -> Result<CommandResult, XorcistError>;

    /// Execute `jj describe -m` to set a commit message.
    fn execute_describe(
        &self,
        revision: &str,
        message: &str,
    ) -> Result<CommandResult, XorcistError>;

    /// Execute `jj bookmark set` to set a bookmark.
    fn execute_bookmark_set(
        &self,
        name: &str,
        revision: &str,
    ) -> Result<CommandResult, XorcistError>;

    /// Execute `jj abandon` to abandon a change.
    fn execute_abandon(&self, revision: &str) -> Result<CommandResult, XorcistError>;

    /// Execute `jj squash` to squash a change into its parent.
    fn execute_squash(&self, revision: &str) -> Result<CommandResult, XorcistError>;

    /// Execute `jj git fetch` to fetch from remote.
    fn execute_git_fetch(&self) -> Result<CommandResult, XorcistError>;

    /// Execute `jj git push` to push to remote.
    fn execute_git_push(&self) -> Result<CommandResult, XorcistError>;

    /// Execute `jj undo` to undo the last operation.
    fn execute_undo(&self) -> Result<CommandResult, XorcistError>;

    /// Execute `jj rebase -r <revision> -d <destination>`.
    fn execute_rebase(
        &self,
        revision: &str,
        destination: &str,
    ) -> Result<CommandResult, XorcistError>;
}

/// Runner for executing jj commands.
#[derive(Debug, Clone)]
pub struct JjRunner {
    /// Working directory for jj commands.
    work_dir: Option<std::path::PathBuf>,
}

impl JjRunner {
    /// Create a new JjRunner.
    pub fn new() -> Self {
        Self { work_dir: None }
    }

    /// Set the working directory for commands.
    pub fn with_work_dir(mut self, dir: &Path) -> Self {
        self.work_dir = Some(dir.to_path_buf());
        self
    }

    /// Run a jj command and capture its output.
    fn execute(&self, args: &[&str]) -> Result<Output, XorcistError> {
        let mut cmd = Command::new("jj");
        cmd.args(args);

        if let Some(dir) = &self.work_dir {
            cmd.current_dir(dir);
        }

        cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                XorcistError::JjNotFound
            } else {
                XorcistError::Io(e)
            }
        })
    }

    /// Run a jj command and return a CommandResult.
    fn run_command(&self, args: &[&str]) -> Result<CommandResult, XorcistError> {
        let output = self.execute(args)?;
        let success = output.status.success();
        let message = command_message(success, &output.stdout, &output.stderr);

        Ok(CommandResult { success, message })
    }
}

impl JjBackend for JjRunner {
    fn run_capture(&self, args: &[&str]) -> Result<String, XorcistError> {
        let output = self.execute(args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XorcistError::JjError(stderr.trim().to_string()));
        }

        String::from_utf8(output.stdout).map_err(|_| XorcistError::InvalidUtf8)
    }

    fn is_available(&self) -> bool {
        Command::new("jj")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Execute `jj new` to create a new change.
    fn execute_new(&self, parent: &str) -> Result<CommandResult, XorcistError> {
        self.run_command(&["new", parent])
    }

    /// Execute `jj new -m` to create a new change with a message.
    fn execute_new_with_message(
        &self,
        parent: &str,
        message: &str,
    ) -> Result<CommandResult, XorcistError> {
        self.run_command(&["new", parent, "-m", message])
    }

    /// Execute `jj edit` to edit a revision.
    fn execute_edit(&self, revision: &str) -> Result<CommandResult, XorcistError> {
        self.run_command(&["edit", revision])
    }

    /// Execute `jj describe -m` to set a commit message.
    fn execute_describe(
        &self,
        revision: &str,
        message: &str,
    ) -> Result<CommandResult, XorcistError> {
        self.run_command(&["describe", revision, "-m", message])
    }

    /// Execute `jj bookmark set` to set a bookmark.
    fn execute_bookmark_set(
        &self,
        name: &str,
        revision: &str,
    ) -> Result<CommandResult, XorcistError> {
        self.run_command(&["bookmark", "set", name, "-r", revision])
    }

    /// Execute `jj abandon` to abandon a change.
    fn execute_abandon(&self, revision: &str) -> Result<CommandResult, XorcistError> {
        self.run_command(&["abandon", revision])
    }

    /// Execute `jj squash` to squash a change into its parent.
    fn execute_squash(&self, revision: &str) -> Result<CommandResult, XorcistError> {
        self.run_command(&["squash", "-r", revision])
    }

    /// Execute `jj git fetch` to fetch from remote.
    fn execute_git_fetch(&self) -> Result<CommandResult, XorcistError> {
        self.run_command(&["git", "fetch"])
    }

    /// Execute `jj git push` to push to remote.
    fn execute_git_push(&self) -> Result<CommandResult, XorcistError> {
        self.run_command(&["git", "push"])
    }

    /// Execute `jj undo` to undo the last operation.
    fn execute_undo(&self) -> Result<CommandResult, XorcistError> {
        self.run_command(&["undo"])
    }

    /// Execute `jj rebase -r <revision> -d <destination>`.
    fn execute_rebase(
        &self,
        revision: &str,
        destination: &str,
    ) -> Result<CommandResult, XorcistError> {
        self.run_command(&["rebase", "-r", revision, "-d", destination])
    }
}

fn command_message(success: bool, stdout: &[u8], stderr: &[u8]) -> String {
    if success {
        let stdout = String::from_utf8_lossy(stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(stderr).trim().to_string();
        match (stdout.is_empty(), stderr.is_empty()) {
            (true, true) => String::new(),
            (false, true) => stdout,
            (true, false) => stderr,
            (false, false) => format!("{stdout}\n{stderr}"),
        }
    } else {
        String::from_utf8_lossy(stderr).trim().to_string()
    }
}

impl Default for JjRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let runner = JjRunner::new();
        assert!(runner.work_dir.is_none());
    }

    #[test]
    fn test_runner_with_work_dir() {
        let runner = JjRunner::new().with_work_dir(Path::new("/tmp"));
        assert_eq!(runner.work_dir, Some(std::path::PathBuf::from("/tmp")));
    }

    #[test]
    fn test_successful_command_message_preserves_stderr() {
        assert_eq!(
            command_message(true, b"", b"Created new commit\n"),
            "Created new commit"
        );
        assert_eq!(
            command_message(true, b"stdout\n", b"stderr\n"),
            "stdout\nstderr"
        );
    }
}
