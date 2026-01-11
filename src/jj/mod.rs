//! jj VCS integration module.

pub mod log;
pub mod repo;
pub mod runner;

pub use log::{LogEntry, fetch_log};
pub use repo::find_jj_repo;
pub use runner::JjRunner;
