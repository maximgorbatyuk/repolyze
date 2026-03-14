use std::collections::{BTreeSet, HashMap};

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{ContributionSummary, ContributorStats, RepositoryTarget};

use crate::backend;
use crate::parse::{ParsedCommit, parse_git_log};

/// Analyze contribution statistics for a repository.
pub fn analyze_contributions(
    target: &RepositoryTarget,
) -> Result<(ContributionSummary, Vec<ParsedCommit>), RepolyzeError> {
    let output = backend::run_git(
        &target.root,
        &["log", "--format=%H%x1f%an%x1f%ae%x1f%aI", "--numstat"],
    )?;

    let commits = parse_git_log(&output)?;
    let summary = aggregate_contributions(&commits);
    Ok((summary, commits))
}

fn aggregate_contributions(commits: &[ParsedCommit]) -> ContributionSummary {
    let mut by_email: HashMap<String, ContributorAccumulator> = HashMap::new();

    for commit in commits {
        let email = commit.author_email.to_lowercase();
        let acc = by_email
            .entry(email)
            .or_insert_with(|| ContributorAccumulator {
                name: commit.author_name.clone(),
                email: commit.author_email.clone(),
                commits: 0,
                lines_added: 0,
                lines_deleted: 0,
                files: BTreeSet::new(),
                dates: BTreeSet::new(),
                timestamps: Vec::new(),
            });

        acc.commits += 1;
        for change in &commit.file_changes {
            acc.lines_added += change.additions;
            acc.lines_deleted += change.deletions;
            acc.files.insert(change.path.clone());
        }

        // Extract date portion for active days
        if let Some(date) = commit.timestamp.split('T').next() {
            acc.dates.insert(date.to_string());
        }
        acc.timestamps.push(commit.timestamp.clone());
    }

    let mut contributors: Vec<ContributorStats> = by_email
        .into_values()
        .map(|acc| {
            let first_commit = acc.timestamps.iter().min().cloned().unwrap_or_default();
            let last_commit = acc.timestamps.iter().max().cloned().unwrap_or_default();

            ContributorStats {
                name: acc.name,
                email: acc.email,
                commits: acc.commits,
                lines_added: acc.lines_added,
                lines_deleted: acc.lines_deleted,
                net_lines: acc.lines_added as i64 - acc.lines_deleted as i64,
                files_touched: acc.files.len() as u64,
                active_days: acc.dates.len() as u64,
                first_commit,
                last_commit,
            }
        })
        .collect();

    // Sort by commits descending
    contributors.sort_by(|a, b| b.commits.cmp(&a.commits));

    let total_commits = commits.len() as u64;

    ContributionSummary {
        contributors,
        total_commits,
    }
}

struct ContributorAccumulator {
    name: String,
    email: String,
    commits: u64,
    lines_added: u64,
    lines_deleted: u64,
    files: BTreeSet<String>,
    dates: BTreeSet<String>,
    timestamps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::FileChange;

    #[test]
    fn contribution_stats_are_aggregated_by_email() {
        let commits = vec![
            ParsedCommit {
                hash: "aaa".to_string(),
                author_name: "Alice".to_string(),
                author_email: "alice@example.com".to_string(),
                timestamp: "2025-01-15T10:00:00+00:00".to_string(),
                file_changes: vec![FileChange {
                    additions: 10,
                    deletions: 0,
                    path: "README.md".to_string(),
                }],
            },
            ParsedCommit {
                hash: "bbb".to_string(),
                author_name: "Bob".to_string(),
                author_email: "bob@example.com".to_string(),
                timestamp: "2025-01-16T14:30:00+00:00".to_string(),
                file_changes: vec![FileChange {
                    additions: 5,
                    deletions: 2,
                    path: "src/lib.rs".to_string(),
                }],
            },
            ParsedCommit {
                hash: "ccc".to_string(),
                author_name: "Alice".to_string(),
                author_email: "alice@example.com".to_string(),
                timestamp: "2025-01-17T09:15:00+00:00".to_string(),
                file_changes: vec![FileChange {
                    additions: 3,
                    deletions: 1,
                    path: "README.md".to_string(),
                }],
            },
        ];

        let summary = aggregate_contributions(&commits);
        assert_eq!(summary.total_commits, 3);
        assert_eq!(summary.contributors.len(), 2);

        // Alice has 2 commits (should be first - sorted by commits desc)
        let alice = &summary.contributors[0];
        assert_eq!(alice.name, "Alice");
        assert_eq!(alice.commits, 2);
        assert_eq!(alice.lines_added, 13);
        assert_eq!(alice.lines_deleted, 1);
        assert_eq!(alice.net_lines, 12);
        assert_eq!(alice.files_touched, 1); // only README.md
        assert_eq!(alice.active_days, 2);

        // Bob has 1 commit
        let bob = &summary.contributors[1];
        assert_eq!(bob.name, "Bob");
        assert_eq!(bob.commits, 1);
        assert_eq!(bob.lines_added, 5);
        assert_eq!(bob.lines_deleted, 2);
        assert_eq!(bob.files_touched, 1);
    }
}
