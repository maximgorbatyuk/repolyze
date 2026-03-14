use std::path::Path;

use ignore::WalkBuilder;

/// Entry from a .gitignore-aware walk of a repository.
#[derive(Debug)]
pub enum WalkEntry {
    File { path: std::path::PathBuf, size: u64 },
    Directory { path: std::path::PathBuf },
}

/// Walk a repository directory, respecting .gitignore rules.
/// Excludes the .git directory itself.
pub fn walk_repository(root: &Path) -> Vec<WalkEntry> {
    let mut entries = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(false) // include hidden files (but .gitignore still respected)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path().to_path_buf();

        // Skip the .git directory
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        // Skip the root directory itself
        if path == root {
            continue;
        }

        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            entries.push(WalkEntry::Directory { path });
        } else if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            entries.push(WalkEntry::File { path, size });
        }
    }

    entries
}
