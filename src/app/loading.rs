//! Lazy loading methods for App.

use crate::error::XorcistError;
use crate::jj::fetch_graph_log_after;

use super::{App, DEFAULT_BATCH_SIZE, LOAD_MORE_THRESHOLD};

impl App {
    /// Set the log entry limit and determine if more entries might be available.
    pub fn set_log_limit(&mut self, limit: Option<usize>) {
        self.log_limit = limit;
        // If no limit (--all), we have all entries
        // Otherwise, assume more entries exist if we loaded exactly the limit
        self.has_more_entries = match limit {
            None => false,
            Some(n) => self.graph_log.commit_count() >= n,
        };
    }

    /// Request a check for loading more entries.
    /// This sets a flag that will be checked by the event loop.
    pub fn request_load_more_check(&mut self) {
        self.pending_load_more = true;
    }

    /// Check if we should load more entries.
    /// Returns true if load is needed and conditions are met.
    pub fn should_load_more(&self) -> bool {
        if !self.pending_load_more {
            return false;
        }
        // Skip if:
        // - No limit set (--all mode, already have everything)
        // - No more entries available
        // - Already loading
        // - Not near the end of the list
        if self.log_limit.is_none() || !self.has_more_entries || self.is_loading_more {
            return false;
        }

        let entries_from_end = self.commit_count().saturating_sub(self.selected);
        entries_from_end <= LOAD_MORE_THRESHOLD
    }

    /// Mark that we're starting to load more entries.
    pub fn start_loading(&mut self) {
        self.is_loading_more = true;
        self.pending_load_more = false;
    }

    /// Actually load more entries.
    /// Should be called after start_loading() and a redraw.
    pub fn load_more_entries(&mut self) -> Result<bool, XorcistError> {
        // Get the last commit's change_id to use as anchor
        let last_selection = self.commit_count().saturating_sub(1);
        let Some(after_change_id) = self.graph_log.change_id_for_selection(last_selection) else {
            self.is_loading_more = false;
            return Ok(false);
        };
        let after_change_id = after_change_id.to_string();

        // Fetch more entries
        let batch_size = self.log_limit.unwrap_or(DEFAULT_BATCH_SIZE);
        let additional = fetch_graph_log_after(&self.runner, &after_change_id, batch_size)?;

        self.is_loading_more = false;

        if additional.is_empty() || additional.commit_count() == 0 {
            self.has_more_entries = false;
            return Ok(false);
        }

        // If we got fewer than requested, we've reached the end
        if additional.commit_count() < batch_size {
            self.has_more_entries = false;
        }

        // Merge additional lines into existing graph_log
        self.graph_log.extend(additional);
        Ok(true)
    }
}
