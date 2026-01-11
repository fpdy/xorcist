//! jj repository detection.

use std::path::{Path, PathBuf};

/// Information about a jj repository.
#[derive(Debug, Clone)]
pub struct JjRepo {
    /// Root directory of the repository (contains .jj).
    pub root: PathBuf,
    /// Whether this is a colocated repository (has both .jj and .git).
    #[allow(dead_code)] // Will be used for future features
    pub colocated: bool,
}

/// Find a jj repository by walking up from the given directory.
///
/// Returns `None` if no `.jj` directory is found.
pub fn find_jj_repo(start: &Path) -> Option<JjRepo> {
    let mut current = start.to_path_buf();

    loop {
        let jj_dir = current.join(".jj");
        if jj_dir.is_dir() {
            let git_dir = current.join(".git");
            let colocated = git_dir.exists();
            return Some(JjRepo {
                root: current,
                colocated,
            });
        }

        if !current.pop() {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_repo_not_found() {
        let temp = TempDir::new().unwrap();
        let result = find_jj_repo(temp.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_find_repo_in_root() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".jj")).unwrap();

        let result = find_jj_repo(temp.path());
        assert!(result.is_some());
        let repo = result.unwrap();
        assert_eq!(repo.root, temp.path());
        assert!(!repo.colocated);
    }

    #[test]
    fn test_find_repo_in_subdirectory() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".jj")).unwrap();
        let subdir = temp.path().join("src").join("lib");
        std::fs::create_dir_all(&subdir).unwrap();

        let result = find_jj_repo(&subdir);
        assert!(result.is_some());
        let repo = result.unwrap();
        assert_eq!(repo.root, temp.path());
    }

    #[test]
    fn test_find_colocated_repo() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".jj")).unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();

        let result = find_jj_repo(temp.path());
        assert!(result.is_some());
        let repo = result.unwrap();
        assert!(repo.colocated);
    }
}
