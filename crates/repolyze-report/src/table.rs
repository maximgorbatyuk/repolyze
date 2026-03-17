use std::time::Duration;

use repolyze_core::analytics::RepoComparisonRow;
use repolyze_core::model::{RepositoryAnalysis, UserActivityRow, UsersContributionRow};

pub const USERS_CONTRIBUTION_TITLE: &str = "Users contribution";
pub const USERS_CONTRIBUTION_DESC: &str =
    "Per-contributor commit counts, lines modified, and files touched.";

pub const ACTIVITY_TITLE: &str = "Most active days and hours";
pub const ACTIVITY_DESC: &str =
    "Average commit frequency by weekday and hour for each contributor.";

pub const HEATMAP_TITLE: &str = "Activity heatmap";
pub const HEATMAP_DESC: &str = "Daily commit activity over the past year, grouped by week.";

pub const COMPARE_REPOS_TITLE: &str = "Compare repositories";
pub const COMPARE_REPOS_DESC: &str =
    "Side-by-side comparison of repository activity and commit frequency.";

/// Build a summary header showing period, repo count, and elapsed time.
pub fn render_analysis_header(repos: &[RepositoryAnalysis], elapsed: Duration) -> String {
    let repo_count = repos.len();

    // Derive period from earliest first_commit and latest last_commit across all contributors
    let mut earliest: Option<&str> = None;
    let mut latest: Option<&str> = None;
    for repo in repos {
        for c in &repo.contributions.contributors {
            if !c.first_commit.is_empty() {
                earliest = Some(match earliest {
                    Some(e) if e <= c.first_commit.as_str() => e,
                    _ => &c.first_commit,
                });
            }
            if !c.last_commit.is_empty() {
                latest = Some(match latest {
                    Some(l) if l >= c.last_commit.as_str() => l,
                    _ => &c.last_commit,
                });
            }
        }
    }

    let period_start = earliest
        .map(format_period_datetime)
        .unwrap_or_else(|| "?".to_string());
    let period_end = latest
        .map(format_period_datetime)
        .unwrap_or_else(|| "?".to_string());
    let elapsed_str = format_duration(elapsed);

    let mut out = String::new();
    out.push_str(&format!("Period:    {period_start} .. {period_end}\n"));
    out.push_str(&format!(
        "Projects:  {repo_count} repositor{}\n",
        if repo_count == 1 { "y" } else { "ies" }
    ));
    out.push_str(&format!("Elapsed:   {elapsed_str}\n"));
    out.push('\n');
    out
}

fn format_period_datetime(ts: &str) -> String {
    let Some((date, time_with_offset)) = ts.split_once('T') else {
        return ts.to_string();
    };

    let time_without_zone = time_with_offset
        .split(['+', '-', 'Z'])
        .next()
        .unwrap_or(time_with_offset);
    let time = time_without_zone
        .split('.')
        .next()
        .unwrap_or(time_without_zone);

    format!("{date} {time}")
}

fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let millis = d.subsec_millis();
    if total_secs >= 60 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{mins}m {secs}.{millis:03}s")
    } else if total_secs > 0 {
        format!("{total_secs}.{millis:03}s")
    } else {
        format!("0.{millis:03}s")
    }
}

pub fn render_users_contribution_table(rows: &[UsersContributionRow]) -> String {
    if rows.is_empty() {
        return "No contributor data available.".to_string();
    }

    let mut out = format!("{USERS_CONTRIBUTION_DESC}\n\n");

    let headers = &[
        "Email",
        "Commits",
        "Lines Modified",
        "Lines per commit",
        "Files Touched",
    ];
    let right_align = &[false, true, true, true, true];

    let data: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.email.clone(),
                r.commits.to_string(),
                r.lines_modified.to_string(),
                format!("{:.2}", r.lines_per_commit),
                r.files_touched.to_string(),
            ]
        })
        .collect();

    let total_commits: u64 = rows.iter().map(|r| r.commits).sum();
    let total_lines: u64 = rows.iter().map(|r| r.lines_modified).sum();
    let total_files: u64 = rows.iter().map(|r| r.files_touched).sum();
    let total_lpc = if total_commits > 0 {
        total_lines as f64 / total_commits as f64
    } else {
        0.0
    };
    let totals = vec![
        "Total".to_string(),
        total_commits.to_string(),
        total_lines.to_string(),
        format!("{:.2}", total_lpc),
        total_files.to_string(),
    ];

    out.push_str(&render_plain_table(
        headers,
        &data,
        right_align,
        Some(&totals),
    ));
    out
}

pub fn render_user_activity_table(rows: &[UserActivityRow]) -> String {
    if rows.is_empty() {
        return "No activity data available.".to_string();
    }

    let mut legend = format!("{ACTIVITY_DESC}\n\n");
    legend.push_str("Legend:\n");
    legend.push_str("  Day         Most active week day\n");
    legend.push_str("  C/D (best)  Avg commits per active day on the most active weekday\n");
    legend.push_str("  C/D         Avg commits per active day\n");
    legend.push_str("  C/H (best)  Avg commits per active hour-bucket on the most active hour\n");
    legend.push_str("  C/H         Avg commits per active hour-bucket\n");
    legend.push('\n');

    let headers = &["Email", "Day", "C/D (best)", "C/D", "C/H (best)", "C/H"];
    let right_align = &[false, false, true, true, true, true];

    let data: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.email.clone(),
                r.most_active_week_day.clone(),
                format!("{:.2}", r.average_commits_per_day_in_most_active_day),
                format!("{:.2}", r.average_commits_per_day),
                format!("{:.2}", r.average_commits_per_hour_in_most_active_hour),
                format!("{:.2}", r.average_commits_per_hour),
            ]
        })
        .collect();

    format!(
        "{legend}{}",
        render_plain_table(headers, &data, right_align, None)
    )
}

const WEEKDAY_NAMES_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub fn render_repo_comparison_table(rows: &[RepoComparisonRow]) -> String {
    if rows.len() < 2 {
        return String::new();
    }

    let mut out = format!("{COMPARE_REPOS_DESC}\n\n");

    // Section 1: Top 3 most active
    let mut by_cpd: Vec<&RepoComparisonRow> = rows.iter().collect();
    by_cpd.sort_by(|a, b| {
        b.commits_per_day
            .partial_cmp(&a.commits_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    out.push_str("Most active repositories (commits per active day):\n");
    let name_width = rows.iter().map(|r| r.name.len()).max().unwrap_or(10);
    for row in by_cpd.iter().take(3) {
        out.push_str(&format!(
            "  {:<width$}  {:.2}\n",
            row.name,
            row.commits_per_day,
            width = name_width
        ));
    }
    out.push('\n');

    // Section 2: Top 3 least active
    out.push_str("Least active repositories (commits per active day):\n");
    for row in by_cpd.iter().rev().take(3) {
        out.push_str(&format!(
            "  {:<width$}  {:.2}\n",
            row.name,
            row.commits_per_day,
            width = name_width
        ));
    }
    out.push('\n');

    // Section 3: Top 3 per weekday
    out.push_str("Most active repositories by weekday:\n");
    for (day, day_name) in WEEKDAY_NAMES_SHORT.iter().enumerate() {
        let mut day_sorted: Vec<&RepoComparisonRow> = rows.iter().collect();
        day_sorted.sort_by(|a, b| {
            b.weekday_commits_per_day[day]
                .partial_cmp(&a.weekday_commits_per_day[day])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top3: Vec<String> = day_sorted
            .iter()
            .take(3)
            .filter(|r| r.weekday_commits_per_day[day] > 0.0)
            .map(|r| format!("{} ({:.2})", r.name, r.weekday_commits_per_day[day]))
            .collect();

        if !top3.is_empty() {
            out.push_str(&format!("  {day_name:<5} {}\n", top3.join("  ")));
        }
    }

    out
}

fn render_plain_table(
    headers: &[&str],
    data: &[Vec<String>],
    right_align: &[bool],
    totals: Option<&[String]>,
) -> String {
    let col_count = headers.len();

    // Compute column widths from headers, data, and optional totals
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in data {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            widths[i] = widths[i].max(cell.len());
        }
    }
    if let Some(t) = totals {
        for (i, cell) in t.iter().enumerate().take(col_count) {
            widths[i] = widths[i].max(cell.len());
        }
    }

    let mut out = String::new();

    // Header row — left-aligned always
    out.push_str(&format_row_plain(
        headers.iter().map(|h| h.to_string()),
        &widths,
        &vec![false; col_count],
    ));

    // Separator
    out.push_str(&separator_line(&widths));

    // Data rows
    for row in data {
        out.push_str(&format_row_plain(row.iter().cloned(), &widths, right_align));
    }

    // Totals
    if let Some(t) = totals {
        out.push_str(&separator_line(&widths));
        out.push_str(&format_row_plain(t.iter().cloned(), &widths, right_align));
    }

    out
}

fn separator_line(widths: &[usize]) -> String {
    let mut out = String::new();
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*w));
    }
    out.push('\n');
    out
}

fn format_row_plain(
    cells: impl Iterator<Item = String>,
    widths: &[usize],
    right_align: &[bool],
) -> String {
    let cells_vec: Vec<String> = cells.collect();
    let mut out = String::new();
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let cell = cells_vec.get(i).map(|s| s.as_str()).unwrap_or("");
        let is_right = right_align.get(i).copied().unwrap_or(false);
        if is_right {
            out.push_str(&format!("{:>width$}", cell, width = width));
        } else {
            out.push_str(&format!("{:<width$}", cell, width = width));
        }
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_users_contribution_table_uses_rf8_headers() {
        let rows = vec![UsersContributionRow {
            email: "alice@example.com".to_string(),
            commits: 5,
            lines_modified: 42,
            lines_per_commit: 8.4,
            files_touched: 4,
        }];

        let table = render_users_contribution_table(&rows);

        assert!(table.contains("Email"));
        assert!(table.contains("Commits"));
        assert!(table.contains("Lines Modified"));
        assert!(table.contains("Lines per commit"));
        assert!(table.contains("Files Touched"));
        assert!(!table.contains("Most active week day"));
        assert!(table.contains("alice@example.com"));
        assert!(table.contains("Total"));
    }

    #[test]
    fn render_users_contribution_table_right_aligns_numbers() {
        let rows = vec![
            UsersContributionRow {
                email: "alice@example.com".to_string(),
                commits: 100,
                lines_modified: 5000,
                lines_per_commit: 50.0,
                files_touched: 20,
            },
            UsersContributionRow {
                email: "bob@example.com".to_string(),
                commits: 5,
                lines_modified: 42,
                lines_per_commit: 8.4,
                files_touched: 4,
            },
        ];

        let table = render_users_contribution_table(&rows);
        // Total row should be present
        assert!(table.contains("Total"));
        assert!(table.contains("105")); // 100 + 5
        assert!(table.contains("5042")); // 5000 + 42
        // Separators use dashes, not pipes
        assert!(!table.contains('|'));
        assert!(table.contains("---"));
    }

    #[test]
    fn render_user_activity_table_uses_rf9_headers() {
        let rows = vec![UserActivityRow {
            email: "alice@example.com".to_string(),
            most_active_week_day: "Monday".to_string(),
            average_commits_per_day_in_most_active_day: 2.0,
            average_commits_per_day: 1.5,
            average_commits_per_hour_in_most_active_hour: 2.0,
            average_commits_per_hour: 1.0,
        }];

        let table = render_user_activity_table(&rows);

        assert!(table.contains("Legend:"));
        assert!(table.contains("C/D (best)"));
        assert!(table.contains("C/H (best)"));
        assert!(table.contains("C/D"));
        assert!(table.contains("C/H"));
        assert!(table.contains("Day"));
        assert!(table.contains("alice@example.com"));
        assert!(!table.contains('|'));
    }

    #[test]
    fn render_empty_table_returns_helpful_message() {
        assert_eq!(
            render_users_contribution_table(&[]),
            "No contributor data available."
        );
        assert_eq!(
            render_user_activity_table(&[]),
            "No activity data available."
        );
    }

    #[test]
    fn render_analysis_header_preserves_full_datetimes_in_period() {
        let repos = vec![RepositoryAnalysis {
            repository: repolyze_core::model::RepositoryTarget {
                root: "/tmp/repo".into(),
            },
            contributions: repolyze_core::model::ContributionSummary {
                contributors: vec![repolyze_core::model::ContributorStats {
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                    commits: 1,
                    lines_added: 1,
                    lines_deleted: 0,
                    net_lines: 1,
                    files_touched: 1,
                    active_days: 1,
                    first_commit: "2025-01-01T09:10:11+00:00".to_string(),
                    last_commit: "2025-01-15T10:20:30+00:00".to_string(),
                }],
                activity_by_contributor: vec![],
                total_commits: 1,
            },
            activity: repolyze_core::model::ActivitySummary::default(),
            size: repolyze_core::model::SizeMetrics::default(),
        }];

        let header = render_analysis_header(&repos, Duration::from_millis(250));

        assert!(header.contains("2025-01-01 09:10:11"));
        assert!(header.contains("2025-01-15 10:20:30"));
    }
}
