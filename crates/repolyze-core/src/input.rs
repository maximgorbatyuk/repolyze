use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::RepolyzeError;
use crate::model::{PartialFailure, RepositoryTarget};

/// Resolves user-provided paths into validated `RepositoryTarget` values.
///
/// - Canonicalizes each path
/// - Walks upward to find `.git` directory if needed
/// - Deduplicates by canonical path
/// - Returns error for paths that don't exist or aren't Git repositories
pub fn resolve_inputs(paths: &[PathBuf]) -> Result<Vec<RepositoryTarget>, RepolyzeError> {
    let mut targets = Vec::new();

    for path in paths {
        targets.extend(resolve_input(path)?);
    }

    Ok(dedup_targets(targets))
}

pub fn resolve_inputs_with_failures(
    paths: &[PathBuf],
) -> (Vec<RepositoryTarget>, Vec<PartialFailure>) {
    let mut targets = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match resolve_input(path) {
            Ok(discovered) => targets.extend(discovered),
            Err(error) => failures.push(PartialFailure {
                path: path.clone(),
                reason: error.to_string(),
            }),
        }
    }

    (dedup_targets(targets), failures)
}

pub fn resolve_input(path: &Path) -> Result<Vec<RepositoryTarget>, RepolyzeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| RepolyzeError::PathNotFound(path.to_path_buf()))?;

    // Try upward walk first — if inside a repo, use that repo
    if let Ok(root) = find_git_root(&canonical) {
        return Ok(vec![RepositoryTarget { root }]);
    }

    // If it's a directory, scan downward for nested repos
    if canonical.is_dir() {
        let roots = discover_git_roots(&canonical);
        if roots.is_empty() {
            return Err(RepolyzeError::NoRepositoriesFound(canonical));
        }
        return Ok(roots
            .into_iter()
            .map(|root| RepositoryTarget { root })
            .collect());
    }

    Err(RepolyzeError::NotAGitRepository(path.to_path_buf()))
}

fn dedup_targets(targets: Vec<RepositoryTarget>) -> Vec<RepositoryTarget> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for target in targets {
        if seen.insert(target.root.clone()) {
            deduped.push(target);
        }
    }

    deduped.sort_by(|a, b| a.root.cmp(&b.root));
    deduped
}

/// Recursively scan a directory for Git repositories.
/// Stops descending into a directory once a `.git` marker is found there.
fn discover_git_roots(dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut visited = HashSet::new();
    discover_git_roots_recursive(dir, &mut roots, &mut visited);
    roots
}

fn discover_git_roots_recursive(
    dir: &Path,
    roots: &mut Vec<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) {
    let canonical = match dir.canonicalize() {
        Ok(path) => path,
        Err(_) => return,
    };

    if !visited.insert(canonical.clone()) {
        return;
    }

    // .git can be a directory (normal repo) or a file (worktree/submodule)
    if canonical.join(".git").exists() {
        roots.push(canonical);
        // Don't descend further — this is a repo root
        return;
    }

    let entries = match std::fs::read_dir(&canonical) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if is_dir {
            discover_git_roots_recursive(&path, roots, visited);
        }
    }
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
            matches!(err, RepolyzeError::NoRepositoriesFound(_)),
            "expected NoRepositoriesFound, got: {err}"
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

    #[test]
    fn resolve_inputs_with_failures_keeps_valid_targets() {
        let dir = create_temp_git_repo();
        let (targets, failures) =
            resolve_inputs_with_failures(&[dir.path().to_path_buf(), PathBuf::from("/missing")]);

        assert_eq!(targets.len(), 1);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].reason.contains("path does not exist"));
    }

    fn create_temp_git_repo_at(path: PathBuf) -> PathBuf {
        std::fs::create_dir_all(&path).unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&path)
            .output()
            .unwrap();
        path.canonicalize().unwrap()
    }

    #[test]
    fn resolve_inputs_with_failures_discovers_nested_git_repositories() {
        let root = tempfile::tempdir().unwrap();
        let repo_a = create_temp_git_repo_at(root.path().join("workspace/repo-a"));
        let repo_b = create_temp_git_repo_at(root.path().join("workspace/tools/repo-b"));

        let (targets, failures) = resolve_inputs_with_failures(&[root.path().join("workspace")]);

        let roots: Vec<_> = targets.into_iter().map(|t| t.root).collect();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&repo_a));
        assert!(roots.contains(&repo_b));
        assert!(failures.is_empty());
    }

    #[test]
    fn resolve_inputs_prefers_enclosing_git_repository_for_subdirectory() {
        let repo = create_temp_git_repo();
        let nested = repo.path().join("src/deep");
        std::fs::create_dir_all(&nested).unwrap();

        let targets = resolve_inputs(&[nested]).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].root, repo.path().canonicalize().unwrap());
    }

    #[test]
    fn resolve_inputs_with_failures_reports_directory_without_repositories() {
        let dir = tempfile::tempdir().unwrap();

        let (targets, failures) = resolve_inputs_with_failures(&[dir.path().to_path_buf()]);

        assert!(targets.is_empty());
        assert_eq!(failures.len(), 1);
        assert!(failures[0].reason.contains("no git repositories found"));
    }

    #[test]
    fn resolve_inputs_with_failures_discovers_deeply_nested_repositories() {
        let root = tempfile::tempdir().unwrap();
        let mut nested = root.path().join("workspace");
        for segment in [
            "a",
            "b",
            "c",
            "d",
            "e",
            "f",
            "g",
            "h",
            "i",
            "j",
            "k",
            "repo-deep",
        ] {
            nested = nested.join(segment);
        }
        let deep_repo = create_temp_git_repo_at(nested);

        let (targets, failures) = resolve_inputs_with_failures(&[root.path().join("workspace")]);

        assert!(failures.is_empty());
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].root, deep_repo);
    }
}
