//! Custom error types for xorcist.

use thiserror::Error;

/// Errors that can occur in xorcist.
#[derive(Error, Debug)]
pub enum XorcistError {
    /// Not in a jj repository.
    #[error("not in a jj repository (or any parent directory)")]
    NotInRepo,

    /// jj command not found.
    #[error("jj command not found in PATH")]
    JjNotFound,

    /// jj command failed.
    #[error("jj command failed: {0}")]
    JjError(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 decode error.
    #[error("invalid UTF-8 in jj output")]
    InvalidUtf8,
}
