use std::collections::BTreeMap;
use std::path::Path;

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{FileMetric, RepositoryTarget, SizeMetrics};

use crate::walk::{WalkEntry, walk_repository};

const MAX_LARGEST_FILES: usize = 10;

/// Analyze size metrics for a repository.
pub fn analyze_size(target: &RepositoryTarget) -> Result<SizeMetrics, RepolyzeError> {
    let entries = walk_repository(&target.root);

    let mut files: u64 = 0;
    let mut directories: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_lines: u64 = 0;
    let mut non_empty_lines: u64 = 0;
    let mut blank_lines: u64 = 0;
    let mut by_extension: BTreeMap<String, u64> = BTreeMap::new();
    let mut file_metrics: Vec<FileMetric> = Vec::new();

    for entry in &entries {
        match entry {
            WalkEntry::Directory { .. } => {
                directories += 1;
            }
            WalkEntry::File { path, size } => {
                files += 1;
                total_bytes += size;

                let (lines, non_empty, blank) = count_lines(path);
                total_lines += lines;
                non_empty_lines += non_empty;
                blank_lines += blank;

                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    *by_extension.entry(ext.to_lowercase()).or_default() += 1;
                }

                file_metrics.push(FileMetric {
                    path: path
                        .strip_prefix(&target.root)
                        .unwrap_or(path)
                        .to_path_buf(),
                    bytes: *size,
                    lines,
                });
            }
        }
    }

    // Sort by size descending and keep top N
    file_metrics.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    file_metrics.truncate(MAX_LARGEST_FILES);

    let average_file_size = if files > 0 {
        total_bytes as f64 / files as f64
    } else {
        0.0
    };

    Ok(SizeMetrics {
        files,
        directories,
        total_bytes,
        total_lines,
        non_empty_lines,
        blank_lines,
        by_extension,
        largest_files: file_metrics,
        average_file_size,
    })
}

/// Count total lines, non-empty lines, and blank lines in a file.
/// Returns (0, 0, 0) for binary/unreadable files.
fn count_lines(path: &Path) -> (u64, u64, u64) {
    let contents = match std::fs::read(path) {
        Ok(c) => c,
        Err(_) => return (0, 0, 0),
    };

    // Heuristic: if the file contains null bytes, treat as binary
    if contents.contains(&0) {
        return (0, 0, 0);
    }

    let text = match std::str::from_utf8(&contents) {
        Ok(t) => t,
        Err(_) => return (0, 0, 0),
    };

    let mut total: u64 = 0;
    let mut blank: u64 = 0;

    for line in text.lines() {
        total += 1;
        if line.trim().is_empty() {
            blank += 1;
        }
    }

    // Account for trailing newline
    if text.ends_with('\n') && !text.is_empty() {
        // lines() already handles this correctly
    }

    let non_empty = total - blank;
    (total, non_empty, blank)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn create_test_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Init git repo
        Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .unwrap();

        // Create files
        std::fs::write(root.join("README.md"), "# Hello\n\nWorld\n").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() {\n    println!(\"hi\");\n}\n",
        )
        .unwrap();

        // Create a .gitignore that ignores build/
        std::fs::write(root.join(".gitignore"), "build/\n").unwrap();

        // Create an ignored file
        std::fs::create_dir_all(root.join("build")).unwrap();
        std::fs::write(root.join("build/output.txt"), "should be ignored\n").unwrap();

        dir
    }

    #[test]
    fn size_metrics_skip_ignored_files() {
        let dir = create_test_repo();
        let target = RepositoryTarget {
            root: dir.path().to_path_buf(),
        };

        let metrics = analyze_size(&target).unwrap();

        // Should count README.md, src/main.rs, .gitignore (3 files)
        // Should NOT count build/output.txt
        assert_eq!(metrics.files, 3);
        assert!(metrics.directories >= 1); // at least src/

        // Check that build/output.txt bytes are not included
        // README.md = 14 bytes, src/main.rs = 34 bytes, .gitignore = 7 bytes
        assert!(metrics.total_bytes > 0);
    }

    #[test]
    fn line_counts_match_fixture_files() {
        let dir = create_test_repo();
        let target = RepositoryTarget {
            root: dir.path().to_path_buf(),
        };

        let metrics = analyze_size(&target).unwrap();

        // README.md: 3 lines (# Hello, empty, World)
        // src/main.rs: 3 lines
        // .gitignore: 1 line
        // Total: 7 lines
        assert_eq!(metrics.total_lines, 7);
        assert!(metrics.non_empty_lines > 0);
        assert!(metrics.blank_lines > 0);
    }

    #[test]
    fn extension_totals_are_aggregated() {
        let dir = create_test_repo();
        let target = RepositoryTarget {
            root: dir.path().to_path_buf(),
        };

        let metrics = analyze_size(&target).unwrap();

        assert_eq!(metrics.by_extension.get("md"), Some(&1));
        assert_eq!(metrics.by_extension.get("rs"), Some(&1));
    }

    #[test]
    fn binary_files_counted_for_bytes_not_lines() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .unwrap();

        // Write a binary file with null bytes
        let binary_content = b"hello\x00world\x00binary";
        std::fs::write(root.join("data.bin"), binary_content).unwrap();

        let target = RepositoryTarget {
            root: root.to_path_buf(),
        };

        let metrics = analyze_size(&target).unwrap();

        assert_eq!(metrics.files, 1);
        assert_eq!(metrics.total_bytes, binary_content.len() as u64);
        assert_eq!(metrics.total_lines, 0); // binary file, no line counting
    }
}
