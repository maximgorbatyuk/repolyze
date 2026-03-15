use repolyze_store::models::ContributorRecord;
use repolyze_store::sqlite::SqliteStore;

#[test]
fn metadata_excludes_sqlite_internal_tables() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("test.db")).unwrap();
    let meta = store.database_metadata().unwrap();
    for row in &meta.tables {
        assert!(!row.table_name.starts_with("sqlite_"));
    }
    assert!(!meta.tables.is_empty());
}

#[test]
fn metadata_row_counts_are_correct() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("test.db")).unwrap();
    store.upsert_repository("/tmp/a", "a").unwrap();
    store.upsert_repository("/tmp/b", "b").unwrap();
    store
        .upsert_contributor(&ContributorRecord::new("alice@test.com", "Alice"))
        .unwrap();

    let meta = store.database_metadata().unwrap();
    let repos = meta
        .tables
        .iter()
        .find(|t| t.table_name == "repositories")
        .unwrap();
    assert_eq!(repos.record_count, 2);
    let contribs = meta
        .tables
        .iter()
        .find(|t| t.table_name == "contributors")
        .unwrap();
    assert_eq!(contribs.record_count, 1);
}

#[test]
fn metadata_percentages_sum_to_100() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("test.db")).unwrap();
    store.upsert_repository("/tmp/a", "a").unwrap();
    let meta = store.database_metadata().unwrap();
    if meta.total_rows > 0 {
        let sum: f64 = meta.tables.iter().map(|t| t.percentage).sum();
        assert!((sum - 100.0).abs() < 0.2, "got {sum}");
    }
}

#[test]
fn metadata_total_matches_sum() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("test.db")).unwrap();
    store.upsert_repository("/tmp/a", "a").unwrap();
    let meta = store.database_metadata().unwrap();
    let sum: i64 = meta.tables.iter().map(|t| t.record_count).sum();
    assert_eq!(meta.total_rows, sum);
}

#[test]
fn metadata_sorted_by_count_desc_then_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("test.db")).unwrap();
    let meta = store.database_metadata().unwrap();
    for w in meta.tables.windows(2) {
        assert!(
            w[0].record_count > w[1].record_count
                || (w[0].record_count == w[1].record_count && w[0].table_name <= w[1].table_name)
        );
    }
}

#[test]
fn metadata_empty_database() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&dir.path().join("test.db")).unwrap();
    let meta = store.database_metadata().unwrap();
    assert_eq!(meta.total_rows, 0);
    for row in &meta.tables {
        assert_eq!(row.record_count, 0);
        assert_eq!(row.percentage, 0.0);
    }
}
