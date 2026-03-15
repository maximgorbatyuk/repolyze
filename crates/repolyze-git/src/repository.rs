use std::path::Path;

use repolyze_core::error::RepolyzeError;

use crate::backend::run_git;

#[derive(Debug, Clone)]
pub struct HeadMetadata {
    pub head_commit_hash: String,
    pub branch_name: Option<String>,
}

pub fn current_head_metadata(repo: &Path) -> Result<HeadMetadata, RepolyzeError> {
    let head_commit_hash = run_git(repo, &["rev-parse", "HEAD"])?.trim().to_string();
    let branch_raw = run_git(repo, &["branch", "--show-current"])?
        .trim()
        .to_string();
    let branch_name = if branch_raw.is_empty() {
        None
    } else {
        Some(branch_raw)
    };

    Ok(HeadMetadata {
        head_commit_hash,
        branch_name,
    })
}

pub fn is_worktree_clean(repo: &Path) -> Result<bool, RepolyzeError> {
    let status = run_git(repo, &["status", "--porcelain", "--untracked-files=all"])?;
    Ok(status.trim().is_empty())
}
