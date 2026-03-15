use repolyze_store::path::{database_path_for_dev, database_path_from_home, resolve_database_path};

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
    assert!(path.parent().is_some());
}

#[test]
fn resolve_database_path_respects_env_override() {
    let dir = tempfile::tempdir().unwrap();
    let custom = dir.path().join("custom.db");
    unsafe {
        std::env::set_var("REPOLYZE_DB_PATH", &custom);
    }
    let result = resolve_database_path().unwrap();
    unsafe {
        std::env::remove_var("REPOLYZE_DB_PATH");
    }
    assert_eq!(result, custom);
}
