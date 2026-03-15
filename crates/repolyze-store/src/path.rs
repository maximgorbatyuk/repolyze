use std::path::PathBuf;

/// Database path for release builds: `~/.repolyze/repolyze.db`
pub fn database_path_from_home(home: &str) -> PathBuf {
    PathBuf::from(home).join(".repolyze").join("repolyze.db")
}

/// Database path for dev builds: `<binary_dir>/repolyze-dev.db`
pub fn database_path_for_dev() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| std::io::Error::other("no parent for exe"))?;
    Ok(dir.join("repolyze-dev.db"))
}

/// Picks the right database path based on build profile.
/// - Debug builds → next to the binary (`target/debug/repolyze-dev.db`)
/// - Release builds → `~/.repolyze/repolyze.db`
pub fn resolve_database_path() -> std::io::Result<PathBuf> {
    if cfg!(debug_assertions) {
        database_path_for_dev()
    } else {
        let home = std::env::var("HOME").map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("HOME not set: {e}"))
        })?;
        Ok(database_path_from_home(&home))
    }
}
