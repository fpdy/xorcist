//! jj VCS integration module.

pub mod log;
pub mod repo;
pub mod runner;
pub mod show;

pub use log::{LogEntry, fetch_log, fetch_log_after};
pub use repo::find_jj_repo;
pub use runner::JjRunner;
pub use show::{DiffStatus, ShowOutput, fetch_show};

pub(crate) fn parse_bookmarks_field(field: &str) -> Vec<String> {
    if field.is_empty() {
        Vec::new()
    } else {
        field.split(',').map(String::from).collect()
    }
}
