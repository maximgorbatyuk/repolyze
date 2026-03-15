use repolyze_store::models::ContributorRecord;
use repolyze_store::sqlite::SqliteStore;

#[test]
fn snapshot_writer_persists_summary_weekday_and_hour_stats() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let mut store = SqliteStore::open(&db_path).unwrap();

    let repository_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
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
    let contributor_id = store
        .upsert_contributor(&ContributorRecord::new("alice@example.com", "Alice"))
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
    store
        .upsert_snapshot_contributor_weekday_stat(snapshot_id, contributor_id, 2, 2, 1)
        .unwrap();
    store
        .upsert_snapshot_contributor_hour_stat(snapshot_id, contributor_id, 10, 2, 1)
        .unwrap();

    let summary_rows = store.snapshot_summary_row_count(snapshot_id).unwrap();
    let weekday_rows = store.snapshot_weekday_row_count(snapshot_id).unwrap();
    let hour_rows = store.snapshot_hour_row_count(snapshot_id).unwrap();

    assert_eq!(summary_rows, 1);
    assert_eq!(weekday_rows, 1);
    assert_eq!(hour_rows, 1);
}
