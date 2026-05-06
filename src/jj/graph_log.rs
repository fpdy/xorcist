//! Graph log fetching and parsing for jj.
//!
//! This module provides functionality to fetch jj log output with graph visualization
//! and parse it into a structured format for TUI display.

use regex::Regex;
use std::sync::LazyLock;

use crate::error::XorcistError;
use crate::jj::runner::JjBackend;

/// Template for graph log output with shortened timestamps and bookmarks.
///
/// Format: `change_id author timestamp [bookmarks] description`
/// - change_id: 8-character shortest unique prefix
/// - author: author name
/// - timestamp: shortened format (e.g., "12h" instead of "12 hours ago")
/// - bookmarks: comma-separated bookmark names wrapped in brackets (if any)
/// - description: first line of commit message
const GRAPH_LOG_TEMPLATE: &str = r#"separate(" ", "\x1f" ++ change_id.shortest(8) ++ "\x1f", author.name(), author.timestamp().ago().replace(regex:"\\s+seconds? ago", "s").replace(regex:"\\s+minutes? ago", "m").replace(regex:"\\s+hours? ago", "h").replace(regex:"\\s+days? ago", "d").replace(regex:"\\s+weeks? ago", "w").replace(regex:"\\s+months? ago", "mo").replace(regex:"\\s+years? ago", "y"), if(bookmarks, "[" ++ bookmarks.map(|b| b.name()).join(",") ++ "]"), "\x1e" ++ description.first_line() ++ "\x1e")"#;

const CHANGE_ID_MARKER: char = '\x1f';
const DESCRIPTION_MARKER: char = '\x1e';

/// Regex pattern for extracting change_id from graph output.
/// Matches lowercase jj change ids after graph symbols.
#[cfg(test)]
static CHANGE_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match after graph symbols (@, ◆, ○, ●, etc.) and whitespace
    // The change_id is lowercase letters and may be longer than 8 chars when
    // jj needs a longer unique prefix.
    Regex::new(r"^[^a-z]*([a-z]{8,})\s").expect("Invalid regex pattern")
});

/// Regex pattern for extracting all fields from a commit line.
/// Format: `change_id author timestamp [bookmarks] description`
static COMMIT_LINE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match: graph_symbols change_id author timestamp [bookmarks]? description
    // - graph_symbols: non-letter characters at the start
    // - change_id: at least 8 lowercase letters
    // - author: anything up to the timestamp (supports spaces)
    // - timestamp: common compact jj display tokens; this is intentionally
    //   broader than the template's current replacements.
    // - bookmarks: optional, wrapped in [] (e.g., "[main,dev]")
    // - description: everything after (may be empty)
    Regex::new(r"^[^a-z]*([a-z]{8,})\s+(.+?)\s+(now|yesterday|\d{4}-\d{2}-\d{2}|\d+(?:mo|[smhdwy]))\s*(?:\[([^\]]*)\]\s*)?(.*)$")
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
}

impl GraphLine {
    /// Create a new GraphLine from raw text.
    fn new(raw: String) -> Self {
        let parse_plain = strip_ansi(&raw);
        let (change_id, description) = extract_commit_fields(&parse_plain);
        let raw = strip_parse_markers(&raw);
        let plain = strip_parse_markers(&parse_plain);
        Self {
            raw,
            plain,
            change_id,
            description,
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
            .map(|line| GraphLine::new(line.to_string()))
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
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.commit_line_indices.is_empty()
    }

    /// Find the selectable index for a change id.
    pub fn selection_for_change_id(&self, change_id: &str) -> Option<usize> {
        self.commit_line_indices
            .iter()
            .position(|&line_idx| self.lines[line_idx].change_id.as_deref() == Some(change_id))
    }
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    ANSI_STRIP_REGEX.replace_all(s, "").to_string()
}

fn strip_parse_markers(s: &str) -> String {
    s.chars()
        .filter(|ch| *ch != CHANGE_ID_MARKER && *ch != DESCRIPTION_MARKER)
        .collect()
}

/// Extract change_id from a plain text line.
///
/// The change_id is the first lowercase id after graph symbols.
#[cfg(test)]
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
    if let Some(fields) = extract_marked_commit_fields(plain) {
        return fields;
    }

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

fn extract_marked_commit_fields(plain: &str) -> Option<(Option<String>, Option<String>)> {
    let change_id = extract_between_markers(plain, CHANGE_ID_MARKER)?;
    let description = extract_between_markers(plain, DESCRIPTION_MARKER).unwrap_or_default();
    if change_id.len() < 8 || !change_id.chars().all(|ch| ch.is_ascii_lowercase()) {
        return None;
    }
    Some((Some(change_id), Some(description)))
}

fn extract_between_markers(s: &str, marker: char) -> Option<String> {
    let start = s.find(marker)?;
    let rest = &s[start + marker.len_utf8()..];
    let end = rest.find(marker)?;
    Some(rest[..end].to_string())
}

/// Fetch graph log from jj with colored output.
pub fn fetch_graph_log(
    runner: &dyn JjBackend,
    limit: Option<usize>,
) -> Result<GraphLog, XorcistError> {
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
    fn test_change_id_contract_accepts_at_least_8_characters() {
        assert_eq!(extract_change_id("@  abcdefg Author 1h too short"), None);
        assert_eq!(
            extract_change_id("@  abcdefgh Author 1h exact"),
            Some("abcdefgh".to_string())
        );
        assert_eq!(
            extract_change_id("@  abcdefghi Author 1h longer"),
            Some("abcdefghi".to_string())
        );

        let too_short = GraphLine::new("@  abcdefg Author 1h too short".to_string());
        let exact = GraphLine::new("@  abcdefgh Author 1h exact".to_string());
        let longer = GraphLine::new("@  abcdefghi Author 1h longer".to_string());

        assert!(!too_short.is_commit_line());
        assert!(exact.is_commit_line());
        assert!(longer.is_commit_line());
    }

    #[test]
    fn test_graph_line_tolerates_timestamp_token_changes() {
        let line = GraphLine::new("@  qzmtztvn Alice yesterday feat: test".to_string());

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("feat: test".to_string()));
    }

    #[test]
    fn test_graph_line_prefers_marked_fields_and_hides_markers() {
        let raw = "@  \x1fqzmtztvn\x1f Alice any timestamp [main] \x1efeat: marked\x1e";
        let line = GraphLine::new(raw.to_string());

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("feat: marked".to_string()));
        assert_eq!(
            line.plain,
            "@  qzmtztvn Alice any timestamp [main] feat: marked"
        );
        assert_eq!(
            line.raw,
            "@  qzmtztvn Alice any timestamp [main] feat: marked"
        );
    }

    #[test]
    fn test_selection_for_change_id() {
        let log =
            GraphLog::from_output("@  qzmtztvn Author 1h first\n◆  abcdefghi Author 1d second");

        assert_eq!(log.selection_for_change_id("qzmtztvn"), Some(0));
        assert_eq!(log.selection_for_change_id("abcdefghi"), Some(1));
        assert_eq!(log.selection_for_change_id("missing"), None);
    }

    #[test]
    fn test_graph_line_creation() {
        let raw = "\x1b[1m@\x1b[0m  \x1b[1m\x1b[38;5;5mq\x1b[0mzmtztvn 1XD 11m feat: test";
        let line = GraphLine::new(raw.to_string());

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("feat: test".to_string()));
    }

    #[test]
    fn test_graph_line_author_name_with_spaces() {
        let line = GraphLine::new("@  qzmtztvn Alice Example 11m feat: test".to_string());

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("feat: test".to_string()));
    }

    #[test]
    fn test_graph_line_empty_description() {
        let raw = "@  qzmtztvn Author 1h ";
        let line = GraphLine::new(raw.to_string());

        assert!(line.is_commit_line());
        assert_eq!(line.change_id, Some("qzmtztvn".to_string()));
        assert_eq!(line.description, Some("".to_string()));
    }

    #[test]
    fn test_graph_line_no_description() {
        // Line with no trailing space - description should still be captured as empty
        let raw = "@  qzmtztvn Author 1h";
        let line = GraphLine::new(raw.to_string());

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
    fn test_graph_only_lines_remain_visible_but_not_selectable() {
        let output = "@  qzmtztvn 1XD 11m feat: test
│
├─╮
│ ◆  xyzwvuts 1XD 1h test
├─╯";

        let log = GraphLog::from_output(output);

        assert_eq!(log.lines.len(), 5);
        assert_eq!(log.lines[1].plain, "│");
        assert_eq!(log.lines[2].plain, "├─╮");
        assert_eq!(log.lines[4].plain, "├─╯");
        assert_eq!(log.commit_line_indices, vec![0, 3]);
        assert_eq!(log.line_index_for_selection(0), Some(0));
        assert_eq!(log.line_index_for_selection(1), Some(3));
        assert_eq!(log.line_index_for_selection(2), None);
    }

    #[test]
    fn test_graph_log_empty() {
        let log = GraphLog::from_output("");
        assert!(log.is_empty());
        assert_eq!(log.commit_count(), 0);
    }
}
