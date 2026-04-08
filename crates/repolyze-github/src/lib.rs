mod activity;
mod api_types;
pub mod client;
mod contributors;
mod size;

use std::sync::mpsc::Sender;

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{RepositoryAnalysis, RepositoryTarget};
use repolyze_core::service::RemoteAnalyzer;

use client::GitHubClient;

/// Backend for analyzing GitHub repositories via the API.
pub struct GitHubBackend {
    client: GitHubClient,
}

impl GitHubBackend {
    /// Create a new backend, detecting `gh` CLI availability.
    pub fn new(progress_tx: Option<Sender<String>>) -> Self {
        Self {
            client: GitHubClient::new(progress_tx),
        }
    }

    /// Analyze a GitHub repository and return the full analysis.
    pub fn analyze(&self, owner: &str, repo: &str) -> Result<RepositoryAnalysis, RepolyzeError> {
        self.client
            .log(&format!("Analyzing github.com/{owner}/{repo}..."));

        let contributions = contributors::fetch_contributions(&self.client, owner, repo)?;
        let activity_summary = activity::fetch_activity(&self.client, owner, repo)?;
        let (size_metrics, _repo_info) = size::fetch_size(&self.client, owner, repo)?;

        self.client.log("Analysis complete.");

        Ok(RepositoryAnalysis {
            repository: RepositoryTarget::GitHub {
                owner: owner.to_string(),
                repo: repo.to_string(),
            },
            contributions,
            activity: activity_summary,
            size: size_metrics,
        })
    }
}

impl RemoteAnalyzer for GitHubBackend {
    fn analyze_remote(&self, owner: &str, repo: &str) -> Result<RepositoryAnalysis, RepolyzeError> {
        self.analyze(owner, repo)
    }
}
