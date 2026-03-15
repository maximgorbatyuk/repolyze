use repolyze_store::path::{database_path_for_dev, database_path_from_home};

#[test]
fn database_path_defaults_to_repolyze_db_in_home_directory() {
    let path = database_path_from_home("/tmp/test-home");
    assert_eq!(
        path,
        std::path::PathBuf::from("/tmp/test-home/.repolyze/repolyze.db")
    );
}

#[test]
fn dev_database_path_is_next_to_binary() {
    let path = database_path_for_dev().unwrap();
    assert!(path.ends_with("repolyze-dev.db"));
    // The parent should be the directory containing the test binary
    assert!(path.parent().is_some());
}
