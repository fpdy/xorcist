//! jj VCS integration module.

pub mod graph_log;
pub mod repo;
pub mod runner;
pub mod show;

pub use graph_log::{GraphLog, fetch_graph_log, fetch_graph_log_after};
pub use repo::find_jj_repo;
pub use runner::JjRunner;
pub(crate) use show::parse_diff_summary;
pub use show::{DiffEntry, DiffStatus, ShowOutput, fetch_diff_file, fetch_show};

pub(crate) fn parse_bookmarks_field(field: &str) -> Vec<String> {
    if field.is_empty() {
        Vec::new()
    } else {
        field.split(',').map(String::from).collect()
    }
}
