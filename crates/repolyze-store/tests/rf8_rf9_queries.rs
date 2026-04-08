use repolyze_store::models::ContributorRecord;
use repolyze_store::sqlite::SqliteStore;

fn seed_snapshot_with_one_contributor(store: &SqliteStore) -> SnapshotFixture {
    let repository_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    let contributor_id = store
        .upsert_contributor(&ContributorRecord::new("alice@example.com", "Alice"))
        .unwrap();
    let payload = serde_json::json!({
        "repository": { "Local": { "root": "/tmp/repo-a" } },
        "contributions": {
            "contributors": [{
                "name": "Alice",
                "email": "alice@example.com",
                "commits": 3,
                "lines_added": 12,
                "lines_deleted": 4,
                "net_lines": 8,
                "files_touched": 2,
                "active_days": 2,
                "first_commit": "2025-01-01T09:00:00+00:00",
                "last_commit": "2025-01-15T10:00:00+00:00"
            }],
            "activity_by_contributor": [{
                "email": "alice@example.com",
                "weekday_commits": [0, 0, 2, 1, 0, 0, 0],
                "hour_commits": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "active_dates": ["2025-01-01", "2025-01-15"],
                "active_dates_by_weekday": [[], [], ["2025-01-01"], ["2025-01-15"], [], [], []],
                "active_hour_buckets": ["2025-01-01:10", "2025-01-15:14"],
                "active_hour_buckets_by_hour": [[], [], [], [], [], [], [], [], [], [], ["2025-01-01:10"], [], [], [], ["2025-01-15:14"], [], [], [], [], [], [], [], [], []]
            }],
            "total_commits": 3
        },
        "activity": { "by_hour": [0,0,0,0,0,0,0,0,0,0,2,0,0,0,1,0,0,0,0,0,0,0,0,0], "by_weekday": [0,0,2,1,0,0,0], "heatmap": [[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]] },
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
    let snapshot_id = store
        .insert_snapshot_header(
            repository_id,
            "head",
            "abc123",
            Some("main"),
            Some("2025-01-01T00:00:00+00:00"),
            Some("2025-01-15T10:00:00+00:00"),
            3,
            1,
            &payload,
            "0.1.1",
        )
        .unwrap();

    store
        .upsert_snapshot_contributor_summary(
            snapshot_id,
            contributor_id,
            3,
            12,
            4,
            16,
            2,
            2,
            "2025-01-01T09:00:00+00:00",
            "2025-01-15T10:00:00+00:00",
            Some(2),
            Some(10),
        )
        .unwrap();

    // weekday 2 = Tuesday, 2 commits, 1 active date
    store
        .upsert_snapshot_contributor_weekday_stat(snapshot_id, contributor_id, 2, 2, 1)
        .unwrap();
    // weekday 3 = Wednesday, 1 commit, 1 active date
    store
        .upsert_snapshot_contributor_weekday_stat(snapshot_id, contributor_id, 3, 1, 1)
        .unwrap();

    // hour 10, 2 commits
    store
        .upsert_snapshot_contributor_hour_stat(snapshot_id, contributor_id, 10, 2, 1)
        .unwrap();
    // hour 14, 1 commit
    store
        .upsert_snapshot_contributor_hour_stat(snapshot_id, contributor_id, 14, 1, 1)
        .unwrap();

    SnapshotFixture { snapshot_id }
}

struct SnapshotFixture {
    snapshot_id: i64,
}

#[test]
fn rf8_and_rf9_queries_return_snapshot_scoped_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let fixture = seed_snapshot_with_one_contributor(&store);

    let rf8_rows = store
        .contribution_rows_for_snapshots(&[fixture.snapshot_id])
        .unwrap();
    let rf9_rows = store
        .user_activity_rows_for_snapshots(&[fixture.snapshot_id])
        .unwrap();

    assert_eq!(rf8_rows.len(), 1);
    assert_eq!(rf9_rows.len(), 1);
    assert_eq!(rf8_rows[0].email, "alice@example.com");
    assert_eq!(rf9_rows[0].email, "alice@example.com");
    assert_eq!(rf9_rows[0].most_active_week_day, "Wednesday");
}

#[test]
fn rf8_and_rf9_queries_merge_same_email_across_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let fixture_a = seed_snapshot_with_one_contributor(&store);

    let repository_id = store.upsert_repository("/tmp/repo-b", "repo-b").unwrap();
    let contributor_id = store
        .upsert_contributor(&ContributorRecord::new("alice@example.com", "Alice"))
        .unwrap();
    let payload = serde_json::json!({
        "repository": { "Local": { "root": "/tmp/repo-b" } },
        "contributions": {
            "contributors": [{
                "name": "Alice",
                "email": "alice@example.com",
                "commits": 2,
                "lines_added": 8,
                "lines_deleted": 2,
                "net_lines": 6,
                "files_touched": 1,
                "active_days": 1,
                "first_commit": "2025-01-20T12:00:00+00:00",
                "last_commit": "2025-01-20T12:00:00+00:00"
            }],
            "activity_by_contributor": [{
                "email": "alice@example.com",
                "weekday_commits": [0, 0, 0, 0, 0, 0, 2],
                "hour_commits": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "active_dates": ["2025-01-20"],
                "active_dates_by_weekday": [[], [], [], [], [], [], ["2025-01-20"]],
                "active_hour_buckets": ["2025-01-20:12"],
                "active_hour_buckets_by_hour": [[], [], [], [], [], [], [], [], [], [], [], [], ["2025-01-20:12"], [], [], [], [], [], [], [], [], [], [], []]
            }],
            "total_commits": 2
        },
        "activity": { "by_hour": [0,0,0,0,0,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0], "by_weekday": [0,0,0,0,0,0,2], "heatmap": [[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],[0,0,0,0,0,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0]] },
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

    let snapshot_b = store
        .insert_snapshot_header(
            repository_id,
            "head",
            "def456",
            Some("main"),
            Some("2025-01-20T00:00:00+00:00"),
            Some("2025-01-20T12:00:00+00:00"),
            2,
            1,
            &payload,
            "0.1.1",
        )
        .unwrap();

    store
        .upsert_snapshot_contributor_summary(
            snapshot_b,
            contributor_id,
            2,
            8,
            2,
            10,
            1,
            1,
            "2025-01-20T12:00:00+00:00",
            "2025-01-20T12:00:00+00:00",
            Some(6),
            Some(12),
        )
        .unwrap();
    store
        .upsert_snapshot_contributor_weekday_stat(snapshot_b, contributor_id, 6, 2, 1)
        .unwrap();
    store
        .upsert_snapshot_contributor_hour_stat(snapshot_b, contributor_id, 12, 2, 1)
        .unwrap();

    let snapshot_ids = [fixture_a.snapshot_id, snapshot_b];
    let rf8_rows = store
        .contribution_rows_for_snapshots(&snapshot_ids)
        .unwrap();
    let rf9_rows = store
        .user_activity_rows_for_snapshots(&snapshot_ids)
        .unwrap();

    assert_eq!(rf8_rows.len(), 1);
    assert_eq!(rf9_rows.len(), 1);
    assert_eq!(rf8_rows[0].email, "alice@example.com");
    assert_eq!(rf8_rows[0].commits, 5);
    assert_eq!(rf9_rows[0].email, "alice@example.com");
}
