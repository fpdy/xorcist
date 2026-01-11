//! Application state management.

use crate::error::XorcistError;
use crate::jj::{JjRunner, LogEntry, ShowOutput, fetch_show};

/// Current view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Log,
    Detail,
}

/// State for detail view.
#[derive(Debug, Clone)]
pub struct DetailState {
    /// The ShowOutput being displayed.
    pub show_output: ShowOutput,
    /// Vertical scroll offset.
    pub scroll: usize,
    /// Total content height (for scroll calculation).
    pub content_height: usize,
}

/// Application state.
pub struct App {
    /// Log entries to display.
    pub entries: Vec<LogEntry>,
    /// Currently selected index.
    pub selected: usize,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Repository root path.
    pub repo_root: String,
    /// Current view mode.
    pub view: View,
    /// Detail view state.
    pub detail_state: Option<DetailState>,
    /// jj command runner.
    runner: JjRunner,
}

impl App {
    /// Create a new App with the given log entries.
    pub fn new(entries: Vec<LogEntry>, repo_root: String, runner: JjRunner) -> Self {
        Self {
            entries,
            selected: 0,
            should_quit: false,
            repo_root,
            view: View::default(),
            detail_state: None,
            runner,
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Jump to the first entry.
    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Jump to the last entry.
    pub fn select_last(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Page down (move by visible height).
    pub fn page_down(&mut self, page_size: usize) {
        if self.entries.is_empty() {
            return;
        }
        let new_selected = self.selected.saturating_add(page_size);
        self.selected = new_selected.min(self.entries.len() - 1);
    }

    /// Page up (move by visible height).
    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }

    /// Request application quit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Open detail view for selected entry.
    pub fn open_detail(&mut self) -> Result<(), XorcistError> {
        if let Some(entry) = self.entries.get(self.selected) {
            let show_output = fetch_show(&self.runner, &entry.change_id)?;
            self.detail_state = Some(DetailState {
                show_output,
                scroll: 0,
                content_height: 0, // Calculated during render
            });
            self.view = View::Detail;
        }
        Ok(())
    }

    /// Close detail view and return to log.
    pub fn close_detail(&mut self) {
        self.view = View::Log;
        self.detail_state = None;
    }

    /// Scroll detail view down.
    pub fn detail_scroll_down(&mut self, amount: usize) {
        if let Some(state) = &mut self.detail_state {
            state.scroll = state.scroll.saturating_add(amount);
        }
    }

    /// Scroll detail view up.
    pub fn detail_scroll_up(&mut self, amount: usize) {
        if let Some(state) = &mut self.detail_state {
            state.scroll = state.scroll.saturating_sub(amount);
        }
    }

    /// Set content height for detail view (called from render).
    pub fn set_detail_content_height(&mut self, height: usize) {
        if let Some(state) = &mut self.detail_state {
            state.content_height = height;
            // Clamp scroll to valid range
            if height > 0 && state.scroll >= height {
                state.scroll = height.saturating_sub(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn make_entry(id: &str) -> LogEntry {
        LogEntry {
            change_id: id.to_string(),
            commit_id: format!("commit_{id}"),
            author: "Test".to_string(),
            timestamp: "now".to_string(),
            description: format!("Entry {id}"),
            is_working_copy: false,
            is_immutable: false,
            is_empty: false,
            bookmarks: vec![],
        }
    }

    fn make_runner() -> JjRunner {
        JjRunner::new().with_work_dir(Path::new("/tmp"))
    }

    #[test]
    fn test_navigation() {
        let entries = vec![make_entry("1"), make_entry("2"), make_entry("3")];
        let mut app = App::new(entries, "/repo".to_string(), make_runner());

        assert_eq!(app.selected, 0);

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
        let entries: Vec<_> = (0..20).map(|i| make_entry(&i.to_string())).collect();
        let mut app = App::new(entries, "/repo".to_string(), make_runner());

        app.page_down(5);
        assert_eq!(app.selected, 5);

        app.page_down(5);
        assert_eq!(app.selected, 10);

        app.page_up(3);
        assert_eq!(app.selected, 7);

        // Page down past the end
        app.page_down(100);
        assert_eq!(app.selected, 19);

        // Page up past the beginning
        app.page_up(100);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_empty_entries() {
        let mut app = App::new(vec![], "/repo".to_string(), make_runner());

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
        let entries = vec![make_entry("1")];
        let mut app = App::new(entries, "/repo".to_string(), make_runner());

        assert_eq!(app.view, View::Log);
        assert!(app.detail_state.is_none());

        // Note: open_detail would fail without a real jj repo
        // We just test close_detail here
        app.view = View::Detail;
        app.detail_state = Some(DetailState {
            show_output: ShowOutput {
                change_id: "abc".to_string(),
                commit_id: "def".to_string(),
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
        let mut app = App::new(vec![], "/repo".to_string(), make_runner());
        app.detail_state = Some(DetailState {
            show_output: ShowOutput {
                change_id: "abc".to_string(),
                commit_id: "def".to_string(),
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
        let mut app = App::new(vec![], "/repo".to_string(), make_runner());
        app.detail_state = Some(DetailState {
            show_output: ShowOutput {
                change_id: "abc".to_string(),
                commit_id: "def".to_string(),
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
}
