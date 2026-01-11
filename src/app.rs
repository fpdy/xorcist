//! Application state management.

use crate::jj::LogEntry;

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
}

impl App {
    /// Create a new App with the given log entries.
    pub fn new(entries: Vec<LogEntry>, repo_root: String) -> Self {
        Self {
            entries,
            selected: 0,
            should_quit: false,
            repo_root,
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_navigation() {
        let entries = vec![make_entry("1"), make_entry("2"), make_entry("3")];
        let mut app = App::new(entries, "/repo".to_string());

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
        let mut app = App::new(entries, "/repo".to_string());

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
        let mut app = App::new(vec![], "/repo".to_string());

        // Should not panic on empty list
        app.select_next();
        app.select_previous();
        app.select_first();
        app.select_last();
        app.page_down(5);
        app.page_up(5);

        assert_eq!(app.selected, 0);
    }
}
