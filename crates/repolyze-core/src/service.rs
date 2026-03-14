use crate::error::RepolyzeError;
use crate::model::{ActivitySummary, ContributionSummary, RepositoryTarget, SizeMetrics};

pub trait GitAnalyzer {
    fn analyze_git(
        &self,
        target: &RepositoryTarget,
    ) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError>;
}

pub trait MetricsAnalyzer {
    fn analyze_size(&self, target: &RepositoryTarget) -> Result<SizeMetrics, RepolyzeError>;
}
