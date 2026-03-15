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
    pub cacheable: bool,
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
    fn record_scan_result(
        &self,
        key: Option<&RepositoryCacheMetadata>,
        repository_root: &std::path::Path,
        trigger_source: &str,
        cache_status: &str,
        status: &str,
        failure_reason: Option<&str>,
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
    trigger_source: &str,
) -> ComparisonReport {
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for target in targets {
        let cache_key = match git.cache_metadata(target) {
            Ok(metadata) => metadata,
            Err(error) => {
                if let Err(e) = store.record_scan_result(
                    None,
                    &target.root,
                    trigger_source,
                    "miss",
                    "failed",
                    Some(&error.to_string()),
                ) {
                    eprintln!("warning: failed to record scan result: {e}");
                }
                failures.push(PartialFailure {
                    path: target.root.clone(),
                    reason: error.to_string(),
                });
                continue;
            }
        };

        match analyze_target_with_store(target, &cache_key, git, metrics, store, trigger_source) {
            Ok(analysis) => results.push(analysis),
            Err(error) => {
                let cache_status = if cache_key.cacheable {
                    "miss"
                } else {
                    "bypass"
                };
                if let Err(e) = store.record_scan_result(
                    Some(&cache_key),
                    &target.root,
                    trigger_source,
                    cache_status,
                    "failed",
                    Some(&error.to_string()),
                ) {
                    eprintln!("warning: failed to record scan result: {e}");
                }
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
    cache_key: &RepositoryCacheMetadata,
    git: &G,
    metrics: &M,
    store: &S,
    trigger_source: &str,
) -> Result<RepositoryAnalysis, RepolyzeError> {
    if cache_key.cacheable
        && let Some(cached) = store.load_snapshot(cache_key)?
    {
        if let Err(e) = store.record_scan_result(
            Some(cache_key),
            &target.root,
            trigger_source,
            "hit",
            "success",
            None,
        ) {
            eprintln!("warning: failed to record scan result: {e}");
        }
        return Ok(cached);
    }

    let analysis = analyze_target(target, git, metrics)?;

    if cache_key.cacheable {
        if let Err(e) = store.save_snapshot(cache_key, &analysis) {
            eprintln!("warning: failed to record scan result: {e}");
        }
        if let Err(e) = store.record_scan_result(
            Some(cache_key),
            &target.root,
            trigger_source,
            "miss",
            "success",
            None,
        ) {
            eprintln!("warning: failed to record scan result: {e}");
        }
    } else {
        if let Err(e) = store.record_scan_result(
            Some(cache_key),
            &target.root,
            trigger_source,
            "bypass",
            "success",
            None,
        ) {
            eprintln!("warning: failed to record scan result: {e}");
        }
    }

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
        scan_events: RefCell<Vec<(String, String)>>,
    }

    impl FakeAnalysisStore {
        fn with_hit(analysis: RepositoryAnalysis) -> Self {
            Self {
                cached: RefCell::new(Some(analysis)),
                scan_events: RefCell::new(Vec::new()),
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
        fn record_scan_result(
            &self,
            _key: Option<&RepositoryCacheMetadata>,
            _repository_root: &std::path::Path,
            _trigger_source: &str,
            cache_status: &str,
            status: &str,
            _failure_reason: Option<&str>,
        ) -> Result<(), RepolyzeError> {
            self.scan_events
                .borrow_mut()
                .push((cache_status.to_string(), status.to_string()));
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
                cacheable: true,
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

        let result = analyze_targets_with_store(&[target], &git, &metrics, &store, "cli");

        assert_eq!(result.repositories.len(), 1);
        assert_eq!(
            result.repositories[0].repository.root,
            cached.repository.root
        );
        assert_eq!(
            store.scan_events.borrow().as_slice(),
            &[("hit".to_string(), "success".to_string())]
        );
    }
}
