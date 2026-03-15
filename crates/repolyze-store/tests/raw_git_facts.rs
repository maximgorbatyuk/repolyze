use repolyze_store::models::{CommitFileChangeRecord, CommitRecord, ContributorRecord};
use repolyze_store::sqlite::SqliteStore;

#[test]
fn raw_commit_writer_dedupes_commit_hash_per_repository() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let mut store = SqliteStore::open(&db_path).unwrap();

    let repository_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    let contributor_id = store
        .upsert_contributor(&ContributorRecord::new("alice@example.com", "Alice"))
        .unwrap();

    let commit = CommitRecord::new(
        repository_id,
        contributor_id,
        "abc123",
        "Alice",
        "alice@example.com",
        "2025-01-15T10:00:00+00:00",
        10,
        2,
        2,
        12,
        4,
        16,
    );
    let file_change = CommitFileChangeRecord::new("src/lib.rs", 12, 4, 16);

    let first_id = store
        .upsert_commit(&commit, std::slice::from_ref(&file_change))
        .unwrap();
    let second_id = store
        .upsert_commit(&commit, std::slice::from_ref(&file_change))
        .unwrap();

    assert_eq!(first_id, second_id);
    assert_eq!(store.commit_count(repository_id).unwrap(), 1);
}
