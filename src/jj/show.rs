//! jj show command execution.

use crate::error::XorcistError;
use crate::jj::runner::JjBackend;

/// Output from jj show command.
#[derive(Debug, Clone)]
pub struct ShowOutput {
    /// Change ID (full).
    pub change_id: String,
    /// Shortest unique prefix of change ID.
    pub change_id_prefix: String,
    /// Rest of change ID after the unique prefix.
    pub change_id_rest: String,
    /// Commit ID (full).
    #[allow(dead_code)] // Will be used for copy-to-clipboard etc.
    pub commit_id: String,
    /// Shortest unique prefix of commit ID.
    pub commit_id_prefix: String,
    /// Rest of commit ID after the unique prefix.
    pub commit_id_rest: String,
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
/// Uses shortest() to get unique prefix for change_id and commit_id.
const SHOW_TEMPLATE: &str = r#"change_id.shortest(4).prefix() ++ "\x00" ++ change_id.shortest(4).rest() ++ "\x00" ++ commit_id.shortest(4).prefix() ++ "\x00" ++ commit_id.shortest(4).rest() ++ "\x00" ++ author.name() ++ "\x00" ++ committer.timestamp().ago() ++ "\x00" ++ description ++ "\x00" ++ bookmarks.join(",") ++ "\n""#;

/// Number of null-byte-separated metadata fields emitted by `SHOW_TEMPLATE`.
const SHOW_META_FIELD_COUNT: usize = ShowMetaField::COUNT;

/// Field order for metadata emitted by `SHOW_TEMPLATE`.
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum ShowMetaField {
    ChangeIdPrefix = 0,
    ChangeIdRest = 1,
    CommitIdPrefix = 2,
    CommitIdRest = 3,
    Author = 4,
    Timestamp = 5,
    Description = 6,
    Bookmarks = 7,
}

impl ShowMetaField {
    const COUNT: usize = 8;

    const fn index(self) -> usize {
        self as usize
    }
}

/// Fetch show output for a revision.
pub fn fetch_show(runner: &dyn JjBackend, revision: &str) -> Result<ShowOutput, XorcistError> {
    // 1. Fetch metadata using template
    let meta_output =
        runner.run_capture(&["log", "-r", revision, "--no-graph", "-T", SHOW_TEMPLATE])?;
    let meta = parse_show_meta(&meta_output)?;

    // 2. Fetch diff summary
    let diff_output = runner.run_capture(&["diff", "-r", revision, "--summary"])?;
    let diff_summary = parse_diff_summary(&diff_output);

    Ok(ShowOutput {
        change_id: meta.change_id,
        change_id_prefix: meta.change_id_prefix,
        change_id_rest: meta.change_id_rest,
        commit_id: meta.commit_id,
        commit_id_prefix: meta.commit_id_prefix,
        commit_id_rest: meta.commit_id_rest,
        author: meta.author,
        timestamp: meta.timestamp,
        description: meta.description,
        bookmarks: meta.bookmarks,
        diff_summary,
    })
}

/// Fetch diff output for a specific file in a revision.
pub fn fetch_diff_file(
    runner: &dyn JjBackend,
    revision: &str,
    path: &str,
) -> Result<String, XorcistError> {
    runner.run_capture(&["diff", "-r", revision, "--color=never", "--git", "--", path])
}

/// Parsed metadata from jj log output.
#[derive(Debug)]
struct ShowMeta {
    change_id: String,
    change_id_prefix: String,
    change_id_rest: String,
    commit_id: String,
    commit_id_prefix: String,
    commit_id_rest: String,
    author: String,
    timestamp: String,
    description: String,
    bookmarks: Vec<String>,
}

/// Parse metadata from jj log output.
///
/// The output format is: change_prefix\x00change_rest\x00commit_prefix\x00commit_rest\x00author\x00timestamp\x00description\x00bookmarks\n
/// Note: description may contain newlines, so we split by \x00 on the entire output
/// rather than processing line by line.
fn parse_show_meta(output: &str) -> Result<ShowMeta, XorcistError> {
    // Remove trailing newline if present, then split by null byte
    let output = output.trim_end_matches('\n');
    let parts: Vec<&str> = output.split('\x00').collect();

    if parts.len() != SHOW_META_FIELD_COUNT {
        return Err(XorcistError::JjError(format!(
            "unexpected show output format: expected {} fields, got {}",
            SHOW_META_FIELD_COUNT,
            parts.len()
        )));
    }

    let bookmarks = super::parse_bookmarks_field(parts[ShowMetaField::Bookmarks.index()]);

    // Trim trailing newline from description (jj adds one at the end)
    let description = parts[ShowMetaField::Description.index()]
        .trim_end_matches('\n')
        .to_string();

    let change_id_prefix = parts[ShowMetaField::ChangeIdPrefix.index()].to_string();
    let change_id_rest = parts[ShowMetaField::ChangeIdRest.index()].to_string();
    let commit_id_prefix = parts[ShowMetaField::CommitIdPrefix.index()].to_string();
    let commit_id_rest = parts[ShowMetaField::CommitIdRest.index()].to_string();

    Ok(ShowMeta {
        change_id: format!("{change_id_prefix}{change_id_rest}"),
        change_id_prefix,
        change_id_rest,
        commit_id: format!("{commit_id_prefix}{commit_id_rest}"),
        commit_id_prefix,
        commit_id_rest,
        author: parts[ShowMetaField::Author.index()].to_string(),
        timestamp: parts[ShowMetaField::Timestamp.index()].to_string(),
        description,
        bookmarks,
    })
}

/// Parse diff summary output from jj diff --summary.
pub(crate) fn parse_diff_summary(output: &str) -> Vec<DiffEntry> {
    output
        .lines()
        .filter_map(|line| {
            if line.is_empty() {
                return None;
            }

            // Format: "M path/to/file.rs" or "A new_file.rs".
            // Preserve path bytes after the separator, including leading/trailing spaces.
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
    fn test_show_meta_field_count_matches_template_contract() {
        assert_eq!(SHOW_META_FIELD_COUNT, 8);
        assert_eq!(ShowMetaField::ChangeIdPrefix.index(), 0);
        assert_eq!(ShowMetaField::ChangeIdRest.index(), 1);
        assert_eq!(ShowMetaField::CommitIdPrefix.index(), 2);
        assert_eq!(ShowMetaField::CommitIdRest.index(), 3);
        assert_eq!(ShowMetaField::Author.index(), 4);
        assert_eq!(ShowMetaField::Timestamp.index(), 5);
        assert_eq!(ShowMetaField::Description.index(), 6);
        assert_eq!(ShowMetaField::Bookmarks.index(), 7);
    }

    #[test]
    fn test_parse_show_meta() {
        // Format: change_prefix\0change_rest\0commit_prefix\0commit_rest\0author\0timestamp\0description\0bookmarks
        let output =
            "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\x00main,dev\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.change_id_prefix, "abc");
        assert_eq!(result.change_id_rest, "123");
        assert_eq!(result.change_id, "abc123");
        assert_eq!(result.commit_id_prefix, "def");
        assert_eq!(result.commit_id_rest, "456");
        assert_eq!(result.commit_id, "def456");
        assert_eq!(result.author, "Alice");
        assert_eq!(result.timestamp, "2 hours ago");
        assert_eq!(result.description, "Add feature");
        assert_eq!(result.bookmarks, vec!["main", "dev"]);
    }

    #[test]
    fn test_parse_show_meta_field_order_contract() {
        let output = "change-prefix\x00change-rest\x00commit-prefix\x00commit-rest\x00author-name\x00timestamp-value\x00description text\x00bookmark-a,bookmark-b\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.change_id_prefix, "change-prefix");
        assert_eq!(result.change_id_rest, "change-rest");
        assert_eq!(result.change_id, "change-prefixchange-rest");
        assert_eq!(result.commit_id_prefix, "commit-prefix");
        assert_eq!(result.commit_id_rest, "commit-rest");
        assert_eq!(result.commit_id, "commit-prefixcommit-rest");
        assert_eq!(result.author, "author-name");
        assert_eq!(result.timestamp, "timestamp-value");
        assert_eq!(result.description, "description text");
        assert_eq!(result.bookmarks, vec!["bookmark-a", "bookmark-b"]);
    }

    #[test]
    fn test_parse_show_meta_no_bookmarks() {
        let output = "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\x00\n";
        let result = parse_show_meta(output).unwrap();

        assert!(result.bookmarks.is_empty());
    }

    #[test]
    fn test_parse_show_meta_multiline_description() {
        // In jj template output, newlines within description are preserved.
        // Our parser handles multi-line descriptions correctly.
        let output =
            "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00First line\nSecond line\x00main\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.description, "First line\nSecond line");
        assert_eq!(result.bookmarks, vec!["main"]);
    }

    #[test]
    fn test_parse_show_meta_description_with_trailing_newline() {
        // jj's description often has a trailing newline, which should be trimmed
        let output = "abc\x00123\x00def\x00456\x00Alice\x002 hours ago\x00Add feature\n\x00main\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.description, "Add feature");
        assert_eq!(result.bookmarks, vec!["main"]);
    }

    #[test]
    fn test_parse_show_meta_empty_rest() {
        // When the entire ID is the unique prefix, rest is empty
        let output = "abcd\x00\x00defg\x00\x00Alice\x00now\x00Test\x00\n";
        let result = parse_show_meta(output).unwrap();

        assert_eq!(result.change_id_prefix, "abcd");
        assert!(result.change_id_rest.is_empty());
        assert_eq!(result.change_id, "abcd");
        assert_eq!(result.commit_id_prefix, "defg");
        assert!(result.commit_id_rest.is_empty());
    }

    #[test]
    fn test_parse_show_meta_errors_on_wrong_field_count() {
        let too_few =
            parse_show_meta("change\x00rest\x00commit\x00rest\x00author\x00time\x00description\n")
                .unwrap_err();
        assert!(
            too_few
                .to_string()
                .contains("unexpected show output format: expected 8 fields, got 7")
        );

        let too_many = parse_show_meta(
            "change\x00rest\x00commit\x00rest\x00author\x00time\x00description\x00bookmark\x00extra\n",
        )
        .unwrap_err();
        assert!(
            too_many
                .to_string()
                .contains("unexpected show output format: expected 8 fields, got 9")
        );
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
    fn test_parse_diff_summary_renamed_and_copied() {
        let output = r#"R src/old.rs -> src/new.rs
C src/template.rs -> src/copy.rs
"#;
        let entries = parse_diff_summary(output);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, DiffStatus::Renamed);
        assert_eq!(entries[0].path, "src/old.rs -> src/new.rs");
        assert_eq!(entries[1].status, DiffStatus::Copied);
        assert_eq!(entries[1].path, "src/template.rs -> src/copy.rs");
    }

    #[test]
    fn test_parse_diff_summary_with_spaces_in_path() {
        let output = "M path/with spaces/file.rs\n";
        let entries = parse_diff_summary(output);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "path/with spaces/file.rs");
    }

    #[test]
    fn test_parse_diff_summary_preserves_edge_spaces_in_path() {
        let output = "M  leading and trailing .rs \n";
        let entries = parse_diff_summary(output);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, " leading and trailing .rs ");
    }
}
