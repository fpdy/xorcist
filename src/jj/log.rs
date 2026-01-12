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
    /// Whether this commit is empty.
    pub is_empty: bool,
    /// Bookmarks pointing to this commit.
    pub bookmarks: Vec<String>,
}

/// Template for machine-readable log output.
/// Fields are separated by \x00 (null byte) for reliable parsing.
/// Uses shortest() to get unique prefix for change_id and commit_id.
const LOG_TEMPLATE: &str = r#"change_id.shortest(4).prefix() ++ "\x00" ++ change_id.shortest(4).rest() ++ "\x00" ++ commit_id.shortest(4).prefix() ++ "\x00" ++ commit_id.shortest(4).rest() ++ "\x00" ++ author.name() ++ "\x00" ++ committer.timestamp().ago() ++ "\x00" ++ coalesce(description.first_line(), "(no description)") ++ "\x00" ++ empty ++ "\x00" ++ bookmarks.join(",") ++ "\n""#;

/// Fetch log entries from jj.
///
/// Uses revset `::` to get all history (not just the default limited view).
pub fn fetch_log(runner: &JjRunner, limit: Option<usize>) -> Result<Vec<LogEntry>, XorcistError> {
    let mut args = vec!["log", "--no-graph", "-T", LOG_TEMPLATE, "-r", "::"];

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

/// Fetch additional log entries starting after the given change_id.
///
/// Uses revset `::change_id-` (ancestors of parent) combined with `-n limit`
/// to get the next batch of commits in topological order.
/// Returns an empty Vec if there are no more entries.
pub fn fetch_log_after(
    runner: &JjRunner,
    after_change_id: &str,
    limit: usize,
) -> Result<Vec<LogEntry>, XorcistError> {
    // ::X- means "all ancestors of X's parent(s)", which excludes X itself
    // Combined with -n limit, this gives us the next `limit` entries in topo order
    let revset = format!("::{after_change_id}-");
    let limit_str = limit.to_string();

    let args = vec![
        "log",
        "--no-graph",
        "-T",
        LOG_TEMPLATE,
        "-r",
        &revset,
        "-n",
        &limit_str,
    ];

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
    // Fields:
    // change_prefix, change_rest, commit_prefix, commit_rest,
    // author, timestamp, description, empty, bookmarks
    if parts.len() < 9 {
        return None;
    }

    let bookmarks = super::parse_bookmarks_field(parts[8]);

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
        is_empty: parts[7] == "true",
        bookmarks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        // Fields:
        // change_prefix\0change_rest\0commit_prefix\0commit_rest\0author\0timestamp\0description\0empty\0bookmarks
        let line =
            "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\x00false\x00main,dev";
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
        assert!(!entry.is_empty);
        assert_eq!(entry.bookmarks, vec!["main", "dev"]);
    }

    #[test]
    fn test_parse_log_line_no_bookmarks() {
        let line = "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert!(entry.bookmarks.is_empty());
        assert!(!entry.is_empty);
    }

    #[test]
    fn test_parse_log_line_empty_rest() {
        // When the entire ID is the unique prefix, rest is empty
        let line = "abcd\x00\x00defg\x00\x00Alice\x00now\x00Test\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.change_id_prefix, "abcd");
        assert!(entry.change_id_rest.is_empty());
        assert_eq!(entry.change_id, "abcd");
        assert_eq!(entry.commit_id_prefix, "defg");
        assert!(entry.commit_id_rest.is_empty());
    }

    #[test]
    fn test_parse_log_output() {
        // Format: change_prefix\0change_rest\0commit_prefix\0commit_rest\0author\0timestamp\0description\0empty\0bookmarks
        let output = "abc\x00123\x00def\x00456\x00Alice\x00now\x00First\x00false\x00\nghi\x00789\x00jkl\x00012\x00Bob\x001h ago\x00Second\x00false\x00main\n";
        let entries = parse_log_output(output);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].change_id_prefix, "abc");
        assert_eq!(entries[0].change_id_rest, "123");
        assert_eq!(entries[0].change_id, "abc123");
        assert!(!entries[0].is_empty);
        assert_eq!(entries[1].change_id, "ghi789");
        assert_eq!(entries[1].bookmarks, vec!["main"]);
    }
}
