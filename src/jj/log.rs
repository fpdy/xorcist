//! jj log parsing and fetching.

use crate::error::XorcistError;
use crate::jj::runner::JjRunner;

/// A single log entry from jj log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Short change ID (e.g., "abc123").
    pub change_id: String,
    /// Shortest unique prefix of change ID.
    pub change_id_prefix: String,
    /// Rest of change ID after the unique prefix.
    pub change_id_rest: String,
    /// Short commit ID.
    #[allow(dead_code)] // Will be used for commit details view
    pub commit_id: String,
    /// Shortest unique prefix of commit ID.
    #[allow(dead_code)]
    pub commit_id_prefix: String,
    /// Rest of commit ID after the unique prefix.
    #[allow(dead_code)]
    pub commit_id_rest: String,
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
/// Uses shortest() to get unique prefix for change_id and commit_id.
const LOG_TEMPLATE: &str = r#"change_id.shortest(4).prefix() ++ "\x00" ++ change_id.shortest(4).rest() ++ "\x00" ++ commit_id.shortest(4).prefix() ++ "\x00" ++ commit_id.shortest(4).rest() ++ "\x00" ++ author.name() ++ "\x00" ++ committer.timestamp().ago() ++ "\x00" ++ coalesce(description.first_line(), "(no description)") ++ "\x00" ++ current_working_copy ++ "\x00" ++ immutable ++ "\x00" ++ empty ++ "\x00" ++ bookmarks.join(",") ++ "\n""#;

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
    if parts.len() < 11 {
        return None;
    }

    let bookmarks = if parts[10].is_empty() {
        Vec::new()
    } else {
        parts[10].split(',').map(String::from).collect()
    };

    let change_id_prefix = parts[0].to_string();
    let change_id_rest = parts[1].to_string();
    let commit_id_prefix = parts[2].to_string();
    let commit_id_rest = parts[3].to_string();

    Some(LogEntry {
        change_id: format!("{change_id_prefix}{change_id_rest}"),
        change_id_prefix,
        change_id_rest,
        commit_id: format!("{commit_id_prefix}{commit_id_rest}"),
        commit_id_prefix,
        commit_id_rest,
        author: parts[4].to_string(),
        timestamp: parts[5].to_string(),
        description: parts[6].to_string(),
        is_working_copy: parts[7] == "true",
        is_immutable: parts[8] == "true",
        is_empty: parts[9] == "true",
        bookmarks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        // Format: change_prefix\0change_rest\0commit_prefix\0commit_rest\0author\0timestamp\0description\0working_copy\0immutable\0empty\0bookmarks
        let line = "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\x00true\x00false\x00false\x00main,dev";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.change_id_prefix, "abc");
        assert_eq!(entry.change_id_rest, "123");
        assert_eq!(entry.change_id, "abc123");
        assert_eq!(entry.commit_id_prefix, "def");
        assert_eq!(entry.commit_id_rest, "456");
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
        let line = "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\x00false\x00true\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert!(entry.bookmarks.is_empty());
        assert!(!entry.is_working_copy);
        assert!(entry.is_immutable);
    }

    #[test]
    fn test_parse_log_line_empty_rest() {
        // When the entire ID is the unique prefix, rest is empty
        let line = "abcd\x00\x00defg\x00\x00Alice\x00now\x00Test\x00false\x00false\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.change_id_prefix, "abcd");
        assert!(entry.change_id_rest.is_empty());
        assert_eq!(entry.change_id, "abcd");
        assert_eq!(entry.commit_id_prefix, "defg");
        assert!(entry.commit_id_rest.is_empty());
    }

    #[test]
    fn test_graph_symbol() {
        let mut entry = LogEntry {
            change_id: "abc123".to_string(),
            change_id_prefix: "abc".to_string(),
            change_id_rest: "123".to_string(),
            commit_id: "def456".to_string(),
            commit_id_prefix: "def".to_string(),
            commit_id_rest: "456".to_string(),
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
        // Format: change_prefix\0change_rest\0commit_prefix\0commit_rest\0author\0timestamp\0description\0working_copy\0immutable\0empty\0bookmarks
        let output = "abc\x00123\x00def\x00456\x00Alice\x00now\x00First\x00true\x00false\x00false\x00\nghi\x00789\x00jkl\x00012\x00Bob\x001h ago\x00Second\x00false\x00false\x00false\x00main\n";
        let entries = parse_log_output(output);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].change_id_prefix, "abc");
        assert_eq!(entries[0].change_id_rest, "123");
        assert_eq!(entries[0].change_id, "abc123");
        assert_eq!(entries[1].change_id, "ghi789");
        assert_eq!(entries[1].bookmarks, vec!["main"]);
    }
}
