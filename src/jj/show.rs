//! jj show command execution.

use crate::error::XorcistError;
use crate::jj::runner::JjRunner;

/// Output from jj show command.
#[derive(Debug, Clone)]
pub struct ShowOutput {
    /// Change ID (full).
    pub change_id: String,
    /// Commit ID (full).
    pub commit_id: String,
    /// Author information.
    pub author: String,
    /// Committer timestamp.
    pub timestamp: String,
    /// Full description.
    pub description: String,
    /// Bookmarks.
    pub bookmarks: Vec<String>,
    /// Diff summary (list of changed files with status).
    pub diff_summary: Vec<DiffEntry>,
}

/// A single file change entry.
#[derive(Debug, Clone)]
pub struct DiffEntry {
    /// Change type: Added, Modified, Deleted, Renamed, etc.
    pub status: DiffStatus,
    /// File path.
    pub path: String,
}

/// Status of a file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

/// Template for machine-readable show output.
/// Fields are separated by \x00 (null byte) for reliable parsing.
const SHOW_TEMPLATE: &str = r#"change_id ++ "\x00" ++ commit_id ++ "\x00" ++ author.name() ++ "\x00" ++ committer.timestamp().ago() ++ "\x00" ++ description ++ "\x00" ++ bookmarks.join(",") ++ "\n""#;

/// Fetch show output for a revision.
pub fn fetch_show(runner: &JjRunner, revision: &str) -> Result<ShowOutput, XorcistError> {
    // 1. Fetch metadata using template
    let meta_output =
        runner.run_capture(&["log", "-r", revision, "--no-graph", "-T", SHOW_TEMPLATE])?;
    let meta = parse_show_meta(&meta_output)?;

    // 2. Fetch diff summary
    let diff_output = runner.run_capture(&["diff", "-r", revision, "--summary"])?;
    let diff_summary = parse_diff_summary(&diff_output);

    Ok(ShowOutput {
        change_id: meta.change_id,
        commit_id: meta.commit_id,
        author: meta.author,
        timestamp: meta.timestamp,
        description: meta.description,
        bookmarks: meta.bookmarks,
        diff_summary,
    })
}

/// Parsed metadata from jj log output.
struct ShowMeta {
    change_id: String,
    commit_id: String,
    author: String,
    timestamp: String,
    description: String,
    bookmarks: Vec<String>,
}

/// Parse metadata from jj log output.
///
/// The output format is: change_id\x00commit_id\x00author\x00timestamp\x00description\x00bookmarks\n
/// Note: description may contain newlines, so we split by \x00 on the entire output
/// rather than processing line by line.
fn parse_show_meta(output: &str) -> Result<ShowMeta, XorcistError> {
    // Remove trailing newline if present, then split by null byte
    let output = output.trim_end_matches('\n');
    let parts: Vec<&str> = output.split('\x00').collect();

    if parts.len() < 6 {
        return Err(XorcistError::JjError(format!(
            "unexpected show output format: expected 6 fields, got {}",
            parts.len()
        )));
    }

    let bookmarks = if parts[5].is_empty() {
        Vec::new()
    } else {
        parts[5].split(',').map(String::from).collect()
    };

    // Trim trailing newline from description (jj adds one at the end)
    let description = parts[4].trim_end_matches('\n').to_string();

    Ok(ShowMeta {
        change_id: parts[0].to_string(),
        commit_id: parts[1].to_string(),
        author: parts[2].to_string(),
        timestamp: parts[3].to_string(),
        description,
        bookmarks,
    })
}

/// Parse diff summary output from jj diff --summary.
fn parse_diff_summary(output: &str) -> Vec<DiffEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            // Format: "M path/to/file.rs" or "A new_file.rs"
            let (status_char, path) = line.split_once(' ')?;
            let status = match status_char {
                "A" => DiffStatus::Added,
                "M" => DiffStatus::Modified,
                "D" => DiffStatus::Deleted,
                "R" => DiffStatus::Renamed,
                "C" => DiffStatus::Copied,
                _ => return None,
            };
            Some(DiffEntry {
                status,
                path: path.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_show_meta() {
        let output = "abc123\x00def456\x00Alice\x002 hours ago\x00Add feature\x00main,dev\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.change_id, "abc123");
        assert_eq!(result.commit_id, "def456");
        assert_eq!(result.author, "Alice");
        assert_eq!(result.timestamp, "2 hours ago");
        assert_eq!(result.description, "Add feature");
        assert_eq!(result.bookmarks, vec!["main", "dev"]);
    }

    #[test]
    fn test_parse_show_meta_no_bookmarks() {
        let output = "abc123\x00def456\x00Alice\x002 hours ago\x00Add feature\x00\n";
        let result = parse_show_meta(output).unwrap();

        assert!(result.bookmarks.is_empty());
    }

    #[test]
    fn test_parse_show_meta_multiline_description() {
        // In jj template output, newlines within description are preserved.
        // Our parser handles multi-line descriptions correctly.
        let output =
            "abc123\x00def456\x00Alice\x002 hours ago\x00First line\nSecond line\x00main\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.description, "First line\nSecond line");
        assert_eq!(result.bookmarks, vec!["main"]);
    }

    #[test]
    fn test_parse_show_meta_description_with_trailing_newline() {
        // jj's description often has a trailing newline, which should be trimmed
        let output = "abc123\x00def456\x00Alice\x002 hours ago\x00Add feature\n\x00main\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.description, "Add feature");
        assert_eq!(result.bookmarks, vec!["main"]);
    }

    #[test]
    fn test_parse_diff_summary() {
        let output = r#"A src/new_file.rs
M src/main.rs
D src/old_file.rs
"#;
        let entries = parse_diff_summary(output);

        assert_eq!(entries.len(), 3);

        assert_eq!(entries[0].status, DiffStatus::Added);
        assert_eq!(entries[0].path, "src/new_file.rs");

        assert_eq!(entries[1].status, DiffStatus::Modified);
        assert_eq!(entries[1].path, "src/main.rs");

        assert_eq!(entries[2].status, DiffStatus::Deleted);
        assert_eq!(entries[2].path, "src/old_file.rs");
    }

    #[test]
    fn test_parse_diff_summary_empty() {
        let output = "";
        let entries = parse_diff_summary(output);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_diff_summary_with_spaces_in_path() {
        let output = "M path/with spaces/file.rs\n";
        let entries = parse_diff_summary(output);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "path/with spaces/file.rs");
    }
}
