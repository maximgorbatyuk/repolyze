use std::collections::BTreeMap;

use repolyze_core::model::{
    ActivitySummary, ContributionSummary, ContributorStats, RepositoryAnalysis, RepositoryTarget,
    SizeMetrics,
};
use repolyze_core::service::{AnalysisStore, RepositoryCacheMetadata};
use repolyze_store::sqlite::SqliteStore;

#[test]
fn cache_roundtrip_restores_repository_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let analysis = RepositoryAnalysis {
        repository: RepositoryTarget {
            root: "/tmp/repo-a".into(),
        },
        contributions: ContributionSummary {
            contributors: vec![ContributorStats {
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                commits: 3,
                lines_added: 12,
                lines_deleted: 4,
                net_lines: 8,
                files_touched: 2,
                active_days: 2,
                first_commit: "2025-01-01T09:00:00+00:00".to_string(),
                last_commit: "2025-01-15T10:00:00+00:00".to_string(),
            }],
            total_commits: 3,
        },
        activity: ActivitySummary::default(),
        size: SizeMetrics {
            files: 5,
            directories: 2,
            total_bytes: 1000,
            total_lines: 100,
            non_empty_lines: 90,
            blank_lines: 10,
            by_extension: BTreeMap::new(),
            largest_files: vec![],
            average_file_size: 200.0,
        },
    };

    let cache_key = RepositoryCacheMetadata {
        repository_root: "/tmp/repo-a".into(),
        history_scope: "head".to_string(),
        head_commit_hash: "abc123".to_string(),
        branch_name: Some("main".to_string()),
    };

    store.save_snapshot(&cache_key, &analysis).unwrap();
    let loaded = store.load_snapshot(&cache_key).unwrap();

    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.repository.root, analysis.repository.root);
    assert_eq!(loaded.contributions.total_commits, 3);
    assert_eq!(loaded.contributions.contributors.len(), 1);
    assert_eq!(loaded.contributions.contributors[0].name, "Alice");
}
