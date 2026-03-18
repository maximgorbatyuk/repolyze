use std::collections::BTreeMap;

use repolyze_core::model::{
    ActivitySummary, ContributionSummary, ContributorActivityStats, ContributorStats,
    RepositoryAnalysis, RepositoryTarget, SizeMetrics,
};
use repolyze_core::service::{AnalysisStore, RepositoryCacheMetadata};
use repolyze_store::sqlite::SqliteStore;
use rusqlite::{Connection, params};

fn make_activity_stats() -> ContributorActivityStats {
    let active_dates = ["2025-01-01".to_string(), "2025-01-15".to_string()]
        .into_iter()
        .collect();

    let mut active_dates_by_weekday = std::array::from_fn(|_| std::collections::BTreeSet::new());
    active_dates_by_weekday[0].insert("2025-01-01".to_string());
    active_dates_by_weekday[2].insert("2025-01-15".to_string());

    let active_hour_buckets = ["2025-01-01:9".to_string(), "2025-01-15:10".to_string()]
        .into_iter()
        .collect();

    let mut active_hour_buckets_by_hour =
        std::array::from_fn(|_| std::collections::BTreeSet::new());
    active_hour_buckets_by_hour[9].insert("2025-01-01:9".to_string());
    active_hour_buckets_by_hour[10].insert("2025-01-15:10".to_string());

    let mut weekday_commits = [0; 7];
    weekday_commits[0] = 2;
    weekday_commits[2] = 1;

    let mut hour_commits = [0; 24];
    hour_commits[9] = 1;
    hour_commits[10] = 2;

    ContributorActivityStats {
        email: "alice@example.com".to_string(),
        weekday_commits,
        hour_commits,
        active_dates,
        active_dates_by_weekday,
        active_hour_buckets,
        active_hour_buckets_by_hour,
        commits_by_date: std::collections::BTreeMap::new(),
    }
}

fn make_analysis() -> RepositoryAnalysis {
    RepositoryAnalysis {
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
                file_extensions: std::collections::BTreeMap::new(),
                active_days: 2,
                first_commit: "2025-01-01T09:00:00+00:00".to_string(),
                last_commit: "2025-01-15T10:00:00+00:00".to_string(),
            }],
            activity_by_contributor: vec![make_activity_stats()],
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
    }
}

#[test]
fn cache_roundtrip_restores_repository_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let analysis = make_analysis();

    let cache_key = RepositoryCacheMetadata {
        repository_root: "/tmp/repo-a".into(),
        history_scope: "head".to_string(),
        head_commit_hash: "abc123".to_string(),
        branch_name: Some("main".to_string()),
        cacheable: true,
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

#[test]
fn save_snapshot_populates_snapshot_fact_tables_and_analysis_period() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let analysis = make_analysis();
    let cache_key = RepositoryCacheMetadata {
        repository_root: "/tmp/repo-a".into(),
        history_scope: "head".to_string(),
        head_commit_hash: "abc123".to_string(),
        branch_name: Some("main".to_string()),
        cacheable: true,
    };

    store.save_snapshot(&cache_key, &analysis).unwrap();

    let conn = Connection::open(&db_path).unwrap();
    let (start_at, end_at): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT analysis_period_start_at, analysis_period_end_at FROM analysis_snapshots LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let summary_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_contributor_summaries",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let weekday_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_contributor_weekday_stats",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let hour_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_contributor_hour_stats",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(start_at.as_deref(), Some("2025-01-01T09:00:00+00:00"));
    assert_eq!(end_at.as_deref(), Some("2025-01-15T10:00:00+00:00"));
    assert_eq!(summary_count, 1);
    assert_eq!(weekday_count, 2);
    assert_eq!(hour_count, 2);
}

#[test]
fn load_snapshot_accepts_legacy_payload_without_activity_by_contributor() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();
    let conn = Connection::open(&db_path).unwrap();

    let repo_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    let by_hour = vec![0; 24];
    let by_weekday = vec![0; 7];
    let heatmap = vec![vec![0; 24]; 7];
    let legacy_payload = serde_json::json!({
        "repository": { "root": "/tmp/repo-a" },
        "contributions": {
            "contributors": [{
                "name": "Alice",
                "email": "alice@example.com",
                "commits": 1,
                "lines_added": 1,
                "lines_deleted": 0,
                "net_lines": 1,
                "files_touched": 1,
                "active_days": 1,
                "first_commit": "2025-01-01T00:00:00+00:00",
                "last_commit": "2025-01-01T00:00:00+00:00"
            }],
            "total_commits": 1
        },
        "activity": { "by_hour": by_hour, "by_weekday": by_weekday, "heatmap": heatmap },
        "size": {
            "files": 1,
            "directories": 1,
            "total_bytes": 1,
            "total_lines": 1,
            "non_empty_lines": 1,
            "blank_lines": 0,
            "by_extension": {},
            "largest_files": [],
            "average_file_size": 1.0
        }
    })
    .to_string();

    conn.execute(
        "INSERT INTO analysis_snapshots (repository_id, history_scope, head_commit_hash, branch_name, analysis_period_start_at, analysis_period_end_at, commits_count, contributors_count, analysis_payload_json, snapshot_created_at, repolyze_version, schema_version, is_complete)
         VALUES (?1, 'head', 'abc123', 'main', NULL, NULL, 1, 1, ?2, '1', ?3, ?4, 1)",
        params![repo_id, legacy_payload, env!("CARGO_PKG_VERSION"), repolyze_store::migrations::SCHEMA_VERSION],
    )
    .unwrap();

    let loaded = store
        .load_snapshot(&RepositoryCacheMetadata {
            repository_root: "/tmp/repo-a".into(),
            history_scope: "head".to_string(),
            head_commit_hash: "abc123".to_string(),
            branch_name: Some("main".to_string()),
            cacheable: true,
        })
        .unwrap()
        .unwrap();

    assert!(loaded.contributions.activity_by_contributor.is_empty());
}

#[test]
fn load_snapshot_ignores_stale_repolyze_version() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let analysis = make_analysis();
    let cache_key = RepositoryCacheMetadata {
        repository_root: "/tmp/repo-a".into(),
        history_scope: "head".to_string(),
        head_commit_hash: "abc123".to_string(),
        branch_name: Some("main".to_string()),
        cacheable: true,
    };

    let repo_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    store
        .insert_snapshot_header(
            repo_id,
            &cache_key.history_scope,
            &cache_key.head_commit_hash,
            cache_key.branch_name.as_deref(),
            Some("2025-01-01T09:00:00+00:00"),
            Some("2025-01-15T10:00:00+00:00"),
            analysis.contributions.total_commits as i64,
            analysis.contributions.contributors.len() as i64,
            &serde_json::to_string(&analysis).unwrap(),
            "0.0.0",
        )
        .unwrap();

    let loaded = store.load_snapshot(&cache_key).unwrap();
    assert!(loaded.is_none());
}
