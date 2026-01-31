//! Navigation methods for App.

use unicode_width::UnicodeWidthStr;

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

    // === Diff view navigation ===

    /// Select next file in diff view.
    pub fn diff_select_next(&mut self) {
        let count = self.diff_state.files.len();
        if count > 0 && self.diff_state.selected < count - 1 {
            self.diff_state.selected += 1;
        }
    }

    /// Select previous file in diff view.
    pub fn diff_select_previous(&mut self) {
        if self.diff_state.selected > 0 {
            self.diff_state.selected -= 1;
        }
    }

    /// Scroll diff text down.
    pub fn diff_scroll_down(&mut self, amount: usize) {
        self.diff_state.diff_scroll = self.diff_state.diff_scroll.saturating_add(amount);
    }

    /// Scroll diff text up.
    pub fn diff_scroll_up(&mut self, amount: usize) {
        self.diff_state.diff_scroll = self.diff_state.diff_scroll.saturating_sub(amount);
    }

    /// Clamp diff scroll to valid range.
    pub fn clamp_diff_scroll(&mut self, visible_height: usize) {
        let content_height = self.diff_state.diff_lines.len();
        let max_scroll = content_height.saturating_sub(visible_height);
        if self.diff_state.diff_scroll > max_scroll {
            self.diff_state.diff_scroll = max_scroll;
        }
    }

    /// Scroll diff text right (horizontal).
    pub fn diff_scroll_right(&mut self, amount: usize) {
        self.diff_state.diff_h_scroll = self.diff_state.diff_h_scroll.saturating_add(amount);
    }

    /// Scroll diff text left (horizontal).
    pub fn diff_scroll_left(&mut self, amount: usize) {
        self.diff_state.diff_h_scroll = self.diff_state.diff_h_scroll.saturating_sub(amount);
    }

    /// Clamp horizontal diff scroll to valid range.
    pub fn clamp_diff_h_scroll(&mut self, visible_width: usize) {
        // Use display width (unicode_width) instead of byte length for correct CJK handling
        let max_line_width = self
            .diff_state
            .diff_lines
            .iter()
            .map(|l| l.width())
            .max()
            .unwrap_or(0);
        let max_scroll = max_line_width.saturating_sub(visible_width);
        if self.diff_state.diff_h_scroll > max_scroll {
            self.diff_state.diff_h_scroll = max_scroll;
        }
    }

    /// Ensure selected file is visible in file list.
    pub fn ensure_diff_file_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        let selected = self.diff_state.selected;
        if selected < self.diff_state.file_scroll {
            self.diff_state.file_scroll = selected;
        } else if selected >= self.diff_state.file_scroll + visible_height {
            self.diff_state.file_scroll = selected.saturating_sub(visible_height - 1);
        }
    }
}
