use std::path::PathBuf;

pub fn database_path_from_home(home: &str) -> PathBuf {
    PathBuf::from(home).join(".repolyze").join("repolyze.db")
}
