use crate::aggregate::build_comparison_report;
use crate::error::RepolyzeError;
use crate::model::{
    ActivitySummary, ComparisonReport, ContributionSummary, PartialFailure, RepositoryAnalysis,
    RepositoryTarget, SizeMetrics,
};

pub trait GitAnalyzer {
    fn analyze_git(
        &self,
        target: &RepositoryTarget,
    ) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError>;
}

pub trait MetricsAnalyzer {
    fn analyze_size(&self, target: &RepositoryTarget) -> Result<SizeMetrics, RepolyzeError>;
}

pub fn analyze_targets<G: GitAnalyzer, M: MetricsAnalyzer>(
    targets: &[RepositoryTarget],
    git: &G,
    metrics: &M,
) -> ComparisonReport {
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for target in targets {
        match analyze_target(target, git, metrics) {
            Ok(analysis) => results.push(analysis),
            Err(error) => failures.push(PartialFailure {
                path: target.root.clone(),
                reason: error.to_string(),
            }),
        }
    }

    build_comparison_report(results, failures)
}

fn analyze_target<G: GitAnalyzer, M: MetricsAnalyzer>(
    target: &RepositoryTarget,
    git: &G,
    metrics: &M,
) -> Result<RepositoryAnalysis, RepolyzeError> {
    let (contributions, activity) = git.analyze_git(target)?;
    let size = metrics.analyze_size(target)?;

    Ok(RepositoryAnalysis {
        repository: target.clone(),
        contributions,
        activity,
        size,
    })
}
