use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::date_util;
use crate::model::{
    BarChartData, ContributionRow, DAYS_IN_WEEK, HEATMAP_MAX_WEEKS, HeatmapData,
    RepositoryAnalysis, TimelineData, TrendsData, UserActivityRow, UserEffortData,
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

/// Sum commits in `commits_by_date` whose date falls in the inclusive window `[start, end]`.
fn sum_commits_in_window(commits_by_date: &BTreeMap<String, u32>, start: &str, end: &str) -> u64 {
    commits_by_date
        .range(start.to_string()..=end.to_string())
        .map(|(_, c)| *c as u64)
        .sum()
}

/// Compute trends (30-day and 90-day, current vs previous window) from a per-date commit map.
///
/// Windows are inclusive:
/// - last_30d:  `[today-29 .. today]`
/// - prev_30d:  `[today-59 .. today-30]`
/// - last_90d:  `[today-89 .. today]`
/// - prev_90d:  `[today-179 .. today-90]`
///
/// Averages are commits per calendar day in the window (sum / 30 or sum / 90).
/// Percent change is `(current - prev) / prev * 100`, or `None` when the prior window had zero
/// commits. Note this conflates "no prior activity" with "no current activity either" — callers
/// that need to distinguish should inspect `prev_*_avg` and `last_*_avg` directly.
///
/// If `today` is not a valid "YYYY-MM-DD" date, returns `TrendsData::default()` with the given
/// string echoed back as `reference_date` (all averages zero, all percent changes `None`).
pub fn build_trends_data(commits_by_date: &BTreeMap<String, u32>, today: &str) -> TrendsData {
    if date_util::parse_ymd(today).is_none() {
        return TrendsData {
            reference_date: today.to_string(),
            ..TrendsData::default()
        };
    }

    let last_30d_start = date_util::add_days(today, -29);
    let prev_30d_end = date_util::add_days(today, -30);
    let prev_30d_start = date_util::add_days(today, -59);
    let last_90d_start = date_util::add_days(today, -89);
    let prev_90d_end = date_util::add_days(today, -90);
    let prev_90d_start = date_util::add_days(today, -179);

    let last_30d_sum = sum_commits_in_window(commits_by_date, &last_30d_start, today);
    let prev_30d_sum = sum_commits_in_window(commits_by_date, &prev_30d_start, &prev_30d_end);
    let last_90d_sum = sum_commits_in_window(commits_by_date, &last_90d_start, today);
    let prev_90d_sum = sum_commits_in_window(commits_by_date, &prev_90d_start, &prev_90d_end);

    let last_30d_avg = last_30d_sum as f64 / 30.0;
    let prev_30d_avg = prev_30d_sum as f64 / 30.0;
    let last_90d_avg = last_90d_sum as f64 / 90.0;
    let prev_90d_avg = prev_90d_sum as f64 / 90.0;

    let change_30d_pct = if prev_30d_sum > 0 {
        Some((last_30d_avg - prev_30d_avg) / prev_30d_avg * 100.0)
    } else {
        None
    };
    let change_90d_pct = if prev_90d_sum > 0 {
        Some((last_90d_avg - prev_90d_avg) / prev_90d_avg * 100.0)
    } else {
        None
    };

    TrendsData {
        reference_date: today.to_string(),
        last_30d_avg,
        prev_30d_avg,
        change_30d_pct,
        last_90d_avg,
        prev_90d_avg,
        change_90d_pct,
    }
}

/// Merge per-date commit counts across every contributor in every repo, then compute overall trends.
pub fn build_overall_trends(repos: &[RepositoryAnalysis], today: &str) -> TrendsData {
    let mut merged: BTreeMap<String, u32> = BTreeMap::new();
    for repo in repos {
        for act in &repo.contributions.activity_by_contributor {
            for (date, count) in &act.commits_by_date {
                *merged.entry(date.clone()).or_insert(0) += count;
            }
        }
    }
    build_trends_data(&merged, today)
}

pub fn build_user_effort_data(
    repos: &[RepositoryAnalysis],
    identifier: &str,
    settings: &Settings,
    today: &str,
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

    let trends = build_trends_data(&m.commits_by_date, today);

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
        trends,
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

pub fn build_weekday_chart_data(repos: &[RepositoryAnalysis]) -> BarChartData {
    let mut totals = [0u64; 7];
    for repo in repos {
        for (d, &count) in repo.activity.by_weekday.iter().enumerate() {
            totals[d] += count as u64;
        }
    }
    BarChartData {
        title: "Commits by Weekday".to_string(),
        bars: WEEKDAY_NAMES
            .iter()
            .zip(totals.iter())
            .map(|(name, &val)| (name.to_string(), val))
            .collect(),
    }
}

pub fn build_hourly_chart_data(repos: &[RepositoryAnalysis]) -> BarChartData {
    let mut totals = [0u64; 24];
    for repo in repos {
        for (h, &count) in repo.activity.by_hour.iter().enumerate() {
            totals[h] += count as u64;
        }
    }
    BarChartData {
        title: "Commits by Hour".to_string(),
        bars: totals
            .iter()
            .enumerate()
            .map(|(h, &val)| (format!("{h:02}:00"), val))
            .collect(),
    }
}

pub fn build_timeline_data(repos: &[RepositoryAnalysis]) -> TimelineData {
    let cutoff = date_util::add_days(&date_util::today_ymd(), -90);
    let mut aggregated: BTreeMap<String, u32> = BTreeMap::new();
    for repo in repos {
        for act in &repo.contributions.activity_by_contributor {
            for (date, count) in &act.commits_by_date {
                if date.as_str() >= cutoff.as_str() {
                    let entry = aggregated.entry(date.clone()).or_insert(0);
                    *entry = entry.saturating_add(*count);
                }
            }
        }
    }
    let start_date = aggregated.keys().next().cloned().unwrap_or_default();
    let end_date = aggregated.keys().next_back().cloned().unwrap_or_default();
    let points: Vec<(String, u32)> = aggregated.into_iter().collect();
    TimelineData {
        title: "Commit Timeline (last 3 months)".to_string(),
        points,
        start_date,
        end_date,
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

        let effort =
            build_user_effort_data(&repos, "alice@example.com", &no_settings(), "2026-04-15")
                .unwrap();
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
        let effort =
            build_user_effort_data(&repos, "alice@example.com", &no_settings(), "2026-04-15")
                .unwrap();
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
        assert!(
            build_user_effort_data(&repos, "nobody@example.com", &no_settings(), "2026-04-15")
                .is_none()
        );
    }

    #[test]
    fn build_weekday_chart_data_aggregates_across_repos() {
        let mut repo_a = make_repo("a", vec![], vec![], 0);
        repo_a.activity.by_weekday[0] = 5; // Monday
        repo_a.activity.by_weekday[4] = 3; // Friday
        let mut repo_b = make_repo("b", vec![], vec![], 0);
        repo_b.activity.by_weekday[0] = 2; // Monday
        repo_b.activity.by_weekday[6] = 1; // Sunday

        let chart = build_weekday_chart_data(&[repo_a, repo_b]);
        assert_eq!(chart.bars.len(), 7);
        assert_eq!(chart.bars[0], ("Monday".to_string(), 7));
        assert_eq!(chart.bars[4], ("Friday".to_string(), 3));
        assert_eq!(chart.bars[6], ("Sunday".to_string(), 1));
        assert_eq!(chart.bars[1].1, 0); // Tuesday
    }

    #[test]
    fn build_hourly_chart_data_aggregates_across_repos() {
        let mut repo_a = make_repo("a", vec![], vec![], 0);
        repo_a.activity.by_hour[10] = 4;
        let mut repo_b = make_repo("b", vec![], vec![], 0);
        repo_b.activity.by_hour[10] = 6;
        repo_b.activity.by_hour[15] = 2;

        let chart = build_hourly_chart_data(&[repo_a, repo_b]);
        assert_eq!(chart.bars.len(), 24);
        assert_eq!(chart.bars[10], ("10:00".to_string(), 10));
        assert_eq!(chart.bars[15], ("15:00".to_string(), 2));
        assert_eq!(chart.bars[0].1, 0);
    }

    #[test]
    fn build_timeline_data_merges_across_repos() {
        // Use recent dates so they fall within the 90-day window
        let today = date_util::today_ymd();
        let d1 = date_util::add_days(&today, -10);
        let d2 = date_util::add_days(&today, -9);
        let d3 = date_util::add_days(&today, -8);

        let mut weekday = [0u32; 7];
        weekday[0] = 2;
        let hour = [0u32; 24];

        let repo_a = make_repo(
            "repo-a",
            vec![make_contributor("alice@example.com", 2, 20, 5, 2)],
            vec![make_activity(
                "alice@example.com",
                weekday,
                hour,
                &[&d1, &d2],
            )],
            2,
        );
        let repo_b = make_repo(
            "repo-b",
            vec![make_contributor("alice@example.com", 1, 10, 2, 1)],
            vec![make_activity(
                "alice@example.com",
                weekday,
                hour,
                &[&d1, &d3],
            )],
            1,
        );

        let timeline = build_timeline_data(&[repo_a, repo_b]);

        // d1 appears in both repos → count 2
        // d2 in repo_a only → count 1
        // d3 in repo_b only → count 1
        assert_eq!(timeline.points.len(), 3);
        assert_eq!(timeline.start_date, d1);
        assert_eq!(timeline.end_date, d3);
        let merged = timeline.points.iter().find(|(d, _)| *d == d1).unwrap();
        assert_eq!(merged.1, 2);
    }

    #[test]
    fn build_timeline_data_empty_repos() {
        let timeline = build_timeline_data(&[]);
        assert!(timeline.points.is_empty());
        assert!(timeline.start_date.is_empty());
        assert!(timeline.end_date.is_empty());
    }

    #[test]
    fn build_trends_data_empty_map() {
        let map: BTreeMap<String, u32> = BTreeMap::new();
        let trends = build_trends_data(&map, "2026-04-15");
        assert_eq!(trends.reference_date, "2026-04-15");
        assert_eq!(trends.last_30d_avg, 0.0);
        assert_eq!(trends.prev_30d_avg, 0.0);
        assert_eq!(trends.last_90d_avg, 0.0);
        assert_eq!(trends.prev_90d_avg, 0.0);
        assert!(trends.change_30d_pct.is_none());
        assert!(trends.change_90d_pct.is_none());
    }

    #[test]
    fn build_trends_data_windows_and_percent_change() {
        // Reference date 2026-04-15.
        // last_30d  = [2026-03-17 .. 2026-04-15] — place 60 commits on 2026-04-10
        // prev_30d  = [2026-02-15 .. 2026-03-16] — place 30 commits on 2026-03-01
        // last_90d  = [2026-01-16 .. 2026-04-15] — includes both above (90 total)
        // prev_90d  = [2025-10-18 .. 2026-01-15] — place 45 commits on 2025-12-01
        let mut map = BTreeMap::new();
        map.insert("2026-04-10".to_string(), 60u32);
        map.insert("2026-03-01".to_string(), 30u32);
        map.insert("2025-12-01".to_string(), 45u32);
        // Outside all windows — should not be counted
        map.insert("2020-01-01".to_string(), 999u32);

        let trends = build_trends_data(&map, "2026-04-15");
        // 30d windows
        assert!((trends.last_30d_avg - 60.0 / 30.0).abs() < 1e-9);
        assert!((trends.prev_30d_avg - 30.0 / 30.0).abs() < 1e-9);
        // (2.0 - 1.0) / 1.0 * 100 = +100.0
        assert!((trends.change_30d_pct.unwrap() - 100.0).abs() < 1e-9);
        // 90d windows
        assert!((trends.last_90d_avg - (60.0 + 30.0) / 90.0).abs() < 1e-9);
        assert!((trends.prev_90d_avg - 45.0 / 90.0).abs() < 1e-9);
        // (1.0 - 0.5) / 0.5 * 100 = +100.0
        assert!((trends.change_90d_pct.unwrap() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn build_trends_data_no_prior_activity_returns_none() {
        // Commits only in last_30d window — prev windows are empty
        let mut map = BTreeMap::new();
        map.insert("2026-04-01".to_string(), 15u32);
        let trends = build_trends_data(&map, "2026-04-15");
        assert!(trends.last_30d_avg > 0.0);
        assert_eq!(trends.prev_30d_avg, 0.0);
        assert!(trends.change_30d_pct.is_none());
        assert!(trends.change_90d_pct.is_none());
    }

    #[test]
    fn build_trends_data_invalid_today_returns_default_with_echo() {
        let mut map = BTreeMap::new();
        map.insert("2026-04-10".to_string(), 100u32);
        let trends = build_trends_data(&map, "not-a-date");
        assert_eq!(trends.reference_date, "not-a-date");
        assert_eq!(trends.last_30d_avg, 0.0);
        assert_eq!(trends.last_90d_avg, 0.0);
        assert!(trends.change_30d_pct.is_none());
        assert!(trends.change_90d_pct.is_none());
    }

    #[test]
    fn build_trends_data_far_edge_boundaries_are_exclusive() {
        // today-60 falls before prev_30d_start (today-59) → must NOT be counted in prev_30d.
        // today-180 falls before prev_90d_start (today-179) → must NOT be counted in prev_90d.
        let mut map = BTreeMap::new();
        map.insert("2026-02-14".to_string(), 500u32); // 2026-04-15 minus 60 days
        map.insert("2025-10-17".to_string(), 900u32); // 2026-04-15 minus 180 days
        let trends = build_trends_data(&map, "2026-04-15");
        assert_eq!(trends.prev_30d_avg, 0.0);
        assert_eq!(trends.prev_90d_avg, 0.0);
        assert!(trends.change_30d_pct.is_none());
        assert!(trends.change_90d_pct.is_none());
    }

    #[test]
    fn build_overall_trends_empty_repos() {
        let trends = build_overall_trends(&[], "2026-04-15");
        assert_eq!(trends.reference_date, "2026-04-15");
        assert_eq!(trends.last_30d_avg, 0.0);
        assert_eq!(trends.last_90d_avg, 0.0);
        assert!(trends.change_30d_pct.is_none());
    }

    #[test]
    fn build_trends_data_boundary_dates_are_inclusive() {
        // Exactly 29 days ago is the earliest date still in last_30d
        let mut map = BTreeMap::new();
        map.insert("2026-03-17".to_string(), 30u32); // today-29
        map.insert("2026-04-15".to_string(), 0u32); // today
        // today-30 is the latest date in prev_30d
        map.insert("2026-03-16".to_string(), 60u32);

        let trends = build_trends_data(&map, "2026-04-15");
        assert!((trends.last_30d_avg - 1.0).abs() < 1e-9); // 30 / 30
        assert!((trends.prev_30d_avg - 2.0).abs() < 1e-9); // 60 / 30
    }

    #[test]
    fn build_overall_trends_merges_across_repos_and_contributors() {
        // Two repos, each with one contributor; same date appears in both
        let mut weekday = [0u32; 7];
        weekday[0] = 1;
        let hour = [0u32; 24];

        let mut act_a = make_activity("alice@example.com", weekday, hour, &["2026-04-10"]);
        act_a.commits_by_date.insert("2026-04-10".to_string(), 4);
        let mut act_b = make_activity("bob@example.com", weekday, hour, &["2026-04-10"]);
        act_b.commits_by_date.insert("2026-04-10".to_string(), 6);

        let repo_a = make_repo(
            "repo-a",
            vec![make_contributor("alice@example.com", 4, 0, 0, 0)],
            vec![act_a],
            4,
        );
        let repo_b = make_repo(
            "repo-b",
            vec![make_contributor("bob@example.com", 6, 0, 0, 0)],
            vec![act_b],
            6,
        );

        let trends = build_overall_trends(&[repo_a, repo_b], "2026-04-15");
        // 4 + 6 = 10 commits in the 30-day window → 10/30
        assert!((trends.last_30d_avg - 10.0 / 30.0).abs() < 1e-9);
    }

    #[test]
    fn build_user_effort_data_includes_trends() {
        let mut weekday = [0u32; 7];
        weekday[0] = 1;
        let hour = [0u32; 24];

        let today = "2026-04-15";
        let recent = date_util::add_days(today, -1);
        let mut act = make_activity("alice@example.com", weekday, hour, &[&recent]);
        act.commits_by_date.insert(recent.clone(), 3);

        let repos = vec![make_repo(
            "repo-a",
            vec![make_contributor("alice@example.com", 3, 0, 0, 0)],
            vec![act],
            3,
        )];

        let effort =
            build_user_effort_data(&repos, "alice@example.com", &no_settings(), today).unwrap();
        assert_eq!(effort.trends.reference_date, today);
        assert!((effort.trends.last_30d_avg - 3.0 / 30.0).abs() < 1e-9);
    }
}
