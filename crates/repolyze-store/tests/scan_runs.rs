use std::collections::BTreeMap;

use repolyze_core::model::{
    ActivitySummary, ComparisonReport, ContributionSummary, ContributorStats, RepositoryTarget,
    SizeMetrics,
};
use repolyze_core::service::{GitAnalyzer, RepositoryCacheMetadata, analyze_targets_with_store};
use repolyze_store::sqlite::SqliteStore;
use rusqlite::Connection;

struct FakeGitAnalyzer;

impl GitAnalyzer for FakeGitAnalyzer {
    fn cache_metadata(
        &self,
        target: &RepositoryTarget,
    ) -> Result<RepositoryCacheMetadata, repolyze_core::error::RepolyzeError> {
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
    ) -> Result<
        (
            repolyze_core::model::ContributionSummary,
            repolyze_core::model::ActivitySummary,
        ),
        repolyze_core::error::RepolyzeError,
    > {
        Ok((
            ContributionSummary {
                contributors: vec![ContributorStats {
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                    commits: 1,
                    lines_added: 1,
                    lines_deleted: 0,
                    net_lines: 1,
                    files_touched: 1,
                    file_extensions: std::collections::BTreeMap::new(),
                    active_days: 1,
                    first_commit: "2025-01-01T00:00:00+00:00".to_string(),
                    last_commit: "2025-01-01T00:00:00+00:00".to_string(),
                }],
                activity_by_contributor: vec![],
                total_commits: 1,
            },
            ActivitySummary::default(),
        ))
    }
}

struct FakeMetricsAnalyzer;

impl repolyze_core::service::MetricsAnalyzer for FakeMetricsAnalyzer {
    fn analyze_size(
        &self,
        _target: &RepositoryTarget,
    ) -> Result<SizeMetrics, repolyze_core::error::RepolyzeError> {
        Ok(SizeMetrics {
            files: 1,
            directories: 1,
            total_bytes: 1,
            total_lines: 1,
            non_empty_lines: 1,
            blank_lines: 0,
            by_extension: BTreeMap::new(),
            largest_files: vec![],
            average_file_size: 1.0,
        })
    }
}

#[test]
fn analyze_targets_with_store_records_miss_then_hit_runs() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();
    let target = RepositoryTarget {
        root: "/tmp/repo-a".into(),
    };

    let first: ComparisonReport = analyze_targets_with_store(
        std::slice::from_ref(&target),
        &FakeGitAnalyzer,
        &FakeMetricsAnalyzer,
        &store,
        "tui",
    );
    let second: ComparisonReport = analyze_targets_with_store(
        std::slice::from_ref(&target),
        &FakeGitAnalyzer,
        &FakeMetricsAnalyzer,
        &store,
        "tui",
    );

    assert_eq!(first.repositories.len(), 1);
    assert_eq!(second.repositories.len(), 1);

    let conn = Connection::open(&db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT cache_status, trigger_source, status FROM scan_runs ORDER BY id")
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        rows,
        vec![
            ("miss".to_string(), "tui".to_string(), "success".to_string()),
            ("hit".to_string(), "tui".to_string(), "success".to_string()),
        ]
    );
}
