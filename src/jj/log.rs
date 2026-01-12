//! jj log parsing and fetching.

use std::collections::{BinaryHeap, HashMap, HashSet};

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

    /// Full commit ID (stable identifier used for graph construction).
    pub commit_id_full: String,
    /// Parent commit IDs (full IDs). May contain multiple entries for merge commits.
    pub parent_commit_ids: Vec<String>,

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
/// Also includes full commit id and parent commit ids for DAG rendering.
const LOG_TEMPLATE: &str = r#"change_id.shortest(4).prefix() ++ "\x00" ++ change_id.shortest(4).rest() ++ "\x00" ++ commit_id.shortest(4).prefix() ++ "\x00" ++ commit_id.shortest(4).rest() ++ "\x00" ++ commit_id ++ "\x00" ++ parents.map(|c| c.commit_id()).join(",") ++ "\x00" ++ author.name() ++ "\x00" ++ committer.timestamp().ago() ++ "\x00" ++ coalesce(description.first_line(), "(no description)") ++ "\x00" ++ current_working_copy ++ "\x00" ++ immutable ++ "\x00" ++ empty ++ "\x00" ++ bookmarks.join(",") ++ "\n""#;

/// Fetch log entries from jj.
///
/// Uses revset `::` to get all history (not just the default limited view).
/// Entries are reordered to ensure working copy (@) appears first for correct graph rendering.
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
    let entries = reorder_entries_for_graph(entries);
    Ok(entries)
}

/// Fetch additional log entries starting after the given change_id.
///
/// Uses revset `::change_id-` (ancestors of parent) combined with `-n limit`
/// to get the next batch of commits in topological order.
/// Returns an empty Vec if there are no more entries.
/// Entries are reordered for correct graph rendering.
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
    let entries = reorder_entries_for_graph(entries);
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

/// Priority key for topological sort.
/// Higher priority = should appear earlier in output.
/// Priority order: working_copy (highest) > original_index (lower = earlier)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SortPriority {
    is_working_copy: bool,
    /// Negated original index (so that smaller original index = higher priority)
    neg_original_idx: isize,
}

impl Ord for SortPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // working_copy first (true > false)
        // then by neg_original_idx (higher = earlier original position)
        (self.is_working_copy, self.neg_original_idx)
            .cmp(&(other.is_working_copy, other.neg_original_idx))
    }
}

impl PartialOrd for SortPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Reorder log entries for correct graph rendering.
///
/// jj log --no-graph may return entries in an order that doesn't match
/// the visual graph layout (e.g., a sibling head before working copy).
/// This function performs a topological sort with priority:
/// 1. Working copy (@) always comes first among heads
/// 2. Otherwise, preserve original order as tiebreaker
///
/// This ensures the graph rendering in `graph.rs` produces correct results.
pub fn reorder_entries_for_graph(entries: Vec<LogEntry>) -> Vec<LogEntry> {
    if entries.len() <= 1 {
        return entries;
    }

    let n = entries.len();

    // Build commit_id_full -> index mapping (owned strings to avoid borrow issues)
    let id_to_idx: HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.commit_id_full.clone(), i))
        .collect();

    // Store is_working_copy flags before consuming entries
    let is_working_copy: Vec<bool> = entries.iter().map(|e| e.is_working_copy).collect();

    // Count children for each commit (only within this slice)
    let mut child_count: Vec<usize> = vec![0; n];
    for entry in &entries {
        for parent_id in &entry.parent_commit_ids {
            if let Some(&parent_idx) = id_to_idx.get(parent_id) {
                child_count[parent_idx] += 1;
            }
        }
    }

    // Convert to Option<LogEntry> for consumption
    let mut entries_opt: Vec<Option<LogEntry>> = entries.into_iter().map(Some).collect();

    // Initialize priority queue with heads (commits with no children in slice)
    // Use BinaryHeap for max-heap behavior
    let mut heap: BinaryHeap<(SortPriority, usize)> = BinaryHeap::new();
    for (idx, &count) in child_count.iter().enumerate() {
        if count == 0 {
            let priority = SortPriority {
                is_working_copy: is_working_copy[idx],
                neg_original_idx: -(idx as isize),
            };
            heap.push((priority, idx));
        }
    }

    // Kahn's algorithm with priority
    let mut result: Vec<LogEntry> = Vec::with_capacity(n);
    let mut emitted: HashSet<usize> = HashSet::new();

    while let Some((_, idx)) = heap.pop() {
        if emitted.contains(&idx) {
            continue;
        }
        emitted.insert(idx);

        let entry = entries_opt[idx].take().unwrap();

        // Decrement child count for parents and add newly eligible ones
        for parent_id in &entry.parent_commit_ids {
            if let Some(&parent_idx) = id_to_idx.get(parent_id)
                && child_count[parent_idx] > 0
            {
                child_count[parent_idx] -= 1;
                if child_count[parent_idx] == 0 && !emitted.contains(&parent_idx) {
                    let priority = SortPriority {
                        is_working_copy: is_working_copy[parent_idx],
                        neg_original_idx: -(parent_idx as isize),
                    };
                    heap.push((priority, parent_idx));
                }
            }
        }

        result.push(entry);
    }

    // If any entries weren't emitted (shouldn't happen with valid DAG), append them
    for entry in entries_opt.into_iter().flatten() {
        result.push(entry);
    }

    result
}

/// Parse a single log line.
fn parse_log_line(line: &str) -> Option<LogEntry> {
    let parts: Vec<&str> = line.split('\x00').collect();
    // Fields:
    // change_prefix, change_rest, commit_prefix, commit_rest, commit_full, parents,
    // author, timestamp, description, working_copy, immutable, empty, bookmarks
    if parts.len() < 13 {
        return None;
    }

    let bookmarks = super::parse_bookmarks_field(parts[12]);

    let change_id_prefix = parts[0].to_string();
    let change_id_rest = parts[1].to_string();
    let commit_id_prefix = parts[2].to_string();
    let commit_id_rest = parts[3].to_string();

    let commit_id_full = parts[4].to_string();
    let parent_commit_ids = if parts[5].is_empty() {
        Vec::new()
    } else {
        parts[5]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    };

    Some(LogEntry {
        change_id: format!("{change_id_prefix}{change_id_rest}"),
        change_id_prefix,
        change_id_rest,
        commit_id: format!("{commit_id_prefix}{commit_id_rest}"),
        commit_id_prefix,
        commit_id_rest,
        commit_id_full,
        parent_commit_ids,
        author: parts[6].to_string(),
        timestamp: parts[7].to_string(),
        description: parts[8].to_string(),
        is_working_copy: parts[9] == "true",
        is_immutable: parts[10] == "true",
        is_empty: parts[11] == "true",
        bookmarks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        // Fields:
        // change_prefix\0change_rest\0commit_prefix\0commit_rest\0commit_full\0parents\0author\0timestamp\0description\0working_copy\0immutable\0empty\0bookmarks
        let line = "abc\x00123\x00def\x00456\x00def456FULL\x00p1FULL,p2FULL\x00Alice\x002 hours ago\x00Add feature\x00true\x00false\x00false\x00main,dev";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.change_id_prefix, "abc");
        assert_eq!(entry.change_id_rest, "123");
        assert_eq!(entry.change_id, "abc123");
        assert_eq!(entry.commit_id_prefix, "def");
        assert_eq!(entry.commit_id_rest, "456");
        assert_eq!(entry.commit_id, "def456");
        assert_eq!(entry.commit_id_full, "def456FULL");
        assert_eq!(entry.parent_commit_ids, vec!["p1FULL", "p2FULL"]);
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
        let line = "abc\x00123\x00def\x00456\x00def456FULL\x00\x00Alice\x002 hours ago\x00Add feature\x00false\x00true\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert!(entry.bookmarks.is_empty());
        assert!(entry.parent_commit_ids.is_empty());
        assert!(!entry.is_working_copy);
        assert!(entry.is_immutable);
    }

    #[test]
    fn test_parse_log_line_empty_rest() {
        // When the entire ID is the unique prefix, rest is empty
        let line = "abcd\x00\x00defg\x00\x00defgFULL\x00\x00Alice\x00now\x00Test\x00false\x00false\x00false\x00";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.change_id_prefix, "abcd");
        assert!(entry.change_id_rest.is_empty());
        assert_eq!(entry.change_id, "abcd");
        assert_eq!(entry.commit_id_prefix, "defg");
        assert!(entry.commit_id_rest.is_empty());
        assert_eq!(entry.commit_id_full, "defgFULL");
        assert!(entry.parent_commit_ids.is_empty());
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
            commit_id_full: "def456FULL".to_string(),
            parent_commit_ids: vec![],
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
        let output = "abc\x00123\x00def\x00456\x00def456FULL\x00p0\x00Alice\x00now\x00First\x00true\x00false\x00false\x00\nghi\x00789\x00jkl\x00012\x00jkl012FULL\x00\x00Bob\x001h ago\x00Second\x00false\x00false\x00false\x00main\n";
        let entries = parse_log_output(output);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].change_id_prefix, "abc");
        assert_eq!(entries[0].change_id_rest, "123");
        assert_eq!(entries[0].change_id, "abc123");
        assert_eq!(entries[0].commit_id_full, "def456FULL");
        assert_eq!(entries[0].parent_commit_ids, vec!["p0"]);
        assert_eq!(entries[1].change_id, "ghi789");
        assert_eq!(entries[1].bookmarks, vec!["main"]);
    }

    /// Helper to create a test LogEntry with minimal fields.
    fn make_entry(id: &str, parents: &[&str], is_working_copy: bool) -> LogEntry {
        LogEntry {
            change_id: id.to_string(),
            change_id_prefix: id.to_string(),
            change_id_rest: String::new(),
            commit_id: id.to_string(),
            commit_id_prefix: id.to_string(),
            commit_id_rest: String::new(),
            commit_id_full: id.to_string(),
            parent_commit_ids: parents.iter().map(|s| s.to_string()).collect(),
            author: String::new(),
            timestamp: String::new(),
            description: format!("desc_{id}"),
            is_working_copy,
            is_immutable: false,
            is_empty: false,
            bookmarks: vec![],
        }
    }

    #[test]
    fn test_reorder_working_copy_first() {
        // Simulate jj log --no-graph returning wrong order:
        // p (merge, main) comes before y (@, working copy)
        // Correct order should have y first.
        let p = make_entry("p", &["mpkx", "o"], false); // merge commit
        let y = make_entry("y", &["o"], true); // working copy
        let o = make_entry("o", &["mpkx"], false);
        let mpkx = make_entry("mpkx", &[], false);

        // Wrong input order (as jj might return)
        let entries = vec![p, y, o, mpkx];
        let reordered = reorder_entries_for_graph(entries);

        // y (@) should be first
        assert!(reordered[0].is_working_copy, "working copy should be first");
        assert_eq!(reordered[0].commit_id_full, "y");
    }

    #[test]
    fn test_reorder_preserves_topo_order() {
        // Linear chain: A -> B -> C -> D
        // Even if input is [B, A, D, C], output should be topologically valid
        let a = make_entry("A", &["B"], false);
        let b = make_entry("B", &["C"], false);
        let c = make_entry("C", &["D"], false);
        let d = make_entry("D", &[], false);

        // Scrambled input
        let entries = vec![b, a, d, c];
        let reordered = reorder_entries_for_graph(entries);

        // Should produce A, B, C, D (children before parents)
        let ids: Vec<&str> = reordered
            .iter()
            .map(|e| e.commit_id_full.as_str())
            .collect();
        assert_eq!(ids, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_reorder_empty_and_single() {
        // Edge cases
        let empty: Vec<LogEntry> = vec![];
        assert!(reorder_entries_for_graph(empty).is_empty());

        let single = vec![make_entry("X", &[], false)];
        let result = reorder_entries_for_graph(single);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].commit_id_full, "X");
    }

    #[test]
    fn test_reorder_sibling_heads() {
        // Two sibling heads: y(@) and p, both have parent o
        // Even if p comes first in input, y should be first in output
        let p = make_entry("p", &["o"], false);
        let y = make_entry("y", &["o"], true); // working copy
        let o = make_entry("o", &[], false);

        let entries = vec![p, y, o];
        let reordered = reorder_entries_for_graph(entries);

        // y should be first (working copy priority)
        assert_eq!(reordered[0].commit_id_full, "y");
        assert!(reordered[0].is_working_copy);
        // p should be second
        assert_eq!(reordered[1].commit_id_full, "p");
        // o should be last (parent)
        assert_eq!(reordered[2].commit_id_full, "o");
    }
}
