//! Commit DAG graph layout for log view.
//!
//! This module constructs a single-line-per-commit graph column (Unicode line drawing)
//! from parent commit IDs. The intent is to make branch/merge structure visually
//! discoverable in the TUI log list.

use std::collections::HashMap;

use crate::jj::LogEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    /// A lane (graph line) cell.
    Lane { lane: usize },
    /// A node cell (commit symbol: @/◆/○).
    Node {
        is_working_copy: bool,
        is_immutable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell2 {
    pub left: char,
    pub right: char,
    pub kind_left: CellKind,
    pub kind_right: CellKind,
}

impl Cell2 {
    fn lane(lane: usize, left: char, right: char) -> Self {
        Self {
            left,
            right,
            kind_left: CellKind::Lane { lane },
            kind_right: CellKind::Lane { lane },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRow {
    /// Graph cells. Each lane is rendered as a fixed-width 2-character cell.
    pub cells: Vec<Cell2>,
    /// Which lane contains the node symbol for this row.
    pub node_lane: usize,
    /// Number of lanes that are still "active" after processing this row.
    /// This is used to determine which lanes should draw vertical continuations.
    pub active_lane_count: usize,
}

/// Build graph rows (one per log entry).
///
/// The returned Vec is guaranteed to have the same length as `entries`.
pub fn build_graph_rows(entries: &[LogEntry]) -> Vec<GraphRow> {
    let mut idx_of: HashMap<&str, usize> = HashMap::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        idx_of.insert(e.commit_id_full.as_str(), i);
    }

    let mut active_lanes: Vec<String> = Vec::new();
    let mut rows: Vec<GraphRow> = Vec::with_capacity(entries.len());

    for entry in entries {
        let cid = entry.commit_id_full.as_str();

        // Ensure current commit exists in active lanes.
        let node_lane = if let Some(pos) = active_lanes.iter().position(|x| x == cid) {
            pos
        } else {
            active_lanes.insert(0, cid.to_string());
            0
        };

        // Lane count after node insertion but before parent updates.
        // This determines vertical line continuations for this row.
        let active_lane_count_for_render = active_lanes.len();

        // Duplicates (to the right) indicate a convergence.
        let dup_lanes_pre_insert: Vec<usize> = active_lanes
            .iter()
            .enumerate()
            .filter_map(|(i, x)| (i > node_lane && x == cid).then_some(i))
            .collect();

        // Parents: only use parents that exist in the current log slice.
        // Keep jj's parent order: first parent = main line (left lane).
        let parents: Vec<&str> = entry
            .parent_commit_ids
            .iter()
            .map(|s| s.as_str())
            .filter(|p| idx_of.contains_key(p))
            .collect();

        let primary_parent = parents.first().copied();
        let other_parents: Vec<&str> = parents.into_iter().skip(1).collect();
        let split_count = if primary_parent.is_some() {
            other_parents.len()
        } else {
            0
        };

        // Update lanes.
        if let Some(primary) = primary_parent {
            active_lanes[node_lane] = primary.to_string();
            for (k, p) in other_parents.iter().enumerate() {
                active_lanes.insert(node_lane + 1 + k, (*p).to_string());
            }
        } else {
            // Root commit or missing parents: terminate this lane.
            // (We keep convergence rendering minimal here.)
            if node_lane < active_lanes.len() {
                active_lanes.remove(node_lane);
            }
        }

        // Convergence endpoints: shift indices to account for inserts to the right of node_lane.
        // Note: convergence happens regardless of whether the node has parents.
        let mut converge_endpoints: Vec<usize> = dup_lanes_pre_insert
            .into_iter()
            .map(|i| i + split_count)
            .collect();
        converge_endpoints.sort_unstable();

        // Remove converged duplicates from the active lanes (right-to-left).
        for &idx in converge_endpoints.iter().rev() {
            if idx < active_lanes.len() {
                active_lanes.remove(idx);
            }
        }

        let active_lane_count = active_lanes.len();

        // Split endpoints are the newly inserted lanes (right of the node lane).
        let split_endpoints: Vec<usize> = if split_count == 0 {
            Vec::new()
        } else {
            (node_lane + 1..=node_lane + split_count).collect()
        };

        let mut lane_count = active_lane_count.max(node_lane + 1);
        if let Some(m) = split_endpoints.iter().max() {
            lane_count = lane_count.max(m + 1);
        }
        if let Some(m) = converge_endpoints.iter().max() {
            lane_count = lane_count.max(m + 1);
        }

        // Initialize lanes: use lane count after node insertion for vertical continuations.
        // This ensures lanes that exist at this row get vertical lines.
        let mut cells: Vec<Cell2> = (0..lane_count)
            .map(|lane| {
                let left = if lane < active_lane_count_for_render {
                    '│'
                } else {
                    ' '
                };
                Cell2::lane(lane, left, ' ')
            })
            .collect();

        // Node symbol (keep xorcist/jj symbols).
        let node_char = entry.graph_symbol().chars().next().unwrap_or('○');
        if node_lane < cells.len() {
            cells[node_lane].left = node_char;
            cells[node_lane].kind_left = CellKind::Node {
                is_working_copy: entry.is_working_copy,
                is_immutable: entry.is_immutable,
            };
        }

        // Draw horizontal connections to the right (splits and convergences).
        let rightmost = split_endpoints
            .iter()
            .chain(converge_endpoints.iter())
            .copied()
            .max();
        if let Some(target) = rightmost
            && target > node_lane
            && node_lane < cells.len()
        {
            // Node cell uses the right-half for the horizontal line.
            cells[node_lane].right = '─';

            for lane in (node_lane + 1)..=target {
                if lane >= cells.len() {
                    break;
                }

                // Cross if this lane has a vertical line (was active at render time).
                cells[lane].left = if lane < active_lane_count_for_render {
                    '┼'
                } else {
                    '─'
                };

                // Keep drawing the horizontal line until the rightmost target.
                cells[lane].right = if lane == target { ' ' } else { '─' };
            }

            // Endpoints override the left char.
            for lane in split_endpoints {
                if lane < cells.len() {
                    cells[lane].left = '┐';
                }
            }
            for lane in converge_endpoints {
                if lane < cells.len() {
                    cells[lane].left = '┘';
                }
            }
        }

        rows.push(GraphRow {
            cells,
            node_lane,
            active_lane_count,
        });
    }

    rows
}

#[cfg(test)]
pub(crate) fn render_graph_row_plain(row: &GraphRow) -> String {
    let mut s = String::new();
    for c in &row.cells {
        s.push(c.left);
        s.push(c.right);
    }
    s.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str, parents: &[&str]) -> LogEntry {
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
            description: String::new(),
            is_working_copy: false,
            is_immutable: false,
            is_empty: false,
            bookmarks: vec![],
        }
    }

    fn rows_plain(entries: &[LogEntry]) -> Vec<String> {
        build_graph_rows(entries)
            .iter()
            .map(render_graph_row_plain)
            .collect()
    }

    #[test]
    fn graph_linear() {
        let entries = vec![e("A", &["B"]), e("B", &["C"]), e("C", &[])];
        let rows = rows_plain(&entries);
        assert_eq!(rows, vec!["○", "○", "○"]);
    }

    #[test]
    fn graph_branch_and_converge() {
        let entries = vec![e("A", &["B"]), e("C", &["B"]), e("B", &["D"]), e("D", &[])];
        let rows = rows_plain(&entries);
        assert_eq!(rows, vec!["○", "○ │", "○─┘", "○"]);
    }

    #[test]
    fn graph_merge_and_converge() {
        let entries = vec![
            e("M", &["P1", "P2"]),
            e("P1", &["R"]),
            e("P2", &["R"]),
            e("R", &["T"]),
            e("T", &[]),
        ];
        let rows = rows_plain(&entries);
        assert_eq!(rows, vec!["○─┐", "○ │", "│ ○", "○─┘", "○"]);
    }

    #[test]
    fn graph_crossing_converge() {
        // Force active lanes to become [B, X, B] before rendering B.
        let entries = vec![
            e("A", &["B"]),
            e("D", &["X"]),
            e("C", &["B"]),
            e("B", &["R"]),
            e("X", &[]),
            e("R", &[]),
        ];
        let rows = rows_plain(&entries);

        assert_eq!(rows[0], "○");
        assert_eq!(rows[1], "○ │");
        assert_eq!(rows[2], "○ │ │");
        assert_eq!(rows[3], "○─┼─┘");
    }

    #[test]
    fn graph_merge_with_child() {
        // Scenario from actual xorcist bug:
        // A(@) -> M(merge) -> [P1, P2]
        // P1 = main line (first parent, should be left lane)
        // P2 = feat branch (second parent, should be right lane)
        // Both P1 and P2 -> R (common ancestor)
        //
        // Expected jj-style graph:
        // A:  ○      (simple, no branch lines)
        // M:  ○─┐    (merge: branch to right for P2)
        // P1: ○ │    (P1 on left, P2 lane continues on right)
        // P2: │ ○    (P2 node on right lane)
        // R:  ○─┘    (convergence from right)
        let entries = vec![
            e("A", &["M"]),
            e("M", &["P1", "P2"]), // P1 first = main line
            e("P1", &["R"]),
            e("P2", &["R"]),
            e("R", &[]),
        ];
        let rows = rows_plain(&entries);

        assert_eq!(rows[0], "○", "A: working copy, single lane");
        assert_eq!(rows[1], "○─┐", "M: merge, split to right");
        assert_eq!(rows[2], "○ │", "P1: main line on left");
        assert_eq!(rows[3], "│ ○", "P2: feat branch on right");
        assert_eq!(rows[4], "○─┘", "R: convergence");
    }

    #[test]
    fn graph_sibling_heads_wrong_order() {
        // This test reproduces the actual xorcist issue:
        // Real DAG:
        //   y(@) -> o (working copy, parent is o)
        //   p(main) -> [mpkx, o] (merge commit, parents are mpkx and o)
        //   o -> mpkx
        //
        // jj log --no-graph returns: [p, y, o, mpkx, ...]
        // But correct display order should be: [y(@), p, o, mpkx, ...]
        //
        // When input order is wrong (p before y), graph becomes broken.
        let mut p = e("p", &["mpkx", "o"]); // merge commit
        p.bookmarks = vec!["main".to_string()];

        let mut y = e("y", &["o"]);
        y.is_working_copy = true;

        let o = e("o", &["mpkx"]);
        let mpkx = e("mpkx", &[]);

        // WRONG order (as jj log --no-graph might return):
        let entries_wrong = vec![p.clone(), y.clone(), o.clone(), mpkx.clone()];
        let rows_wrong = rows_plain(&entries_wrong);

        // With wrong order, p comes first and takes lane 0,
        // then y(@) appears and gets pushed to a new lane - broken display
        // This documents the current broken behavior
        assert_eq!(
            rows_wrong[0], "○─┐",
            "p: merge at lane 0 (wrong - should be @)"
        );
        // y(@) should be at top but isn't

        // CORRECT order (@ first):
        let entries_correct = vec![y, p, o, mpkx];
        let rows_correct = rows_plain(&entries_correct);

        // With correct order, y(@) is first
        assert_eq!(rows_correct[0], "@", "y(@): working copy at top");
        assert_eq!(
            rows_correct[1], "○─┐",
            "p: merge, splits to right for second parent o"
        );
        // Note: p's parents are [mpkx, o], so main line goes to mpkx (left), o goes right
        // But y's parent is also o, so o is already in active lanes from y
    }
}
