use std::path::Path;
use std::process::Command;

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{ActivitySummary, ContributionSummary, RepositoryTarget};
use repolyze_core::service::GitAnalyzer;

pub struct GitCliBackend;

impl GitAnalyzer for GitCliBackend {
    fn analyze_git(
        &self,
        target: &RepositoryTarget,
    ) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError> {
        let (contributions, commits) = crate::contributions::analyze_contributions(target)?;
        let activity = crate::activity::build_activity_summary(&commits);
        Ok((contributions, activity))
    }
}

/// Runs a git command in the given repository directory and returns stdout.
pub fn run_git(repo: &Path, args: &[&str]) -> Result<String, RepolyzeError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| RepolyzeError::GitCommand(format!("failed to execute git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RepolyzeError::GitCommand(format!(
            "git {} failed: {stderr}",
            args.join(" ")
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| RepolyzeError::Parse(format!("invalid utf-8 in git output: {e}")))
}
