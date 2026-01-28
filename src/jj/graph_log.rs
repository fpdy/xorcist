//! Graph log fetching and parsing for jj.
//!
//! This module provides functionality to fetch jj log output with graph visualization
//! and parse it into a structured format for TUI display.

use regex::Regex;
use std::sync::LazyLock;

use crate::error::XorcistError;
use crate::jj::runner::JjRunner;

/// Template for graph log output with shortened timestamps and bookmarks.
///
/// Format: `change_id author timestamp [bookmarks] description`
/// - change_id: 8-character shortest unique prefix
/// - author: author name
/// - timestamp: shortened format (e.g., "12h" instead of "12 hours ago")
/// - bookmarks: comma-separated bookmark names wrapped in brackets (if any)
/// - description: first line of commit message
const GRAPH_LOG_TEMPLATE: &str = r#"separate(" ", change_id.shortest(8), author.name(), author.timestamp().ago().replace(regex:"\\s+seconds? ago", "s").replace(regex:"\\s+minutes? ago", "m").replace(regex:"\\s+hours? ago", "h").replace(regex:"\\s+days? ago", "d").replace(regex:"\\s+weeks? ago", "w").replace(regex:"\\s+months? ago", "mo").replace(regex:"\\s+years? ago", "y"), if(bookmarks, "[" ++ bookmarks.map(|b| b.name()).join(",") ++ "]"), description.first_line())"#;

/// Regex pattern for extracting change_id from graph output.
/// Matches 8 lowercase letters after graph symbols.
static CHANGE_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match after graph symbols (@, ◆, ○, ●, etc.) and whitespace
    // The change_id is 8 lowercase letters
    Regex::new(r"^[^a-z]*([a-z]{8})\s").expect("Invalid regex pattern")
});

/// Regex pattern for extracting all fields from a commit line.
/// Format: `change_id author timestamp [bookmarks] description`
static COMMIT_LINE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match: graph_symbols change_id(8 letters) author timestamp [bookmarks]? description
    // - graph_symbols: non-letter characters at the start
    // - change_id: exactly 8 lowercase letters
    // - author: non-whitespace characters
    // - timestamp: non-whitespace characters (e.g., "1h", "2d", "3mo")
    // - bookmarks: optional, wrapped in [] (e.g., "[main,dev]")
    // - description: everything after (may be empty)
    Regex::new(r"^[^a-z]*([a-z]{8})\s+(\S+)\s+(\S+)\s*(?:\[([^\]]*)\]\s*)?(.*)$")
        .expect("Invalid regex pattern")
});

/// Regex pattern to strip ANSI escape sequences.
static ANSI_STRIP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*m").expect("Invalid ANSI regex pattern"));

/// A single line from the graph log output.
#[derive(Debug, Clone)]
pub struct GraphLine {
    /// Raw line text with ANSI codes.
    pub raw: String,
    /// Plain text without ANSI codes (for parsing).
    pub plain: String,
    /// Change ID extracted from this line, if any.
    pub change_id: Option<String>,
    /// Description extracted from this line, if any.
    /// Empty string if the commit has no description.
    pub description: Option<String>,
    /// Line index in the full output.
    pub line_index: usize,
}

impl GraphLine {
    /// Create a new GraphLine from raw text.
    fn new(raw: String, line_index: usize) -> Self {
        let plain = strip_ansi(&raw);
        let (change_id, description) = extract_commit_fields(&plain);
        Self {
            raw,
            plain,
            change_id,
            description,
            line_index,
        }
    }

    /// Check if this line contains a commit entry (has a change_id).
    pub fn is_commit_line(&self) -> bool {
        self.change_id.is_some()
    }
}

/// Complete graph log with all lines and selection metadata.
#[derive(Debug, Clone, Default)]
pub struct GraphLog {
    /// All lines from the graph output.
    pub lines: Vec<GraphLine>,
    /// Indices of lines that contain commits (are selectable).
    pub commit_line_indices: Vec<usize>,
}

impl GraphLog {
    /// Create a new GraphLog from raw jj output.
    pub fn from_output(output: &str) -> Self {
        let lines: Vec<GraphLine> = output
            .lines()
            .enumerate()
            .map(|(idx, line)| GraphLine::new(line.to_string(), idx))
            .collect();

        let commit_line_indices: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.is_commit_line())
            .map(|(idx, _)| idx)
            .collect();

        Self {
            lines,
            commit_line_indices,
        }
    }

    /// Get the number of selectable commits.
    pub fn commit_count(&self) -> usize {
        self.commit_line_indices.len()
    }

    /// Get the line index for a given selection index.
    pub fn line_index_for_selection(&self, selection: usize) -> Option<usize> {
        self.commit_line_indices.get(selection).copied()
    }

    /// Get the change_id for a given selection index.
    pub fn change_id_for_selection(&self, selection: usize) -> Option<&str> {
        let line_idx = self.line_index_for_selection(selection)?;
        self.lines[line_idx].change_id.as_deref()
    }

    /// Check if the log is empty.
    pub fn is_empty(&self) -> bool {
        self.commit_line_indices.is_empty()
    }

    /// Extend this graph log with another one.
    ///
    /// This is used for incremental loading of more entries.
    pub fn extend(&mut self, other: GraphLog) {
        let offset = self.lines.len();
        for mut line in other.lines {
            line.line_index += offset;
            self.lines.push(line);
        }
        for idx in other.commit_line_indices {
            self.commit_line_indices.push(idx + offset);
        }
    }
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    ANSI_STRIP_REGEX.replace_all(s, "").to_string()
}

/// Extract change_id from a plain text line.
///
/// The change_id is the first 8 lowercase letters after graph symbols.
#[allow(dead_code)]
fn extract_change_id(plain: &str) -> Option<String> {
    CHANGE_ID_REGEX
        .captures(plain)
        .map(|cap| cap[1].to_string())
}

/// Extract change_id and description from a plain text commit line.
///
/// Returns (change_id, description) where description is Some for commit lines.
/// Note: bookmarks (group 4) are handled by the template itself - they appear in the raw output.
fn extract_commit_fields(plain: &str) -> (Option<String>, Option<String>) {
    match COMMIT_LINE_REGEX.captures(plain) {
        Some(cap) => {
            let change_id = cap[1].to_string();
            // Group 5 is the description (after optional [bookmarks])
            let description = cap.get(5).map(|m| m.as_str().to_string());
            (Some(change_id), description)
        }
        None => (None, None),
    }
}

/// Fetch graph log from jj with colored output.
pub fn fetch_graph_log(runner: &JjRunner, limit: Option<usize>) -> Result<GraphLog, XorcistError> {
    let mut args = vec![
        "log",
        "--color",
        "always",
        "-T",
        GRAPH_LOG_TEMPLATE,
        "-r",
        "::",
    ];

    let limit_str;
    if let Some(n) = limit {
        limit_str = n.to_string();
        args.push("-n");
        args.push(&limit_str);
    }

    let output = runner.run_capture(&args)?;
    Ok(GraphLog::from_output(&output))
}

/// Fetch additional graph log entries after a given change_id.
pub fn fetch_graph_log_after(
    runner: &JjRunner,
    after_change_id: &str,
    limit: usize,
) -> Result<GraphLog, XorcistError> {
    let revset = format!("::{after_change_id}-");
    let limit_str = limit.to_string();

    let args = vec![
        "log",
        "--color",
        "always",
        "-T",
        GRAPH_LOG_TEMPLATE,
        "-r",
        &revset,
        "-n",
        &limit_str,
    ];

    let output = runner.run_capture(&args)?;
    Ok(GraphLog::from_output(&output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[1m\x1b[38;5;5mq\x1b[0m\x1b[38;5;8mzmtztvn\x1b[39m test";
        let result = strip_ansi(input);
        assert_eq!(result, "qzmtztvn test");
    }

    #[test]
    fn test_extract_change_id_simple() {
        // Working copy marker
        let line = "@  qzmtztvn 1XD 11m feat: test";
        assert_eq!(extract_change_id(line), Some("qzmtztvn".to_string()));

        // Regular commit marker
        let line = "◆  rvzpxnov 1XD 12h refactor: something";
        assert_eq!(extract_change_id(line), Some("rvzpxnov".to_string()));

        // Circle marker
        let line = "○  abcdefgh Author 1d fix: bug";
        assert_eq!(extract_change_id(line), Some("abcdefgh".to_string()));
    }

    #[test]
    fn test_extract_change_id_with_graph_branches() {
        // Branch point
        let line = "├─╮";
        assert_eq!(extract_change_id(line), None);

        // Vertical line
        let line = "│ ◆  xyzwvuts 1XD 1h test";
        assert_eq!(extract_change_id(line), Some("xyzwvuts".to_string()));

        // Merge line with content
        let line = "├─╯";
        assert_eq!(extract_change_id(line), None);
    }

    #[test]
    fn test_extract_change_id_edge_cases() {
        // Empty line
        assert_eq!(extract_change_id(""), None);

        // Only graph symbols
        assert_eq!(extract_change_id("│  "), None);

        // Too short id (should not match)
        assert_eq!(extract_change_id("@  abc 1XD 1h test"), None);
    }

    #[test]
    fn test_graph_line_creation() {
        let raw = "\x1b[1m@\x1b[0m  \x1b[1m\x1b[38;5;5mq\x1b[0mzmtztvn 1XD 11m feat: test";
        let line = GraphLine::new(raw.to_string(), 0);

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("feat: test".to_string()));
        assert_eq!(line.line_index, 0);
    }

    #[test]
    fn test_graph_line_empty_description() {
        let raw = "@  qzmtztvn Author 1h ";
        let line = GraphLine::new(raw.to_string(), 0);

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("".to_string()));
    }

    #[test]
    fn test_graph_line_no_description() {
        // Line with no trailing space - description should still be captured as empty
        let raw = "@  qzmtztvn Author 1h";
        let line = GraphLine::new(raw.to_string(), 0);

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("".to_string()));
    }

    #[test]
    fn test_extract_commit_fields() {
        // Normal commit with description
        let (cid, desc) = extract_commit_fields("@  qzmtztvn Author 1h feat: add feature");
        assert_eq!(cid, Some("qzmtztvn".to_string()));
        assert_eq!(desc, Some("feat: add feature".to_string()));

        // Commit with empty description
        let (cid, desc) = extract_commit_fields("@  qzmtztvn Author 1h ");
        assert_eq!(cid, Some("qzmtztvn".to_string()));
        assert_eq!(desc, Some("".to_string()));

        // Non-commit line (graph branch)
        let (cid, desc) = extract_commit_fields("├─╮");
        assert_eq!(cid, None);
        assert_eq!(desc, None);
    }

    #[test]
    fn test_graph_log_from_output() {
        let output = "@  qzmtztvn 1XD 11m feat: test
◆  rvzpxnov 1XD 12h refactor: something
├─╮
│ ◆  xyzwvuts 1XD 1h test
├─╯
◆  abcdefgh 1XD 1d init";

        let log = GraphLog::from_output(output);

        assert_eq!(log.lines.len(), 6);
        assert_eq!(log.commit_count(), 4);
        assert_eq!(log.commit_line_indices, vec![0, 1, 3, 5]);

        assert_eq!(log.change_id_for_selection(0), Some("qzmtztvn"));
        assert_eq!(log.change_id_for_selection(1), Some("rvzpxnov"));
        assert_eq!(log.change_id_for_selection(2), Some("xyzwvuts"));
        assert_eq!(log.change_id_for_selection(3), Some("abcdefgh"));
        assert_eq!(log.change_id_for_selection(4), None);
    }

    #[test]
    fn test_graph_log_empty() {
        let log = GraphLog::from_output("");
        assert!(log.is_empty());
        assert_eq!(log.commit_count(), 0);
    }
}
