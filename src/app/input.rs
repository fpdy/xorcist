//! Input mode methods for App.

use crate::error::XorcistError;

use super::{App, InputMode};

impl App {
    /// Start input mode for text entry.
    pub fn start_input_mode(&mut self, mode: InputMode) {
        self.input_mode = Some(mode);
        self.input.reset();
    }

    /// Cancel input mode without executing.
    pub fn cancel_input_mode(&mut self) {
        self.input_mode = None;
        self.input.reset();
    }

    /// Check if currently in input mode.
    pub fn is_input_mode(&self) -> bool {
        self.input_mode.is_some()
    }

    /// Submit the current input and execute the corresponding command.
    pub fn submit_input(&mut self) -> Result<(), XorcistError> {
        let Some(mode) = self.input_mode.take() else {
            return Ok(());
        };
        let value = self.input.value().to_string();
        self.input.reset();

        match mode {
            InputMode::Describe => self.execute_describe(&value)?,
            InputMode::BookmarkSet => self.execute_bookmark_set(&value)?,
            InputMode::NewWithMessage => self.execute_new_with_message(&value)?,
            InputMode::RebaseDestination => self.execute_rebase(&value)?,
        }
        Ok(())
    }
}
