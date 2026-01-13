//! Navigation methods for App.

use super::App;

impl App {
    /// Get the number of commits in the log.
    pub fn commit_count(&self) -> usize {
        self.graph_log.commit_count()
    }

    /// Get the total number of lines in the graph.
    pub fn line_count(&self) -> usize {
        self.graph_log.lines.len()
    }

    /// Get the line index for the currently selected commit.
    pub fn selected_line_index(&self) -> Option<usize> {
        self.graph_log.line_index_for_selection(self.selected)
    }

    /// Get the change_id for the currently selected commit.
    pub fn selected_change_id(&self) -> Option<&str> {
        self.graph_log.change_id_for_selection(self.selected)
    }

    /// Ensure the selected line is visible in the viewport.
    pub fn ensure_selected_visible(&mut self, viewport_height: usize) {
        if let Some(line_idx) = self.selected_line_index() {
            // If selected line is above viewport, scroll up
            if line_idx < self.scroll_offset {
                self.scroll_offset = line_idx;
            }
            // If selected line is below viewport, scroll down
            else if line_idx >= self.scroll_offset + viewport_height {
                self.scroll_offset = line_idx.saturating_sub(viewport_height - 1);
            }
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        let count = self.commit_count();
        if count > 0 && self.selected < count - 1 {
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
        let count = self.commit_count();
        if count > 0 {
            self.selected = count - 1;
        }
    }

    /// Page down (move by visible height).
    pub fn page_down(&mut self, page_size: usize) {
        let count = self.commit_count();
        if count == 0 {
            return;
        }
        let new_selected = self.selected.saturating_add(page_size);
        self.selected = new_selected.min(count - 1);
    }

    /// Page up (move by visible height).
    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }
}
