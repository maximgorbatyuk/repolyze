use repolyze_core::error::RepolyzeError;

/// A single parsed commit from git log output.
#[derive(Debug, Clone)]
pub struct ParsedCommit {
    pub hash: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
    pub file_changes: Vec<FileChange>,
}

/// Per-file change stats from --numstat output.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub additions: u64,
    pub deletions: u64,
    pub path: String,
}

/// Parses the output of:
/// `git log --format=%H%x1f%an%x1f%ae%x1f%aI --numstat`
pub fn parse_git_log(output: &str) -> Result<Vec<ParsedCommit>, RepolyzeError> {
    let mut commits = Vec::new();
    let mut current: Option<ParsedCommit> = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try to parse as a commit header (fields separated by \x1f)
        let fields: Vec<&str> = line.split('\x1f').collect();
        if fields.len() == 4 {
            if let Some(commit) = current.take() {
                commits.push(commit);
            }
            current = Some(ParsedCommit {
                hash: fields[0].to_string(),
                author_name: fields[1].to_string(),
                author_email: fields[2].to_string(),
                timestamp: fields[3].to_string(),
                file_changes: Vec::new(),
            });
            continue;
        }

        // Try to parse as a numstat line: additions\tdeletions\tpath
        if let Some(ref mut commit) = current {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 3 {
                // Binary files show "-" for additions/deletions
                let additions = parts[0].parse::<u64>().unwrap_or(0);
                let deletions = parts[1].parse::<u64>().unwrap_or(0);
                commit.file_changes.push(FileChange {
                    additions,
                    deletions,
                    path: parts[2].to_string(),
                });
            }
        }
    }

    if let Some(commit) = current.take() {
        commits.push(commit);
    }

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_commit() {
        let output = "abc123\x1fAlice\x1falice@example.com\x1f2025-01-15T10:00:00+00:00\n\
                       3\t0\tREADME.md\n";
        let commits = parse_git_log(output).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].author_name, "Alice");
        assert_eq!(commits[0].file_changes.len(), 1);
        assert_eq!(commits[0].file_changes[0].additions, 3);
    }

    #[test]
    fn parses_multiple_commits() {
        let output = "abc123\x1fAlice\x1falice@example.com\x1f2025-01-15T10:00:00+00:00\n\
                       3\t0\tREADME.md\n\
                       \n\
                       def456\x1fBob\x1fbob@example.com\x1f2025-01-16T14:30:00+00:00\n\
                       1\t0\tsrc/lib.rs\n";
        let commits = parse_git_log(output).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].author_name, "Alice");
        assert_eq!(commits[1].author_name, "Bob");
    }

    #[test]
    fn handles_binary_file_numstat() {
        let output = "abc123\x1fAlice\x1falice@example.com\x1f2025-01-15T10:00:00+00:00\n\
                       -\t-\timage.png\n";
        let commits = parse_git_log(output).unwrap();
        assert_eq!(commits[0].file_changes[0].additions, 0);
        assert_eq!(commits[0].file_changes[0].deletions, 0);
    }
}
