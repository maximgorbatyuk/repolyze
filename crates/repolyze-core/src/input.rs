use std::path::{Path, PathBuf};

use crate::error::RepolyzeError;
use crate::model::RepositoryTarget;

/// Resolves user-provided paths into validated `RepositoryTarget` values.
///
/// - Canonicalizes each path
/// - Walks upward to find `.git` directory if needed
/// - Deduplicates by canonical path
/// - Returns error for paths that don't exist or aren't Git repositories
pub fn resolve_inputs(paths: &[PathBuf]) -> Result<Vec<RepositoryTarget>, RepolyzeError> {
    let mut seen = std::collections::HashSet::new();
    let mut targets = Vec::new();

    for path in paths {
        let canonical = path
            .canonicalize()
            .map_err(|_| RepolyzeError::PathNotFound(path.clone()))?;

        let root = find_git_root(&canonical)?;

        if seen.insert(root.clone()) {
            targets.push(RepositoryTarget { root });
        }
    }

    targets.sort_by(|a, b| a.root.cmp(&b.root));
    Ok(targets)
}

fn find_git_root(path: &Path) -> Result<PathBuf, RepolyzeError> {
    let mut current = if path.is_file() {
        path.parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| RepolyzeError::NotAGitRepository(path.to_path_buf()))?
    } else {
        path.to_path_buf()
    };

    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return Err(RepolyzeError::NotAGitRepository(path.to_path_buf())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn create_temp_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn rejects_non_git_directories() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_inputs(&[dir.path().to_path_buf()]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RepolyzeError::NotAGitRepository(_)),
            "expected NotAGitRepository, got: {err}"
        );
    }

    #[test]
    fn accepts_valid_git_repository() {
        let dir = create_temp_git_repo();
        let result = resolve_inputs(&[dir.path().to_path_buf()]);
        assert!(result.is_ok());
        let targets = result.unwrap();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn deduplicates_equivalent_repository_paths() {
        let dir = create_temp_git_repo();
        let canonical = dir.path().canonicalize().unwrap();
        // Pass same path twice
        let result = resolve_inputs(&[dir.path().to_path_buf(), canonical]);
        assert!(result.is_ok());
        let targets = result.unwrap();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn resolves_subdirectory_to_repo_root() {
        let dir = create_temp_git_repo();
        let subdir = dir.path().join("src");
        std::fs::create_dir_all(&subdir).unwrap();
        let result = resolve_inputs(&[subdir]);
        assert!(result.is_ok());
        let targets = result.unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].root, dir.path().canonicalize().unwrap());
    }

    #[test]
    fn rejects_nonexistent_path() {
        let result = resolve_inputs(&[PathBuf::from("/nonexistent/path/abc123")]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepolyzeError::PathNotFound(_)
        ));
    }
}
