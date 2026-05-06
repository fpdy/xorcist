//! Lazy loading methods for App.

use std::sync::mpsc;
use std::thread;

use crate::error::XorcistError;
use crate::jj::fetch_graph_log;

use super::{App, DEFAULT_BATCH_SIZE, JobKind, JobResult, LOAD_MORE_THRESHOLD};

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

    /// Start loading more entries in the background.
    ///
    /// Instead of appending a partial jj graph, increase the log limit and
    /// re-fetch the graph as a whole so graph topology remains consistent.
    pub fn load_more_entries(&mut self) -> Result<bool, XorcistError> {
        let current_limit = self.log_limit.unwrap_or(DEFAULT_BATCH_SIZE);
        let requested_limit = current_limit.saturating_add(DEFAULT_BATCH_SIZE);
        let previous_selection = self.selected_change_id().map(str::to_string);
        let runner = self.runner();

        let (tx, rx) = mpsc::channel();
        if !self.start_job(JobKind::LoadMore, rx) {
            self.is_loading_more = false;
            return Ok(false);
        }

        thread::spawn(move || {
            let _ = tx.send(JobResult::LoadMore {
                previous_selection,
                requested_limit,
                result: fetch_graph_log(&*runner, Some(requested_limit)),
            });
        });
        Ok(true)
    }
}
