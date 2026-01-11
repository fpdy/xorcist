//! jj command execution wrapper.

use std::path::Path;
use std::process::{Command, Output};

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
