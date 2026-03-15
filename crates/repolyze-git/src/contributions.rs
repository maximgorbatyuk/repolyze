use std::collections::{BTreeSet, HashMap};

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{
    ContributionSummary, ContributorActivityStats, ContributorStats, RepositoryTarget,
};

use crate::activity::parse_hour_and_weekday;
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
                weekday_commits: [0; 7],
                hour_commits: [0; 24],
                active_dates_by_weekday: std::array::from_fn(|_| BTreeSet::new()),
                active_hour_buckets: BTreeSet::new(),
                active_hour_buckets_by_hour: std::array::from_fn(|_| BTreeSet::new()),
            });

        acc.commits += 1;
        for change in &commit.file_changes {
            acc.lines_added += change.additions;
            acc.lines_deleted += change.deletions;
            acc.files.insert(change.path.clone());
        }

        // Extract date portion for active days
        let date = commit
            .timestamp
            .split('T')
            .next()
            .unwrap_or_default()
            .to_string();
        if !date.is_empty() {
            acc.dates.insert(date.clone());
        }
        acc.timestamps.push(commit.timestamp.clone());

        // Activity facts
        if let Some((hour, weekday)) = parse_hour_and_weekday(&commit.timestamp) {
            acc.weekday_commits[weekday] += 1;
            acc.hour_commits[hour] += 1;
            acc.active_dates_by_weekday[weekday].insert(date.clone());
            let hour_bucket = format!("{date}:{hour}");
            acc.active_hour_buckets.insert(hour_bucket.clone());
            acc.active_hour_buckets_by_hour[hour].insert(hour_bucket);
        }
    }

    let mut contributors: Vec<ContributorStats> = Vec::new();
    let mut activity_by_contributor: Vec<ContributorActivityStats> = Vec::new();

    for acc in by_email.into_values() {
        let first_commit = acc.timestamps.iter().min().cloned().unwrap_or_default();
        let last_commit = acc.timestamps.iter().max().cloned().unwrap_or_default();
        let email_lower = acc.email.to_lowercase();

        contributors.push(ContributorStats {
            name: acc.name,
            email: email_lower.clone(),
            commits: acc.commits,
            lines_added: acc.lines_added,
            lines_deleted: acc.lines_deleted,
            net_lines: acc.lines_added as i64 - acc.lines_deleted as i64,
            files_touched: acc.files.len() as u64,
            active_days: acc.dates.len() as u64,
            first_commit,
            last_commit,
        });

        activity_by_contributor.push(ContributorActivityStats {
            email: email_lower,
            weekday_commits: acc.weekday_commits,
            hour_commits: acc.hour_commits,
            active_dates: acc.dates,
            active_dates_by_weekday: acc.active_dates_by_weekday,
            active_hour_buckets: acc.active_hour_buckets,
            active_hour_buckets_by_hour: acc.active_hour_buckets_by_hour,
        });
    }

    // Sort by commits descending
    contributors.sort_by(|a, b| b.commits.cmp(&a.commits));
    activity_by_contributor.sort_by(|a, b| {
        let a_total: u32 = a.weekday_commits.iter().sum();
        let b_total: u32 = b.weekday_commits.iter().sum();
        b_total.cmp(&a_total).then(a.email.cmp(&b.email))
    });

    let total_commits = commits.len() as u64;

    ContributionSummary {
        contributors,
        activity_by_contributor,
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
    weekday_commits: [u32; 7],
    hour_commits: [u32; 24],
    active_dates_by_weekday: [BTreeSet<String>; 7],
    active_hour_buckets: BTreeSet<String>,
    active_hour_buckets_by_hour: [BTreeSet<String>; 24],
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

    fn make_commit(
        email: &str,
        timestamp: &str,
        additions: u64,
        deletions: u64,
        path: &str,
    ) -> ParsedCommit {
        ParsedCommit {
            hash: format!("{email}-{timestamp}"),
            author_name: email.split('@').next().unwrap().to_string(),
            author_email: email.to_string(),
            timestamp: timestamp.to_string(),
            file_changes: vec![FileChange {
                additions,
                deletions,
                path: path.to_string(),
            }],
        }
    }

    #[test]
    fn contribution_summary_tracks_per_user_weekday_hour_and_active_buckets() {
        // 2025-01-13 is Monday (weekday 0)
        // 2025-01-15 is Wednesday (weekday 2)
        let commits = vec![
            make_commit(
                "alice@example.com",
                "2025-01-13T10:00:00+00:00",
                5,
                1,
                "src/a.rs",
            ),
            make_commit(
                "alice@example.com",
                "2025-01-13T10:45:00+00:00",
                3,
                0,
                "src/b.rs",
            ),
            make_commit(
                "alice@example.com",
                "2025-01-15T14:00:00+00:00",
                2,
                2,
                "src/c.rs",
            ),
        ];

        let summary = aggregate_contributions(&commits);
        let alice = summary
            .activity_by_contributor
            .iter()
            .find(|row| row.email == "alice@example.com")
            .unwrap();

        // weekday 0 = Monday: 2 commits, weekday 2 = Wednesday: 1 commit
        assert_eq!(alice.weekday_commits[0], 2);
        assert_eq!(alice.weekday_commits[2], 1);
        // hour 10: 2 commits, hour 14: 1 commit
        assert_eq!(alice.hour_commits[10], 2);
        assert_eq!(alice.hour_commits[14], 1);
        // 2 distinct active dates
        assert_eq!(alice.active_dates.len(), 2);
        // weekday 0 (Monday) has 1 active date, weekday 2 (Wednesday) has 1
        assert_eq!(alice.active_dates_by_weekday[0].len(), 1);
        assert_eq!(alice.active_dates_by_weekday[2].len(), 1);
        // 2 distinct hour buckets: "2025-01-13:10" and "2025-01-15:14"
        assert_eq!(alice.active_hour_buckets.len(), 2);
    }
}
