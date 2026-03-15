use std::collections::{BTreeSet, HashMap};

use crate::model::{RepositoryAnalysis, UserActivityRow, UsersContributionRow};

const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

pub fn build_users_contribution_rows(repos: &[RepositoryAnalysis]) -> Vec<UsersContributionRow> {
    let merged = merge_activity_by_email(repos);
    let mut rows: Vec<UsersContributionRow> = merged
        .into_iter()
        .map(|(email, m)| {
            let commits = m.commits;
            let lines_modified = m.lines_added + m.lines_deleted;
            let lines_per_commit = if commits > 0 {
                lines_modified as f64 / commits as f64
            } else {
                0.0
            };
            let most_active_weekday_idx = m
                .weekday_commits
                .iter()
                .enumerate()
                .max_by_key(|(_, c)| *c)
                .map(|(i, _)| i)
                .unwrap_or(0);
            UsersContributionRow {
                email,
                commits,
                lines_modified,
                lines_per_commit,
                files_touched: m.files_touched,
                most_active_week_day: WEEKDAY_NAMES[most_active_weekday_idx].to_string(),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.commits.cmp(&a.commits).then(a.email.cmp(&b.email)));
    rows
}

pub fn build_user_activity_rows(repos: &[RepositoryAnalysis]) -> Vec<UserActivityRow> {
    let merged = merge_activity_by_email(repos);
    let mut rows: Vec<UserActivityRow> = merged
        .into_iter()
        .map(|(email, m)| {
            let most_active_weekday_idx = m
                .weekday_commits
                .iter()
                .enumerate()
                .max_by_key(|(_, c)| *c)
                .map(|(i, _)| i)
                .unwrap_or(0);

            let most_active_hour_idx = m
                .hour_commits
                .iter()
                .enumerate()
                .max_by_key(|(_, c)| *c)
                .map(|(i, _)| i)
                .unwrap_or(0);

            let total_active_dates = m.active_dates.len() as f64;
            let total_commits: u32 = m.weekday_commits.iter().sum();

            let average_commits_per_day = if total_active_dates > 0.0 {
                total_commits as f64 / total_active_dates
            } else {
                0.0
            };

            let most_active_weekday_dates =
                m.active_dates_by_weekday[most_active_weekday_idx].len() as f64;
            let most_active_weekday_commits = m.weekday_commits[most_active_weekday_idx] as f64;
            let average_commits_per_day_in_most_active_day = if most_active_weekday_dates > 0.0 {
                most_active_weekday_commits / most_active_weekday_dates
            } else {
                0.0
            };

            let total_hour_buckets = m.active_hour_buckets.len() as f64;
            let average_commits_per_hour = if total_hour_buckets > 0.0 {
                total_commits as f64 / total_hour_buckets
            } else {
                0.0
            };

            let most_active_hour_buckets =
                m.active_hour_buckets_by_hour[most_active_hour_idx].len() as f64;
            let most_active_hour_commits = m.hour_commits[most_active_hour_idx] as f64;
            let average_commits_per_hour_in_most_active_hour = if most_active_hour_buckets > 0.0 {
                most_active_hour_commits / most_active_hour_buckets
            } else {
                0.0
            };

            UserActivityRow {
                email,
                most_active_week_day: WEEKDAY_NAMES[most_active_weekday_idx].to_string(),
                average_commits_per_day_in_most_active_day,
                average_commits_per_day,
                average_commits_per_hour_in_most_active_hour,
                average_commits_per_hour,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.average_commits_per_day
            .partial_cmp(&a.average_commits_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.email.cmp(&b.email))
    });
    rows
}

struct MergedContributor {
    commits: u64,
    lines_added: u64,
    lines_deleted: u64,
    files_touched: u64,
    weekday_commits: [u32; 7],
    hour_commits: [u32; 24],
    active_dates: BTreeSet<String>,
    active_dates_by_weekday: [BTreeSet<String>; 7],
    active_hour_buckets: BTreeSet<String>,
    active_hour_buckets_by_hour: [BTreeSet<String>; 24],
}

fn merge_activity_by_email(repos: &[RepositoryAnalysis]) -> HashMap<String, MergedContributor> {
    let mut map: HashMap<String, MergedContributor> = HashMap::new();

    for repo in repos {
        // Merge contributor stats
        for cs in &repo.contributions.contributors {
            let email = cs.email.to_lowercase();
            let entry = map.entry(email).or_insert_with(|| MergedContributor {
                commits: 0,
                lines_added: 0,
                lines_deleted: 0,
                files_touched: 0,
                weekday_commits: [0; 7],
                hour_commits: [0; 24],
                active_dates: BTreeSet::new(),
                active_dates_by_weekday: std::array::from_fn(|_| BTreeSet::new()),
                active_hour_buckets: BTreeSet::new(),
                active_hour_buckets_by_hour: std::array::from_fn(|_| BTreeSet::new()),
            });
            entry.commits += cs.commits;
            entry.lines_added += cs.lines_added;
            entry.lines_deleted += cs.lines_deleted;
            entry.files_touched += cs.files_touched;
        }

        // Merge activity facts
        for act in &repo.contributions.activity_by_contributor {
            let email = act.email.to_lowercase();
            let entry = map.entry(email).or_insert_with(|| MergedContributor {
                commits: 0,
                lines_added: 0,
                lines_deleted: 0,
                files_touched: 0,
                weekday_commits: [0; 7],
                hour_commits: [0; 24],
                active_dates: BTreeSet::new(),
                active_dates_by_weekday: std::array::from_fn(|_| BTreeSet::new()),
                active_hour_buckets: BTreeSet::new(),
                active_hour_buckets_by_hour: std::array::from_fn(|_| BTreeSet::new()),
            });
            for i in 0..7 {
                entry.weekday_commits[i] += act.weekday_commits[i];
                entry.active_dates_by_weekday[i]
                    .extend(act.active_dates_by_weekday[i].iter().cloned());
            }
            for i in 0..24 {
                entry.hour_commits[i] += act.hour_commits[i];
                entry.active_hour_buckets_by_hour[i]
                    .extend(act.active_hour_buckets_by_hour[i].iter().cloned());
            }
            entry.active_dates.extend(act.active_dates.iter().cloned());
            entry
                .active_hour_buckets
                .extend(act.active_hour_buckets.iter().cloned());
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ActivitySummary, ContributionSummary, ContributorActivityStats, ContributorStats,
        RepositoryTarget, SizeMetrics,
    };

    fn make_repo(
        name: &str,
        contributors: Vec<ContributorStats>,
        activity: Vec<ContributorActivityStats>,
        total_commits: u64,
    ) -> RepositoryAnalysis {
        RepositoryAnalysis {
            repository: RepositoryTarget {
                root: format!("/tmp/{name}").into(),
            },
            contributions: ContributionSummary {
                contributors,
                activity_by_contributor: activity,
                total_commits,
            },
            activity: ActivitySummary::default(),
            size: SizeMetrics::default(),
        }
    }

    fn make_contributor(
        email: &str,
        commits: u64,
        added: u64,
        deleted: u64,
        files: u64,
    ) -> ContributorStats {
        ContributorStats {
            name: email.split('@').next().unwrap().to_string(),
            email: email.to_string(),
            commits,
            lines_added: added,
            lines_deleted: deleted,
            net_lines: added as i64 - deleted as i64,
            files_touched: files,
            active_days: 1,
            first_commit: "2025-01-01".to_string(),
            last_commit: "2025-01-15".to_string(),
        }
    }

    fn make_activity(
        email: &str,
        weekday_commits: [u32; 7],
        hour_commits: [u32; 24],
        active_dates: &[&str],
    ) -> ContributorActivityStats {
        let dates: BTreeSet<String> = active_dates.iter().map(|s| s.to_string()).collect();
        let mut active_dates_by_weekday: [BTreeSet<String>; 7] =
            std::array::from_fn(|_| BTreeSet::new());
        // Put all dates into weekday 0 for simplicity in tests
        for (i, &count) in weekday_commits.iter().enumerate() {
            if count > 0 {
                for d in &dates {
                    active_dates_by_weekday[i].insert(d.clone());
                }
            }
        }
        let mut active_hour_buckets = BTreeSet::new();
        let mut active_hour_buckets_by_hour: [BTreeSet<String>; 24] =
            std::array::from_fn(|_| BTreeSet::new());
        for (h, &count) in hour_commits.iter().enumerate() {
            if count > 0 {
                for d in &dates {
                    let bucket = format!("{d}:{h}");
                    active_hour_buckets.insert(bucket.clone());
                    active_hour_buckets_by_hour[h].insert(bucket);
                }
            }
        }
        ContributorActivityStats {
            email: email.to_lowercase(),
            weekday_commits,
            hour_commits,
            active_dates: dates,
            active_dates_by_weekday,
            active_hour_buckets,
            active_hour_buckets_by_hour,
        }
    }

    fn make_report_with_shared_contributor() -> Vec<RepositoryAnalysis> {
        let mut weekday = [0u32; 7];
        weekday[0] = 3; // Monday
        let mut hour = [0u32; 24];
        hour[10] = 3;

        let repo_a = make_repo(
            "repo-a",
            vec![make_contributor("alice@example.com", 3, 30, 5, 3)],
            vec![make_activity(
                "alice@example.com",
                weekday,
                hour,
                &["2025-01-13", "2025-01-14"],
            )],
            3,
        );

        let mut weekday2 = [0u32; 7];
        weekday2[0] = 2; // Monday
        let mut hour2 = [0u32; 24];
        hour2[10] = 2;

        let repo_b = make_repo(
            "repo-b",
            vec![make_contributor("alice@example.com", 2, 10, 2, 1)],
            vec![make_activity(
                "alice@example.com",
                weekday2,
                hour2,
                &["2025-01-13", "2025-01-15"],
            )],
            2,
        );

        vec![repo_a, repo_b]
    }

    #[test]
    fn build_users_contribution_rows_merges_by_email_and_sorts_by_commits() {
        let repos = make_report_with_shared_contributor();

        let rows = build_users_contribution_rows(&repos);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].email, "alice@example.com");
        assert_eq!(rows[0].commits, 5);
        assert_eq!(rows[0].lines_modified, 47); // 40 added + 7 deleted
        assert_eq!(rows[0].files_touched, 4);
        assert_eq!(rows[0].most_active_week_day, "Monday");
    }

    #[test]
    fn build_user_activity_rows_dedupes_dates_across_repositories() {
        // Alice commits on 2025-01-13 in both repos — should count as 1 active date
        // Total: 3 distinct dates, 5 commits → avg = 5/3
        let repos = make_report_with_shared_contributor();

        let rows = build_user_activity_rows(&repos);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].email, "alice@example.com");
        // 3 distinct dates, 5 commits
        let expected_avg = 5.0 / 3.0;
        assert!((rows[0].average_commits_per_day - expected_avg).abs() < 0.01);
    }
}
