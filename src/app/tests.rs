//! Tests for App.

use super::*;
use crate::jj::GraphLog;
use std::path::Path;

fn make_graph_log(count: usize) -> GraphLog {
    // Create a simple graph log with N commits
    // change_id must be 8 lowercase letters to be parsed correctly
    // Use letter-only IDs (a-z) since the regex requires [a-z]{8}
    let mut output = String::new();
    for i in 0..count {
        // Generate 8-char lowercase letter ID using base-26 encoding
        let id = index_to_change_id(i);
        output.push_str(&format!("@  {id} Author {i}h Entry {i}\n"));
    }
    GraphLog::from_output(&output)
}

fn index_to_change_id(i: usize) -> String {
    // Generate an 8-character ID using only lowercase letters a-z
    let mut id = String::with_capacity(8);
    let mut n = i;
    for _ in 0..8 {
        let ch = (b'a' + (n % 26) as u8) as char;
        id.insert(0, ch);
        n /= 26;
    }
    id
}

fn expected_change_id(i: usize) -> String {
    index_to_change_id(i)
}

fn make_runner() -> JjRunner {
    JjRunner::new().with_work_dir(Path::new("/tmp"))
}

#[test]
fn test_navigation() {
    let graph_log = make_graph_log(3);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());

    assert_eq!(app.selected, 0);
    assert_eq!(app.commit_count(), 3);

    app.select_next();
    assert_eq!(app.selected, 1);

    app.select_next();
    assert_eq!(app.selected, 2);

    // Should not go past the end
    app.select_next();
    assert_eq!(app.selected, 2);

    app.select_previous();
    assert_eq!(app.selected, 1);

    app.select_first();
    assert_eq!(app.selected, 0);

    app.select_last();
    assert_eq!(app.selected, 2);
}

#[test]
fn test_page_navigation() {
    let count = 20;
    let graph_log = make_graph_log(count);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());

    assert_eq!(app.commit_count(), count);

    app.page_down(5);
    assert_eq!(app.selected, 5);

    app.page_down(5);
    assert_eq!(app.selected, 10);

    app.page_up(3);
    assert_eq!(app.selected, 7);

    // Page down past the end
    app.page_down(100);
    assert_eq!(app.selected, count - 1);

    // Page up past the beginning
    app.page_up(100);
    assert_eq!(app.selected, 0);
}

#[test]
fn test_empty_entries() {
    let graph_log = GraphLog::default();
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());

    // Should not panic on empty list
    app.select_next();
    app.select_previous();
    app.select_first();
    app.select_last();
    app.page_down(5);
    app.page_up(5);

    assert_eq!(app.selected, 0);
}

#[test]
fn test_view_transitions() {
    let graph_log = make_graph_log(1);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());

    assert_eq!(app.view, View::Log);
    assert!(app.detail_state.is_none());

    // Note: open_detail would fail without a real jj repo
    // We just test close_detail here
    app.view = View::Detail;
    app.detail_state = Some(DetailState {
        show_output: ShowOutput {
            change_id: "abc123".to_string(),
            change_id_prefix: "abc".to_string(),
            change_id_rest: "123".to_string(),
            commit_id: "def456".to_string(),
            commit_id_prefix: "def".to_string(),
            commit_id_rest: "456".to_string(),
            author: "Test".to_string(),
            timestamp: "now".to_string(),
            description: "Test".to_string(),
            bookmarks: vec![],
            diff_summary: vec![],
        },
        scroll: 5,
        content_height: 20,
    });

    app.close_detail();
    assert_eq!(app.view, View::Log);
    assert!(app.detail_state.is_none());
}

#[test]
fn test_detail_scroll() {
    let graph_log = GraphLog::default();
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.detail_state = Some(DetailState {
        show_output: ShowOutput {
            change_id: "abc123".to_string(),
            change_id_prefix: "abc".to_string(),
            change_id_rest: "123".to_string(),
            commit_id: "def456".to_string(),
            commit_id_prefix: "def".to_string(),
            commit_id_rest: "456".to_string(),
            author: "Test".to_string(),
            timestamp: "now".to_string(),
            description: "Test".to_string(),
            bookmarks: vec![],
            diff_summary: vec![],
        },
        scroll: 5,
        content_height: 20,
    });

    app.detail_scroll_down(3);
    assert_eq!(app.detail_state.as_ref().unwrap().scroll, 8);

    app.detail_scroll_up(2);
    assert_eq!(app.detail_state.as_ref().unwrap().scroll, 6);

    // Scroll up past beginning
    app.detail_scroll_up(100);
    assert_eq!(app.detail_state.as_ref().unwrap().scroll, 0);
}

#[test]
fn test_set_detail_content_height() {
    let graph_log = GraphLog::default();
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.detail_state = Some(DetailState {
        show_output: ShowOutput {
            change_id: "abc123".to_string(),
            change_id_prefix: "abc".to_string(),
            change_id_rest: "123".to_string(),
            commit_id: "def456".to_string(),
            commit_id_prefix: "def".to_string(),
            commit_id_rest: "456".to_string(),
            author: "Test".to_string(),
            timestamp: "now".to_string(),
            description: "Test".to_string(),
            bookmarks: vec![],
            diff_summary: vec![],
        },
        scroll: 50,
        content_height: 0,
    });

    // Setting height should clamp scroll
    app.set_detail_content_height(20);
    assert_eq!(app.detail_state.as_ref().unwrap().content_height, 20);
    assert_eq!(app.detail_state.as_ref().unwrap().scroll, 19);
}

#[test]
fn test_should_load_more_not_pending() {
    let graph_log = make_graph_log(100);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.set_log_limit(Some(100));

    // No pending request
    assert!(!app.should_load_more());
}

#[test]
fn test_should_load_more_near_end() {
    let count = 100;
    let graph_log = make_graph_log(count);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.set_log_limit(Some(count));

    // Verify we have the expected number of commits
    assert_eq!(app.commit_count(), count);
    assert!(app.has_more_entries);

    // Move near the end and request load
    app.selected = 95; // 5 from end, within LOAD_MORE_THRESHOLD (50)
    app.request_load_more_check();

    assert!(app.should_load_more());
}

#[test]
fn test_should_load_more_not_near_end() {
    let graph_log = make_graph_log(100);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.set_log_limit(Some(100));

    // Stay at the beginning
    app.selected = 10; // 90 from end, outside LOAD_MORE_THRESHOLD
    app.request_load_more_check();

    assert!(!app.should_load_more());
}

#[test]
fn test_should_load_more_all_mode() {
    let graph_log = make_graph_log(100);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.set_log_limit(None); // --all mode

    app.selected = 95;
    app.request_load_more_check();

    // Should not load in --all mode
    assert!(!app.should_load_more());
}

#[test]
fn test_should_load_more_no_more_entries() {
    let graph_log = make_graph_log(50);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.set_log_limit(Some(100));

    // Fewer entries than limit means no more available
    assert!(!app.has_more_entries);

    app.selected = 45;
    app.request_load_more_check();

    assert!(!app.should_load_more());
}

#[test]
fn test_start_loading_clears_pending() {
    let count = 100;
    let graph_log = make_graph_log(count);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());
    app.set_log_limit(Some(count));

    assert_eq!(app.commit_count(), count);
    assert!(app.has_more_entries);

    app.selected = 95;
    app.request_load_more_check();
    assert!(app.should_load_more());

    app.start_loading();
    assert!(app.is_loading_more);
    assert!(!app.should_load_more()); // pending cleared, is_loading_more blocks
}

#[test]
fn test_selected_change_id() {
    let graph_log = make_graph_log(3);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());

    assert_eq!(
        app.selected_change_id(),
        Some(expected_change_id(0).as_str())
    );

    app.select_next();
    assert_eq!(
        app.selected_change_id(),
        Some(expected_change_id(1).as_str())
    );

    app.select_next();
    assert_eq!(
        app.selected_change_id(),
        Some(expected_change_id(2).as_str())
    );
}

#[test]
fn test_ensure_selected_visible() {
    let graph_log = make_graph_log(20);
    let mut app = App::new(graph_log, "/repo".to_string(), make_runner());

    // Initial state
    assert_eq!(app.scroll_offset, 0);

    // Select item beyond viewport
    app.selected = 15;
    app.ensure_selected_visible(10);

    // Get the actual line index for the selected commit
    let line_idx = app.selected_line_index().unwrap();
    // Selected line should be visible in viewport of 10
    assert!(
        app.scroll_offset <= line_idx,
        "scroll_offset {} should be <= line_idx {}",
        app.scroll_offset,
        line_idx
    );
    assert!(
        app.scroll_offset + 10 > line_idx,
        "scroll_offset {} + 10 should be > line_idx {}",
        app.scroll_offset,
        line_idx
    );
}
