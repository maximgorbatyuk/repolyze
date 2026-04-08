use std::path::PathBuf;

use crate::aggregate::build_comparison_report;
use crate::error::RepolyzeError;
use crate::model::{
    ActivitySummary, ComparisonReport, ContributionSummary, PartialFailure, RepositoryAnalysis,
    RepositoryTarget, SizeMetrics,
};
use crate::settings::Settings;

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
        repository_identifier: &str,
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

/// Analyzer for remote GitHub repositories.
pub trait RemoteAnalyzer {
    fn analyze_remote(&self, owner: &str, repo: &str) -> Result<RepositoryAnalysis, RepolyzeError>;
}

pub fn analyze_targets<G: GitAnalyzer, M: MetricsAnalyzer>(
    targets: &[RepositoryTarget],
    git: &G,
    metrics: &M,
    remote: Option<&dyn RemoteAnalyzer>,
    settings: &Settings,
) -> ComparisonReport {
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for target in targets {
        let result = match target {
            RepositoryTarget::GitHub { owner, repo } => match remote {
                Some(r) => r.analyze_remote(owner, repo),
                None => Err(RepolyzeError::GitHubApi(
                    "GitHub analysis not available".to_string(),
                )),
            },
            RepositoryTarget::Local { .. } => analyze_target(target, git, metrics),
        };
        match result {
            Ok(analysis) => results.push(analysis),
            Err(error) => failures.push(PartialFailure {
                identifier: target.display_path(),
                reason: error.to_string(),
            }),
        }
    }

    build_comparison_report(results, failures, settings)
}

pub fn analyze_targets_with_store<G: GitAnalyzer, M: MetricsAnalyzer, S: AnalysisStore>(
    targets: &[RepositoryTarget],
    git: &G,
    metrics: &M,
    store: &S,
    remote: Option<&dyn RemoteAnalyzer>,
    trigger_source: &str,
    settings: &Settings,
) -> ComparisonReport {
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for target in targets {
        // GitHub targets bypass the local git/cache pipeline
        if let RepositoryTarget::GitHub { owner, repo } = target {
            match remote {
                Some(r) => match r.analyze_remote(owner, repo) {
                    Ok(analysis) => results.push(analysis),
                    Err(error) => failures.push(PartialFailure {
                        identifier: target.display_path(),
                        reason: error.to_string(),
                    }),
                },
                None => failures.push(PartialFailure {
                    identifier: target.display_path(),
                    reason: "GitHub analysis not available".to_string(),
                }),
            }
            continue;
        }

        let cache_key = match git.cache_metadata(target) {
            Ok(metadata) => metadata,
            Err(error) => {
                let id = target.display_path();
                if let Err(e) = store.record_scan_result(
                    None,
                    &id,
                    trigger_source,
                    "miss",
                    "failed",
                    Some(&error.to_string()),
                ) {
                    eprintln!("warning: failed to record scan result: {e}");
                }
                failures.push(PartialFailure {
                    identifier: id,
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
                let id = target.display_path();
                if let Err(e) = store.record_scan_result(
                    Some(&cache_key),
                    &id,
                    trigger_source,
                    cache_status,
                    "failed",
                    Some(&error.to_string()),
                ) {
                    eprintln!("warning: failed to record scan result: {e}");
                }
                failures.push(PartialFailure {
                    identifier: id,
                    reason: error.to_string(),
                });
            }
        }
    }

    build_comparison_report(results, failures, settings)
}

fn analyze_target_with_store<G: GitAnalyzer, M: MetricsAnalyzer, S: AnalysisStore>(
    target: &RepositoryTarget,
    cache_key: &RepositoryCacheMetadata,
    git: &G,
    metrics: &M,
    store: &S,
    trigger_source: &str,
) -> Result<RepositoryAnalysis, RepolyzeError> {
    let id = target.display_path();

    if cache_key.cacheable
        && let Some(cached) = store.load_snapshot(cache_key)?
    {
        if let Err(e) =
            store.record_scan_result(Some(cache_key), &id, trigger_source, "hit", "success", None)
        {
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
            &id,
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
            &id,
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
            _repository_identifier: &str,
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
                repository_root: target.as_local_path().unwrap().to_path_buf(),
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
            repository: RepositoryTarget::Local {
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
        let target = RepositoryTarget::Local {
            root: "/tmp/repo-a".into(),
        };
        let cached = make_repository_analysis("/tmp/repo-a");
        let git = PanicGitAnalyzer;
        let metrics = PanicMetricsAnalyzer;
        let store = FakeAnalysisStore::with_hit(cached.clone());

        let result = analyze_targets_with_store(
            &[target],
            &git,
            &metrics,
            &store,
            None,
            "cli",
            &Settings::default(),
        );

        assert_eq!(result.repositories.len(), 1);
        assert_eq!(
            result.repositories[0].repository.display_path(),
            cached.repository.display_path()
        );
        assert_eq!(
            store.scan_events.borrow().as_slice(),
            &[("hit".to_string(), "success".to_string())]
        );
    }
}
