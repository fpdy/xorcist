//! jj log parsing and fetching.

use crate::error::XorcistError;
use crate::jj::runner::JjRunner;

/// A single log entry from jj log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Short change ID (e.g., "abc123").
    pub change_id: String,
    /// Short commit ID.
    #[allow(dead_code)] // Will be used for commit details view
    pub commit_id: String,
    /// Author name.
    pub author: String,
    /// Relative timestamp (e.g., "2 hours ago").
    pub timestamp: String,
    /// First line of description.
    pub description: String,
    /// Whether this is the working copy (@).
    pub is_working_copy: bool,
    /// Whether this commit is immutable.
    pub is_immutable: bool,
    /// Whether this commit is empty.
    pub is_empty: bool,
    /// Bookmarks pointing to this commit.
    pub bookmarks: Vec<String>,
}

impl LogEntry {
    /// Get the graph symbol for this entry.
    pub fn graph_symbol(&self) -> &'static str {
        if self.is_working_copy {
            "@"
        } else if self.is_immutable {
            "◆"
        } else {
            "○"
        }
    }
}

/// Template for machine-readable log output.
/// Fields are separated by \x00 (null byte) for reliable parsing.
const LOG_TEMPLATE: &str = r#"change_id.short() ++ "\x00" ++ commit_id.short() ++ "\x00" ++ author.name() ++ "\x00" ++ committer.timestamp().ago() ++ "\x00" ++ coalesce(description.first_line(), "(no description)") ++ "\x00" ++ current_working_copy ++ "\x00" ++ immutable ++ "\x00" ++ empty ++ "\x00" ++ bookmarks.join(",") ++ "\n""#;

/// Fetch log entries from jj.
pub fn fetch_log(runner: &JjRunner, limit: Option<usize>) -> Result<Vec<LogEntry>, XorcistError> {
    let mut args = vec!["log", "--no-graph", "-T", LOG_TEMPLATE];

    let limit_str;
    if let Some(n) = limit {
        limit_str = n.to_string();
        args.push("-n");
        args.push(&limit_str);
    }

    let output = runner.run_capture(&args)?;
    let entries = parse_log_output(&output);
    Ok(entries)
}

/// Parse the log output into LogEntry structs.
fn parse_log_output(output: &str) -> Vec<LogEntry> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(parse_log_line)
        .collect()
}

/// Parse a single log line.
fn parse_log_line(line: &str) -> Option<LogEntry> {
    let parts: Vec<&str> = line.split('\x00').collect();
    if parts.len() < 9 {
        return None;
    }

    let bookmarks = if parts[8].is_empty() {
        Vec::new()
    } else {
        parts[8].split(',').map(String::from).collect()
    };

    Some(LogEntry {
        change_id: parts[0].to_string(),
        commit_id: parts[1].to_string(),
        author: parts[2].to_string(),
        timestamp: parts[3].to_string(),
        description: parts[4].to_string(),
        is_working_copy: parts[5] == "true",
        is_immutable: parts[6] == "true",
        is_empty: parts[7] == "true",
        bookmarks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        let line = "abc123\x00def456\x00Alice\x002 hours ago\x00Add feature\x00true\x00false\x00false\x00main,dev";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.change_id, "abc123");
        assert_eq!(entry.commit_id, "def456");
        assert_eq!(entry.author, "Alice");
        assert_eq!(entry.timestamp, "2 hours ago");
        assert_eq!(entry.description, "Add feature");
        assert!(entry.is_working_copy);
        assert!(!entry.is_immutable);
        assert!(!entry.is_empty);
        assert_eq!(entry.bookmarks, vec!["main", "dev"]);
    }

    #[test]
    fn test_parse_log_line_no_bookmarks() {
        let line =
            "abc123\x00def456\x00Alice\x002 hours ago\x00Add feature\x00false\x00true\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert!(entry.bookmarks.is_empty());
        assert!(!entry.is_working_copy);
        assert!(entry.is_immutable);
    }

    #[test]
    fn test_graph_symbol() {
        let mut entry = LogEntry {
            change_id: "abc".to_string(),
            commit_id: "def".to_string(),
            author: "Alice".to_string(),
            timestamp: "now".to_string(),
            description: "test".to_string(),
            is_working_copy: true,
            is_immutable: false,
            is_empty: false,
            bookmarks: vec![],
        };

        assert_eq!(entry.graph_symbol(), "@");

        entry.is_working_copy = false;
        entry.is_immutable = true;
        assert_eq!(entry.graph_symbol(), "◆");

        entry.is_immutable = false;
        assert_eq!(entry.graph_symbol(), "○");
    }

    #[test]
    fn test_parse_log_output() {
        let output = "abc\x00def\x00Alice\x00now\x00First\x00true\x00false\x00false\x00\nghi\x00jkl\x00Bob\x001h ago\x00Second\x00false\x00false\x00false\x00main\n";
        let entries = parse_log_output(output);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].change_id, "abc");
        assert_eq!(entries[1].change_id, "ghi");
        assert_eq!(entries[1].bookmarks, vec!["main"]);
    }
}
