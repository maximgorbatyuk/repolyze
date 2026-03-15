use repolyze_store::path::database_path_from_home;

#[test]
fn database_path_defaults_to_repolyze_db_in_home_directory() {
    let path = database_path_from_home("/tmp/test-home");
    assert_eq!(
        path,
        std::path::PathBuf::from("/tmp/test-home/.repolyze/repolyze.db")
    );
}
