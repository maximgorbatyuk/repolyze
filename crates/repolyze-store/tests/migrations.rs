use repolyze_store::sqlite::SqliteStore;

#[test]
fn sqlite_store_bootstrap_creates_metadata_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");

    let store = SqliteStore::open(&db_path).unwrap();
    let table_names = store.table_names().unwrap();

    assert!(table_names.contains(&"app_settings".to_string()));
    assert!(table_names.contains(&"repositories".to_string()));
    assert!(table_names.contains(&"analysis_snapshots".to_string()));
    assert!(table_names.contains(&"scan_runs".to_string()));
}
