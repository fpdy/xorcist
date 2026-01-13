//! jj VCS integration module.

pub mod graph_log;
pub mod repo;
pub mod runner;
pub mod show;

pub use graph_log::{GraphLog, fetch_graph_log, fetch_graph_log_after};
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
