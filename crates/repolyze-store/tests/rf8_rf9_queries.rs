use repolyze_store::models::ContributorRecord;
use repolyze_store::sqlite::SqliteStore;

fn seed_snapshot_with_one_contributor(store: &SqliteStore) -> SnapshotFixture {
    let repository_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    let contributor_id = store
        .upsert_contributor(&ContributorRecord::new("alice@example.com", "Alice"))
        .unwrap();
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
            "{}",
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
        .users_contribution_rows_for_snapshots(&[fixture.snapshot_id])
        .unwrap();
    let rf9_rows = store
        .user_activity_rows_for_snapshots(&[fixture.snapshot_id])
        .unwrap();

    assert_eq!(rf8_rows.len(), 1);
    assert_eq!(rf9_rows.len(), 1);
    assert_eq!(rf8_rows[0].email, "alice@example.com");
    assert_eq!(rf9_rows[0].email, "alice@example.com");
}
