//! jj command execution wrapper.

use std::path::Path;
use std::process::{Command, Output};

use crate::app::CommandResult;
use crate::error::XorcistError;

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
    pub fn run_capture(&self, args: &[&str]) -> Result<String, XorcistError> {
        let output = self.execute(args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XorcistError::JjError(stderr.trim().to_string()));
        }

        String::from_utf8(output.stdout).map_err(|_| XorcistError::InvalidUtf8)
    }

    /// Execute a jj command and return the raw output.
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

    /// Check if jj is available.
    pub fn is_available(&self) -> bool {
        Command::new("jj")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase1 command execution methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute `jj new` to create a new change.
    pub fn execute_new(&self, parent: &str) -> Result<CommandResult, XorcistError> {
        let args = ["new", parent];
        self.run_command(&args, "jj new")
    }

    /// Execute `jj new -m` to create a new change with a message.
    pub fn execute_new_with_message(
        &self,
        parent: &str,
        message: &str,
    ) -> Result<CommandResult, XorcistError> {
        let args = ["new", parent, "-m", message];
        self.run_command(&args, "jj new -m")
    }

    /// Execute `jj edit` to edit a revision.
    pub fn execute_edit(&self, revision: &str) -> Result<CommandResult, XorcistError> {
        let args = ["edit", revision];
        self.run_command(&args, "jj edit")
    }

    /// Execute `jj describe -m` to set a commit message.
    pub fn execute_describe(
        &self,
        revision: &str,
        message: &str,
    ) -> Result<CommandResult, XorcistError> {
        let args = ["describe", revision, "-m", message];
        self.run_command(&args, "jj describe")
    }

    /// Execute `jj bookmark set` to set a bookmark.
    pub fn execute_bookmark_set(
        &self,
        name: &str,
        revision: &str,
    ) -> Result<CommandResult, XorcistError> {
        let args = ["bookmark", "set", name, "-r", revision];
        self.run_command(&args, "jj bookmark set")
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase2 command execution methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute `jj abandon` to abandon a change.
    pub fn execute_abandon(&self, revision: &str) -> Result<CommandResult, XorcistError> {
        let args = ["abandon", revision];
        self.run_command(&args, "jj abandon")
    }

    /// Execute `jj squash` to squash a change into its parent.
    pub fn execute_squash(&self, revision: &str) -> Result<CommandResult, XorcistError> {
        let args = ["squash", "-r", revision];
        self.run_command(&args, "jj squash")
    }

    /// Execute `jj git fetch` to fetch from remote.
    pub fn execute_git_fetch(&self) -> Result<CommandResult, XorcistError> {
        let args = ["git", "fetch"];
        self.run_command(&args, "jj git fetch")
    }

    /// Execute `jj git push` to push to remote.
    pub fn execute_git_push(&self) -> Result<CommandResult, XorcistError> {
        let args = ["git", "push"];
        self.run_command(&args, "jj git push")
    }

    /// Execute `jj undo` to undo the last operation.
    pub fn execute_undo(&self) -> Result<CommandResult, XorcistError> {
        let args = ["undo"];
        self.run_command(&args, "jj undo")
    }

    /// Run a jj command and return a CommandResult.
    fn run_command(&self, args: &[&str], cmd_name: &str) -> Result<CommandResult, XorcistError> {
        let output = self.execute(args)?;
        let success = output.status.success();
        let message = if success {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        };

        Ok(CommandResult {
            command: cmd_name.to_string(),
            success,
            message,
        })
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
}
