use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::date_util;
use crate::model::{
    ContributionRow, DAYS_IN_WEEK, HEATMAP_MAX_WEEKS, HeatmapData, RepositoryAnalysis,
    UserActivityRow, UserEffortData,
};
use crate::settings::Settings;

const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

fn most_active_index(arr: &[u32]) -> Option<usize> {
    arr.iter()
        .enumerate()
        .max_by_key(|(_, c)| *c)
        .filter(|(_, c)| **c > 0)
        .map(|(i, _)| i)
}

pub fn build_contribution_rows(
    repos: &[RepositoryAnalysis],
    settings: &Settings,
) -> Vec<ContributionRow> {
    let merged = merge_activity_by_email(repos, settings);
    let mut rows: Vec<ContributionRow> = merged
        .into_iter()
        .map(|(identifier, m)| {
            let commits = m.commits;
            let lines_modified = m.lines_added.saturating_add(m.lines_deleted);
            let lines_per_commit = if commits > 0 {
                lines_modified as f64 / commits as f64
            } else {
                0.0
            };
            ContributionRow {
                identifier,
                commits,
                lines_modified,
                lines_per_commit,
                files_touched: m.files_touched,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.commits
            .cmp(&a.commits)
            .then(a.identifier.cmp(&b.identifier))
    });
    rows
}

pub fn build_user_activity_rows(
    repos: &[RepositoryAnalysis],
    settings: &Settings,
) -> Vec<UserActivityRow> {
    let merged = merge_activity_by_email(repos, settings);
    let mut merged: Vec<_> = merged.into_iter().collect();
    merged.sort_by(|a, b| b.1.commits.cmp(&a.1.commits).then(a.0.cmp(&b.0)));

    merged
        .into_iter()
        .map(|(identifier, m)| {
            let most_active_weekday_idx = most_active_index(&m.weekday_commits);
            let most_active_hour_idx = most_active_index(&m.hour_commits);

            let total_active_dates = m.active_dates.len() as f64;
            let total_commits = m.commits;

            let average_commits_per_day = if total_active_dates > 0.0 {
                total_commits as f64 / total_active_dates
            } else {
                0.0
            };

            let average_commits_per_day_in_most_active_day = if let Some(weekday_idx) =
                most_active_weekday_idx
            {
                let most_active_weekday_dates = m.active_dates_by_weekday[weekday_idx].len() as f64;
                let most_active_weekday_commits = m.weekday_commits[weekday_idx] as f64;
                if most_active_weekday_dates > 0.0 {
                    most_active_weekday_commits / most_active_weekday_dates
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let total_hour_buckets = m.active_hour_buckets.len() as f64;
            let average_commits_per_hour = if total_hour_buckets > 0.0 {
                total_commits as f64 / total_hour_buckets
            } else {
                0.0
            };

            let average_commits_per_hour_in_most_active_hour = if let Some(hour_idx) =
                most_active_hour_idx
            {
                let most_active_hour_buckets = m.active_hour_buckets_by_hour[hour_idx].len() as f64;
                let most_active_hour_commits = m.hour_commits[hour_idx] as f64;
                if most_active_hour_buckets > 0.0 {
                    most_active_hour_commits / most_active_hour_buckets
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let most_active_week_day = most_active_weekday_idx
                .map(|i| WEEKDAY_NAMES[i].to_string())
                .unwrap_or_else(|| "N/A".to_string());

            UserActivityRow {
                identifier,
                most_active_week_day,
                average_commits_per_day_in_most_active_day,
                average_commits_per_day,
                average_commits_per_hour_in_most_active_hour,
                average_commits_per_hour,
            }
        })
        .collect()
}

struct MergedContributor {
    name: String,
    commits: u64,
    lines_added: u64,
    lines_deleted: u64,
    files_touched: u64,
    file_extensions: BTreeMap<String, u64>,
    weekday_commits: [u32; 7],
    hour_commits: [u32; 24],
    active_dates: BTreeSet<String>,
    active_dates_by_weekday: [BTreeSet<String>; 7],
    active_hour_buckets: BTreeSet<String>,
    active_hour_buckets_by_hour: [BTreeSet<String>; 24],
    commits_by_date: BTreeMap<String, u32>,
    first_commit: String,
    last_commit: String,
}

impl Default for MergedContributor {
    fn default() -> Self {
        Self {
            name: String::new(),
            commits: 0,
            lines_added: 0,
            lines_deleted: 0,
            files_touched: 0,
            file_extensions: BTreeMap::new(),
            weekday_commits: [0; 7],
            hour_commits: [0; 24],
            active_dates: BTreeSet::new(),
            active_dates_by_weekday: std::array::from_fn(|_| BTreeSet::new()),
            active_hour_buckets: BTreeSet::new(),
            active_hour_buckets_by_hour: std::array::from_fn(|_| BTreeSet::new()),
            commits_by_date: BTreeMap::new(),
            first_commit: String::new(),
            last_commit: String::new(),
        }
    }
}

fn merge_activity_by_email(
    repos: &[RepositoryAnalysis],
    settings: &Settings,
) -> HashMap<String, MergedContributor> {
    let mut map: HashMap<String, MergedContributor> = HashMap::new();

    for repo in repos {
        for cs in &repo.contributions.contributors {
            let key = settings.canonical_key(&cs.email);
            let entry = map.entry(key).or_default();
            if entry.name.is_empty() {
                entry.name = cs.name.clone();
            }
            entry.commits += cs.commits;
            entry.lines_added += cs.lines_added;
            entry.lines_deleted += cs.lines_deleted;
            entry.files_touched += cs.files_touched;
            for (ext, count) in &cs.file_extensions {
                *entry.file_extensions.entry(ext.clone()).or_insert(0) += count;
            }
            if entry.first_commit.is_empty() || cs.first_commit < entry.first_commit {
                entry.first_commit = cs.first_commit.clone();
            }
            if entry.last_commit.is_empty() || cs.last_commit > entry.last_commit {
                entry.last_commit = cs.last_commit.clone();
            }
        }

        for act in &repo.contributions.activity_by_contributor {
            let key = settings.canonical_key(&act.email);
            let entry = map.entry(key).or_default();
            for i in 0..7 {
                entry.weekday_commits[i] =
                    entry.weekday_commits[i].saturating_add(act.weekday_commits[i]);
                entry.active_dates_by_weekday[i]
                    .extend(act.active_dates_by_weekday[i].iter().cloned());
            }
            for i in 0..24 {
                entry.hour_commits[i] = entry.hour_commits[i].saturating_add(act.hour_commits[i]);
                entry.active_hour_buckets_by_hour[i]
                    .extend(act.active_hour_buckets_by_hour[i].iter().cloned());
            }
            entry.active_dates.extend(act.active_dates.iter().cloned());
            entry
                .active_hour_buckets
                .extend(act.active_hour_buckets.iter().cloned());
            for (date, count) in &act.commits_by_date {
                *entry.commits_by_date.entry(date.clone()).or_insert(0) += count;
            }
        }
    }

    map
}

#[derive(Debug, Clone)]
pub struct RepoComparisonRow {
    pub name: String,
    pub total_commits: u64,
    pub active_days: usize,
    pub commits_per_day: f64,
    pub weekday_commits_per_day: [f64; 7],
}

pub fn build_repo_comparison(repos: &[RepositoryAnalysis]) -> Vec<RepoComparisonRow> {
    repos
        .iter()
        .map(|repo| {
            let name = repo.repository.display_name();

            // Union active_dates across all contributors for repo-wide active days
            let mut all_dates: BTreeSet<String> = BTreeSet::new();
            let mut weekday_dates: [BTreeSet<String>; 7] = std::array::from_fn(|_| BTreeSet::new());
            for act in &repo.contributions.activity_by_contributor {
                all_dates.extend(act.active_dates.iter().cloned());
                for (wd_set, act_set) in weekday_dates
                    .iter_mut()
                    .zip(act.active_dates_by_weekday.iter())
                {
                    wd_set.extend(act_set.iter().cloned());
                }
            }

            let active_days = all_dates.len();
            let total_commits = repo.contributions.total_commits;
            let commits_per_day = if active_days > 0 {
                total_commits as f64 / active_days as f64
            } else {
                0.0
            };

            let mut weekday_commits_per_day = [0.0f64; 7];
            for i in 0..7 {
                let wd_days = weekday_dates[i].len();
                if wd_days > 0 {
                    weekday_commits_per_day[i] =
                        repo.activity.by_weekday[i] as f64 / wd_days as f64;
                }
            }

            RepoComparisonRow {
                name,
                total_commits,
                active_days,
                commits_per_day,
                weekday_commits_per_day,
            }
        })
        .collect()
}

const TOP_EXTENSIONS_LIMIT: usize = 3;

fn least_active_index(arr: &[u32]) -> Option<usize> {
    arr.iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .min_by_key(|(_, c)| *c)
        .map(|(i, _)| i)
}

/// Returns deduplicated `(identifier, name)` sorted by commit count descending.
/// When settings map emails to a user name, the identifier is that name.
pub fn get_contributor_emails(
    repos: &[RepositoryAnalysis],
    settings: &Settings,
) -> Vec<(String, String)> {
    let merged = merge_activity_by_email(repos, settings);
    let mut entries: Vec<(String, String, u64)> = merged
        .into_iter()
        .map(|(key, m)| (key, m.name, m.commits))
        .collect();
    entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    entries.into_iter().map(|(e, n, _)| (e, n)).collect()
}

pub fn build_user_effort_data(
    repos: &[RepositoryAnalysis],
    identifier: &str,
    settings: &Settings,
) -> Option<UserEffortData> {
    let merged = merge_activity_by_email(repos, settings);
    // Look up by the identifier as-is first (handles canonical names from settings),
    // then fall back to lowercased email lookup for backward compat.
    // Capture the actual map key so we preserve its original casing.
    let (display_identifier, m) = if let Some(m) = merged.get(identifier) {
        (identifier.to_string(), m)
    } else {
        let lower = identifier.to_lowercase();
        let m = merged.get(&lower)?;
        (lower, m)
    };

    let most_active_weekday_idx = most_active_index(&m.weekday_commits);
    let least_active_weekday_idx = least_active_index(&m.weekday_commits);
    let total_active_dates = m.active_dates.len() as f64;

    let average_commits_per_day = if total_active_dates > 0.0 {
        m.commits as f64 / total_active_dates
    } else {
        0.0
    };

    let most_active_weekday_cpd = most_active_weekday_idx
        .map(|idx| {
            let days = m.active_dates_by_weekday[idx].len() as f64;
            if days > 0.0 {
                m.weekday_commits[idx] as f64 / days
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    let least_active_weekday_cpd = least_active_weekday_idx
        .map(|idx| {
            let days = m.active_dates_by_weekday[idx].len() as f64;
            if days > 0.0 {
                m.weekday_commits[idx] as f64 / days
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    let avg_files_per_commit = if m.commits > 0 {
        m.files_touched as f64 / m.commits as f64
    } else {
        0.0
    };

    let avg_files_per_day = if total_active_dates > 0.0 {
        m.files_touched as f64 / total_active_dates
    } else {
        0.0
    };

    let lines_modified = m.lines_added.saturating_add(m.lines_deleted);
    let avg_lines_per_commit = if m.commits > 0 {
        lines_modified as f64 / m.commits as f64
    } else {
        0.0
    };
    let avg_lines_per_day = if total_active_dates > 0.0 {
        lines_modified as f64 / total_active_dates
    } else {
        0.0
    };

    let mut ext_vec: Vec<(String, u64)> = m
        .file_extensions
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    ext_vec.sort_by(|a, b| b.1.cmp(&a.1));
    ext_vec.truncate(TOP_EXTENSIONS_LIMIT);

    Some(UserEffortData {
        name: m.name.clone(),
        identifier: display_identifier,
        first_commit: m.first_commit.clone(),
        last_commit: m.last_commit.clone(),
        most_active_weekday: most_active_weekday_idx
            .map(|i| WEEKDAY_NAMES[i].to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        most_active_weekday_commits_per_day: most_active_weekday_cpd,
        average_commits_per_day,
        least_active_weekday: least_active_weekday_idx
            .map(|i| WEEKDAY_NAMES[i].to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        least_active_weekday_commits_per_day: least_active_weekday_cpd,
        avg_files_per_commit,
        avg_files_per_day,
        avg_lines_per_commit,
        avg_lines_per_day,
        top_extensions: ext_vec,
    })
}

pub fn build_heatmap_data(
    repos: &[RepositoryAnalysis],
    filter_key: Option<&str>,
    reference_date: &str,
    settings: &Settings,
) -> HeatmapData {
    // Resolve the filter to a set of matching emails
    let allowed_emails: Option<Vec<String>> = filter_key.map(|key| settings.emails_for_key(key));

    // Aggregate commits_by_date across all contributors (optionally filtered)
    let mut aggregated: BTreeMap<String, u32> = BTreeMap::new();
    for repo in repos {
        for act in &repo.contributions.activity_by_contributor {
            if let Some(ref emails) = allowed_emails
                && !emails.contains(&act.email.to_lowercase())
            {
                continue;
            }
            for (date, count) in &act.commits_by_date {
                *aggregated.entry(date.clone()).or_insert(0) += count;
            }
        }
    }

    // Compute start_date = reference_date - 52*7 days, snapped to Monday
    let end_date = reference_date.to_string();
    let raw_start = date_util::add_days(reference_date, -(52 * 7));
    let (sy, sm, sd) = date_util::parse_ymd(&raw_start).unwrap_or((1970, 1, 1));
    let start_dow = date_util::day_of_week(sy, sm, sd);
    let start_date = if start_dow == 0 {
        raw_start
    } else {
        date_util::add_days(&raw_start, -(start_dow as i32))
    };

    // Calculate week_count
    let (ey, em, ed) = date_util::parse_ymd(reference_date).unwrap_or((1970, 1, 1));
    let total_days = {
        let start_jdn = {
            let (y, m, d) = date_util::parse_ymd(&start_date).unwrap_or((1970, 1, 1));
            date_util::to_jdn(y, m, d)
        };
        let end_jdn = date_util::to_jdn(ey, em, ed);
        (end_jdn - start_jdn + 1) as usize
    };
    let week_count = total_days.div_ceil(DAYS_IN_WEEK).min(HEATMAP_MAX_WEEKS);

    // Fill grid
    let mut grid = [[0u32; HEATMAP_MAX_WEEKS]; DAYS_IN_WEEK];
    let mut max_count = 0u32;
    let mut current = start_date.clone();
    for week_col in 0..week_count {
        for weekday_row in &mut grid {
            if current > end_date {
                break;
            }
            let count = aggregated.get(&current).copied().unwrap_or(0);
            weekday_row[week_col] = count;
            if count > max_count {
                max_count = count;
            }
            current = date_util::add_days(&current, 1);
        }
    }

    // Build month labels (where a new month starts)
    let mut month_labels: Vec<(usize, String)> = Vec::new();
    let mut last_month = 0u32;
    let mut day_cursor = start_date.clone();
    for week_col in 0..week_count {
        if let Some((_, m, _)) = date_util::parse_ymd(&day_cursor)
            && m != last_month
        {
            month_labels.push((week_col, date_util::month_abbrev(m).to_string()));
            last_month = m;
        }
        day_cursor = date_util::add_days(&day_cursor, 7);
    }

    HeatmapData {
        start_date,
        end_date,
        grid,
        week_count,
        max_count,
        month_labels,
    }
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
            repository: RepositoryTarget::Local {
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
            file_extensions: BTreeMap::new(),
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
        let mut commits_by_date = BTreeMap::new();
        for d in &dates {
            commits_by_date.insert(d.clone(), 1);
        }
        ContributorActivityStats {
            email: email.to_lowercase(),
            weekday_commits,
            hour_commits,
            active_dates: dates,
            active_dates_by_weekday,
            active_hour_buckets,
            active_hour_buckets_by_hour,
            commits_by_date,
        }
    }

    fn make_report_with_shared_contributor() -> Vec<RepositoryAnalysis> {
        let mut weekday = [0u32; 7];
        weekday[0] = 3; // Monday
        let mut hour = [0u32; 24];
        hour[10] = 3;

        let mut c_a = make_contributor("alice@example.com", 3, 30, 5, 3);
        c_a.file_extensions = BTreeMap::from([("rs".to_string(), 2), ("md".to_string(), 1)]);

        let repo_a = make_repo(
            "repo-a",
            vec![c_a],
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

        let mut c_b = make_contributor("alice@example.com", 2, 10, 2, 1);
        c_b.file_extensions = BTreeMap::from([("rs".to_string(), 3), ("toml".to_string(), 1)]);

        let repo_b = make_repo(
            "repo-b",
            vec![c_b],
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

    fn no_settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn build_contribution_rows_merges_by_email_and_sorts_by_commits() {
        let repos = make_report_with_shared_contributor();

        let rows = build_contribution_rows(&repos, &no_settings());

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].identifier, "alice@example.com");
        assert_eq!(rows[0].commits, 5);
        assert_eq!(rows[0].lines_modified, 47); // 40 added + 7 deleted
        assert_eq!(rows[0].files_touched, 4);
    }

    #[test]
    fn build_user_activity_rows_dedupes_dates_across_repositories() {
        // Alice commits on 2025-01-13 in both repos — should count as 1 active date
        // Total: 3 distinct dates, 5 commits → avg = 5/3
        let repos = make_report_with_shared_contributor();

        let rows = build_user_activity_rows(&repos, &no_settings());

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].identifier, "alice@example.com");
        // 3 distinct dates, 5 commits
        let expected_avg = 5.0 / 3.0;
        assert!((rows[0].average_commits_per_day - expected_avg).abs() < 0.01);
    }

    #[test]
    fn build_user_activity_rows_sorts_by_total_commits_descending() {
        let mut weekday_a = [0u32; 7];
        weekday_a[0] = 10;
        let mut hour_a = [0u32; 24];
        hour_a[10] = 10;

        let mut weekday_b = [0u32; 7];
        weekday_b[0] = 5;
        let mut hour_b = [0u32; 24];
        hour_b[10] = 5;

        let repos = vec![make_repo(
            "repo-a",
            vec![
                make_contributor("big@example.com", 10, 10, 0, 1),
                make_contributor("focused@example.com", 5, 5, 0, 1),
            ],
            vec![
                make_activity(
                    "big@example.com",
                    weekday_a,
                    hour_a,
                    &[
                        "2025-01-13",
                        "2025-01-14",
                        "2025-01-15",
                        "2025-01-16",
                        "2025-01-17",
                    ],
                ),
                make_activity("focused@example.com", weekday_b, hour_b, &["2025-01-13"]),
            ],
            15,
        )];

        let rows = build_user_activity_rows(&repos, &no_settings());

        assert_eq!(rows[0].identifier, "big@example.com");
        assert_eq!(rows[1].identifier, "focused@example.com");
    }

    #[test]
    fn build_heatmap_data_places_commits_in_grid() {
        // 2025-01-13 is Monday (weekday 0)
        // 2025-01-15 is Wednesday (weekday 2)
        // Reference date: 2025-01-19 (Sunday)
        let mut weekday = [0u32; 7];
        weekday[0] = 2;
        weekday[2] = 1;
        let hour = [0u32; 24];

        let repos = vec![make_repo(
            "repo",
            vec![make_contributor("alice@example.com", 3, 10, 0, 1)],
            vec![make_activity(
                "alice@example.com",
                weekday,
                hour,
                &["2025-01-13", "2025-01-15"],
            )],
            3,
        )];

        let data = build_heatmap_data(&repos, None, "2025-01-19", &no_settings());

        assert!(data.max_count >= 1);
        assert!(data.week_count > 0);
        // The grid should contain our commits somewhere in the last week
        let last_week = data.week_count - 1;
        // Monday (0) of that last week = 2025-01-13
        assert_eq!(data.grid[0][last_week], 1); // Monday
        assert_eq!(data.grid[2][last_week], 1); // Wednesday
    }

    #[test]
    fn build_heatmap_data_filters_by_email() {
        let mut weekday = [0u32; 7];
        weekday[0] = 1;
        let hour = [0u32; 24];

        let repos = vec![make_repo(
            "repo",
            vec![
                make_contributor("alice@example.com", 1, 10, 0, 1),
                make_contributor("bob@example.com", 1, 5, 0, 1),
            ],
            vec![
                make_activity("alice@example.com", weekday, hour, &["2025-01-13"]),
                make_activity("bob@example.com", weekday, hour, &["2025-01-14"]),
            ],
            2,
        )];

        let data = build_heatmap_data(
            &repos,
            Some("alice@example.com"),
            "2025-01-19",
            &no_settings(),
        );
        // Only alice's commits should be counted
        assert_eq!(data.max_count, 1);
    }

    #[test]
    fn build_heatmap_data_empty() {
        let repos: Vec<RepositoryAnalysis> = vec![];
        let data = build_heatmap_data(&repos, None, "2025-01-19", &no_settings());
        assert_eq!(data.max_count, 0);
        assert!(data.week_count > 0);
    }

    #[test]
    fn build_heatmap_data_has_month_labels() {
        let repos: Vec<RepositoryAnalysis> = vec![];
        let data = build_heatmap_data(&repos, None, "2025-06-15", &no_settings());
        // Should have multiple month labels across 52 weeks
        assert!(data.month_labels.len() >= 12);
    }

    #[test]
    fn build_heatmap_data_mid_week_end_date() {
        // 2025-01-15 is Wednesday (weekday 2)
        // The grid should stop at Wednesday — Thu/Fri/Sat/Sun of last week should be 0
        let mut weekday = [0u32; 7];
        weekday[2] = 1;
        let hour = [0u32; 24];

        let repos = vec![make_repo(
            "repo",
            vec![make_contributor("alice@example.com", 1, 5, 0, 1)],
            vec![make_activity(
                "alice@example.com",
                weekday,
                hour,
                &["2025-01-15"],
            )],
            1,
        )];

        let data = build_heatmap_data(&repos, None, "2025-01-15", &no_settings());
        let last_week = data.week_count - 1;
        // Wednesday commit present
        assert_eq!(data.grid[2][last_week], 1);
        // Thursday through Sunday of last week should be 0 (beyond end_date)
        for day in 3..7 {
            assert_eq!(data.grid[day][last_week], 0);
        }
    }

    #[test]
    fn build_repo_comparison_computes_commits_per_day() {
        // repo-a: 3 commits on 2 active dates → 1.5 cpd
        // repo-b: 2 commits on 2 active dates → 1.0 cpd
        let repos = make_report_with_shared_contributor();
        let rows = build_repo_comparison(&repos);

        assert_eq!(rows.len(), 2);
        let a = rows.iter().find(|r| r.name == "repo-a").unwrap();
        let b = rows.iter().find(|r| r.name == "repo-b").unwrap();
        assert_eq!(a.total_commits, 3);
        assert_eq!(a.active_days, 2);
        assert!((a.commits_per_day - 1.5).abs() < 0.01);
        assert_eq!(b.total_commits, 2);
        assert_eq!(b.active_days, 2);
        assert!((b.commits_per_day - 1.0).abs() < 0.01);
    }

    #[test]
    fn least_active_index_returns_min_nonzero() {
        let arr = [0, 3, 1, 5, 0, 0, 2];
        assert_eq!(least_active_index(&arr), Some(2));
    }

    #[test]
    fn least_active_index_all_zeros() {
        let arr = [0u32; 7];
        assert_eq!(least_active_index(&arr), None);
    }

    #[test]
    fn get_contributor_emails_deduplicates() {
        let repos = make_report_with_shared_contributor();
        let emails = get_contributor_emails(&repos, &no_settings());
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].0, "alice@example.com");
    }

    #[test]
    fn build_user_effort_data_basic() {
        let mut weekday = [0u32; 7];
        weekday[0] = 3; // Monday
        weekday[4] = 1; // Friday
        let mut hour = [0u32; 24];
        hour[10] = 4;

        let mut ext_map = BTreeMap::new();
        ext_map.insert("rs".to_string(), 5u64);
        ext_map.insert("md".to_string(), 2);

        let mut c = make_contributor("alice@example.com", 4, 40, 10, 7);
        c.file_extensions = ext_map;

        let repos = vec![make_repo(
            "repo",
            vec![c],
            vec![make_activity(
                "alice@example.com",
                weekday,
                hour,
                &["2025-01-13", "2025-01-17"],
            )],
            4,
        )];

        let effort = build_user_effort_data(&repos, "alice@example.com", &no_settings()).unwrap();
        assert_eq!(effort.identifier, "alice@example.com");
        assert_eq!(effort.most_active_weekday, "Monday");
        assert_eq!(effort.least_active_weekday, "Friday");
        assert!((effort.average_commits_per_day - 2.0).abs() < 0.01); // 4 commits / 2 days
        assert_eq!(effort.top_extensions.len(), 2);
        assert_eq!(effort.top_extensions[0].0, "rs");
    }

    #[test]
    fn build_user_effort_data_cross_repo_merge() {
        let repos = make_report_with_shared_contributor();
        let effort = build_user_effort_data(&repos, "alice@example.com", &no_settings()).unwrap();
        assert_eq!(effort.identifier, "alice@example.com");
        // 5 commits / 3 distinct active dates
        assert!((effort.average_commits_per_day - 5.0 / 3.0).abs() < 0.01);
        // Extensions merged: rs = 2+3 = 5, md = 1, toml = 1
        assert_eq!(effort.top_extensions.len(), 3);
        assert_eq!(effort.top_extensions[0], ("rs".to_string(), 5));
    }

    #[test]
    fn build_user_effort_data_unknown_email() {
        let repos = make_report_with_shared_contributor();
        assert!(build_user_effort_data(&repos, "nobody@example.com", &no_settings()).is_none());
    }
}
