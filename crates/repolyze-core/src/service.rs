use std::path::PathBuf;

use crate::aggregate::build_comparison_report;
use crate::error::RepolyzeError;
use crate::model::{
    ActivitySummary, ComparisonReport, ContributionSummary, PartialFailure, RepositoryAnalysis,
    RepositoryTarget, SizeMetrics,
};

#[derive(Debug, Clone)]
pub struct RepositoryCacheMetadata {
    pub repository_root: PathBuf,
    pub history_scope: String,
    pub head_commit_hash: String,
    pub branch_name: Option<String>,
}

pub trait AnalysisStore {
    fn load_snapshot(
        &self,
        key: &RepositoryCacheMetadata,
    ) -> Result<Option<RepositoryAnalysis>, RepolyzeError>;
    fn save_snapshot(
        &self,
        key: &RepositoryCacheMetadata,
        analysis: &RepositoryAnalysis,
    ) -> Result<(), RepolyzeError>;
    fn record_scan_failure(
        &self,
        repository_root: &std::path::Path,
        reason: &str,
    ) -> Result<(), RepolyzeError>;
}

pub trait GitAnalyzer {
    fn cache_metadata(
        &self,
        target: &RepositoryTarget,
    ) -> Result<RepositoryCacheMetadata, RepolyzeError>;
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

pub fn analyze_targets_with_store<G: GitAnalyzer, M: MetricsAnalyzer, S: AnalysisStore>(
    targets: &[RepositoryTarget],
    git: &G,
    metrics: &M,
    store: &S,
) -> ComparisonReport {
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for target in targets {
        match analyze_target_with_store(target, git, metrics, store) {
            Ok(analysis) => results.push(analysis),
            Err(error) => {
                let _ = store.record_scan_failure(&target.root, &error.to_string());
                failures.push(PartialFailure {
                    path: target.root.clone(),
                    reason: error.to_string(),
                });
            }
        }
    }

    build_comparison_report(results, failures)
}

fn analyze_target_with_store<G: GitAnalyzer, M: MetricsAnalyzer, S: AnalysisStore>(
    target: &RepositoryTarget,
    git: &G,
    metrics: &M,
    store: &S,
) -> Result<RepositoryAnalysis, RepolyzeError> {
    let cache_key = git.cache_metadata(target)?;

    if let Some(cached) = store.load_snapshot(&cache_key)? {
        return Ok(cached);
    }

    let analysis = analyze_target(target, git, metrics)?;
    let _ = store.save_snapshot(&cache_key, &analysis);
    Ok(analysis)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    struct FakeAnalysisStore {
        cached: RefCell<Option<RepositoryAnalysis>>,
    }

    impl FakeAnalysisStore {
        fn with_hit(analysis: RepositoryAnalysis) -> Self {
            Self {
                cached: RefCell::new(Some(analysis)),
            }
        }
    }

    impl AnalysisStore for FakeAnalysisStore {
        fn load_snapshot(
            &self,
            _key: &RepositoryCacheMetadata,
        ) -> Result<Option<RepositoryAnalysis>, RepolyzeError> {
            Ok(self.cached.borrow().clone())
        }
        fn save_snapshot(
            &self,
            _key: &RepositoryCacheMetadata,
            _analysis: &RepositoryAnalysis,
        ) -> Result<(), RepolyzeError> {
            Ok(())
        }
        fn record_scan_failure(
            &self,
            _repository_root: &std::path::Path,
            _reason: &str,
        ) -> Result<(), RepolyzeError> {
            Ok(())
        }
    }

    struct PanicGitAnalyzer;

    impl GitAnalyzer for PanicGitAnalyzer {
        fn cache_metadata(
            &self,
            target: &RepositoryTarget,
        ) -> Result<RepositoryCacheMetadata, RepolyzeError> {
            Ok(RepositoryCacheMetadata {
                repository_root: target.root.clone(),
                history_scope: "head".to_string(),
                head_commit_hash: "abc123".to_string(),
                branch_name: Some("main".to_string()),
            })
        }
        fn analyze_git(
            &self,
            _target: &RepositoryTarget,
        ) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError> {
            panic!("should not be called on cache hit");
        }
    }

    struct PanicMetricsAnalyzer;

    impl MetricsAnalyzer for PanicMetricsAnalyzer {
        fn analyze_size(&self, _target: &RepositoryTarget) -> Result<SizeMetrics, RepolyzeError> {
            panic!("should not be called on cache hit");
        }
    }

    fn make_repository_analysis(path: &str) -> RepositoryAnalysis {
        RepositoryAnalysis {
            repository: RepositoryTarget {
                root: PathBuf::from(path),
            },
            contributions: ContributionSummary {
                contributors: vec![],
                activity_by_contributor: vec![],
                total_commits: 0,
            },
            activity: ActivitySummary::default(),
            size: SizeMetrics {
                files: 0,
                directories: 0,
                total_bytes: 0,
                total_lines: 0,
                non_empty_lines: 0,
                blank_lines: 0,
                by_extension: BTreeMap::new(),
                largest_files: vec![],
                average_file_size: 0.0,
            },
        }
    }

    #[test]
    fn analyze_target_uses_cached_snapshot_when_key_matches() {
        let target = RepositoryTarget {
            root: "/tmp/repo-a".into(),
        };
        let cached = make_repository_analysis("/tmp/repo-a");
        let git = PanicGitAnalyzer;
        let metrics = PanicMetricsAnalyzer;
        let store = FakeAnalysisStore::with_hit(cached.clone());

        let result = analyze_targets_with_store(&[target], &git, &metrics, &store);

        assert_eq!(result.repositories.len(), 1);
        assert_eq!(
            result.repositories[0].repository.root,
            cached.repository.root
        );
    }
}
