//! jj VCS integration module.

pub mod graph_log;
pub mod repo;
pub mod runner;
pub mod show;

pub use graph_log::{GraphLog, fetch_graph_log};
pub use repo::find_jj_repo;
pub use runner::{JjBackend, JjRunner};
pub(crate) use show::parse_diff_summary;
pub use show::{DiffEntry, DiffStatus, ShowOutput, fetch_diff_file, fetch_show};

pub(crate) fn parse_bookmarks_field(field: &str) -> Vec<String> {
    if field.is_empty() {
        Vec::new()
    } else {
        field.split(',').map(String::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bookmarks_field;

    #[test]
    fn test_parse_bookmarks_field_empty() {
        assert!(parse_bookmarks_field("").is_empty());
    }

    #[test]
    fn test_parse_bookmarks_field_comma_separated() {
        assert_eq!(parse_bookmarks_field("main,dev"), vec!["main", "dev"]);
    }

    #[test]
    fn test_parse_bookmarks_field_preserves_literal_whitespace() {
        assert_eq!(
            parse_bookmarks_field("main, dev,topic "),
            vec!["main", " dev", "topic "]
        );
    }
}
