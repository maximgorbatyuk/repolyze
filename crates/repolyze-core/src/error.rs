use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RepolyzeError {
    #[error("path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("not a git repository: {0}")]
    NotAGitRepository(PathBuf),

    #[error("git command failed: {0}")]
    GitCommand(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("store error: {0}")]
    Store(String),

    #[error("no git repositories found under directory: {0}")]
    NoRepositoriesFound(PathBuf),
}
