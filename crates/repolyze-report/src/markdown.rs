use std::collections::HashMap;

use repolyze_core::analytics::{RepoComparisonRow, build_heatmap_data, build_repo_comparison};
use repolyze_core::date_util;
use repolyze_core::model::{ComparisonReport, ContributorStats, HeatmapData};
use repolyze_core::settings::Settings;

const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Render a comparison report as Markdown.
pub fn render_markdown(report: &ComparisonReport, settings: &Settings) -> String {
    let mut out = String::new();

    // Title
    out.push_str("# Repolyze Analysis Report\n\n");

    // Scope
    out.push_str("## Scope\n\n");
    out.push_str(&format!(
        "Analyzed **{}** repositor{}.\n\n",
        report.repositories.len(),
        if report.repositories.len() == 1 {
            "y"
        } else {
            "ies"
        }
    ));

    // Repository summary table
    out.push_str("## Repository Summary\n\n");
    out.push_str("| Repository | Files | Lines | Commits | Contributors |\n");
    out.push_str("|---|---|---|---|---|\n");
    for analysis in &report.repositories {
        let name = analysis
            .repository
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| analysis.repository.root.to_string_lossy().to_string());
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            name,
            analysis.size.files,
            analysis.size.total_lines,
            analysis.contributions.total_commits,
            analysis.contributions.contributors.len(),
        ));
    }
    out.push('\n');

    // Top contributors
    out.push_str("## Top Contributors\n\n");
    out.push_str("| Author | Commits | Lines Added | Lines Deleted | Net |\n");
    out.push_str("|---|---|---|---|---|\n");

    // Aggregate contributors by canonical key across repos.
    // The display name is the configured user name (from settings) or the git author name.
    let mut by_key: HashMap<String, (String, ContributorStats)> = HashMap::new();
    for analysis in &report.repositories {
        for c in &analysis.contributions.contributors {
            let key = settings.canonical_key(&c.email);
            by_key
                .entry(key.clone())
                .and_modify(|(_, acc)| {
                    acc.commits += c.commits;
                    acc.lines_added += c.lines_added;
                    acc.lines_deleted += c.lines_deleted;
                    acc.net_lines += c.net_lines;
                    acc.files_touched += c.files_touched;
                })
                .or_insert_with(|| (key, c.clone()));
        }
    }
    let mut merged: Vec<_> = by_key.into_values().collect();
    merged.sort_by(|a, b| b.1.commits.cmp(&a.1.commits));

    for (display_key, contributor) in merged.iter().take(20) {
        // Use configured name if different from email, otherwise git name
        let author = if display_key != &contributor.email.to_lowercase() {
            display_key.as_str()
        } else {
            &contributor.name
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            author,
            contributor.commits,
            contributor.lines_added,
            contributor.lines_deleted,
            contributor.net_lines,
        ));
    }
    out.push('\n');

    // Activity by hour (aggregated across repos)
    out.push_str("## Activity by Hour\n\n");
    out.push_str("| Hour | Commits |\n");
    out.push_str("|---|---|\n");
    let mut hours = [0u32; 24];
    for analysis in &report.repositories {
        for (h, &count) in analysis.activity.by_hour.iter().enumerate() {
            hours[h] += count;
        }
    }
    for (hour, &count) in hours.iter().enumerate() {
        if count > 0 {
            out.push_str(&format!("| {:02}:00 | {count} |\n", hour));
        }
    }
    out.push('\n');

    // Activity by weekday (aggregated across repos)
    out.push_str("## Activity by Weekday\n\n");
    out.push_str("| Day | Commits |\n");
    out.push_str("|---|---|\n");
    let mut weekdays = [0u32; 7];
    for analysis in &report.repositories {
        for (d, &count) in analysis.activity.by_weekday.iter().enumerate() {
            weekdays[d] += count;
        }
    }
    for (day, &count) in weekdays.iter().enumerate() {
        if count > 0 {
            out.push_str(&format!("| {} | {count} |\n", WEEKDAY_NAMES[day]));
        }
    }
    out.push('\n');

    // Size comparison
    out.push_str("## Size Comparison\n\n");
    out.push_str("| Repository | Files | Directories | Bytes | Lines | Avg File Size |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for analysis in &report.repositories {
        let name = analysis
            .repository
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| analysis.repository.root.to_string_lossy().to_string());
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.0} |\n",
            name,
            analysis.size.files,
            analysis.size.directories,
            analysis.size.total_bytes,
            analysis.size.total_lines,
            analysis.size.average_file_size,
        ));
    }
    out.push('\n');

    // Repository comparison (multi-repo only)
    if report.repositories.len() > 1 {
        let comparison = build_repo_comparison(&report.repositories);
        if comparison.len() >= 2 {
            out.push_str("## Compare Repositories\n\n");
            out.push_str(&render_repo_comparison_markdown(&comparison));
        }
    }

    // Activity heatmap
    let today = date_util::today_ymd();
    let heatmap = build_heatmap_data(&report.repositories, None, &today, settings);
    out.push_str(&render_heatmap_section(&heatmap));

    // Failures
    if !report.failures.is_empty() {
        out.push_str("## Failures\n\n");
        for failure in &report.failures {
            out.push_str(&format!(
                "- **{}**: {}\n",
                failure.path.display(),
                failure.reason
            ));
        }
        out.push('\n');
    }

    out
}

const WEEKDAY_NAMES_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn render_repo_comparison_markdown(rows: &[RepoComparisonRow]) -> String {
    let mut out = String::new();

    let mut by_cpd: Vec<&RepoComparisonRow> = rows.iter().collect();
    by_cpd.sort_by(|a, b| {
        b.commits_per_day
            .partial_cmp(&a.commits_per_day)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Section 1: Top 3 most active
    out.push_str("### Most active repositories (commits per active day)\n\n");
    out.push_str("| Repository | Commits | Active days | C/D |\n");
    out.push_str("|---|---|---|---|\n");
    for r in by_cpd.iter().take(3) {
        out.push_str(&format!(
            "| {} | {} | {} | {:.2} |\n",
            r.name, r.total_commits, r.active_days, r.commits_per_day
        ));
    }
    out.push('\n');

    // Section 2: Top 3 least active
    out.push_str("### Least active repositories (commits per active day)\n\n");
    out.push_str("| Repository | Commits | Active days | C/D |\n");
    out.push_str("|---|---|---|---|\n");
    for r in by_cpd.iter().rev().take(3) {
        out.push_str(&format!(
            "| {} | {} | {} | {:.2} |\n",
            r.name, r.total_commits, r.active_days, r.commits_per_day
        ));
    }
    out.push('\n');

    // Section 3: Top 3 per weekday
    out.push_str("### Most active repositories by weekday (C/D)\n\n");
    out.push_str("| Weekday | #1 | #2 | #3 |\n");
    out.push_str("|---|---|---|---|\n");
    for (day, day_name) in WEEKDAY_NAMES_SHORT.iter().enumerate() {
        let mut day_sorted: Vec<&RepoComparisonRow> = rows.iter().collect();
        day_sorted.sort_by(|a, b| {
            b.weekday_commits_per_day[day]
                .partial_cmp(&a.weekday_commits_per_day[day])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let entries: Vec<String> = day_sorted
            .iter()
            .take(3)
            .filter(|r| r.weekday_commits_per_day[day] > 0.0)
            .map(|r| format!("{} ({:.2})", r.name, r.weekday_commits_per_day[day]))
            .collect();

        if entries.is_empty() {
            continue;
        }

        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            day_name,
            entries.first().cloned().unwrap_or_default(),
            entries.get(1).cloned().unwrap_or_default(),
            entries.get(2).cloned().unwrap_or_default(),
        ));
    }
    out.push('\n');

    out
}

fn heatmap_char(count: u32, max: u32) -> char {
    if count == 0 || max == 0 {
        '\u{b7}' // ·
    } else {
        let ratio = count as f64 / max as f64;
        if ratio <= 0.25 {
            '\u{2591}' // ░
        } else if ratio <= 0.50 {
            '\u{2592}' // ▒
        } else if ratio <= 0.75 {
            '\u{2593}' // ▓
        } else {
            '\u{2588}' // █
        }
    }
}

fn render_heatmap_section(data: &HeatmapData) -> String {
    let mut out = String::new();
    out.push_str("## Activity Heatmap\n\n");
    out.push_str(&format!(
        "Period: {} .. {}\n\n",
        data.start_date, data.end_date
    ));
    out.push_str("```\n");

    // Month labels row
    out.push_str("     ");
    let mut last_col = 0;
    for (col, label) in &data.month_labels {
        let char_pos = col * 2;
        if char_pos > last_col {
            out.push_str(&" ".repeat(char_pos - last_col));
        }
        out.push_str(label);
        last_col = char_pos + label.len();
    }
    out.push('\n');

    // Weekday rows
    let weekday_labels = ["Mon", "   ", "Wed", "   ", "Fri", "   ", "Sun"];
    for (weekday, label) in weekday_labels.iter().enumerate() {
        out.push_str(&format!("{label}  "));
        for week_col in 0..data.week_count {
            let count = data.grid[weekday][week_col];
            out.push(heatmap_char(count, data.max_count));
            out.push(' ');
        }
        out.push('\n');
    }

    // Legend with commit-count ranges
    let labels = data.legend_labels();
    let chars = ['\u{b7}', '\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}'];
    out.push('\n');
    out.push_str("     ");
    for (i, (ch, label)) in chars.iter().zip(labels.iter()).enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push(*ch);
        out.push(' ');
        out.push_str(label);
    }
    out.push('\n');
    out.push_str("```\n\n");

    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use repolyze_core::model::{
        ActivitySummary, ComparisonSummary, ContributionSummary, ContributorStats, PartialFailure,
        RepositoryAnalysis, RepositoryTarget, SizeMetrics,
    };

    use super::*;

    fn make_two_repo_report() -> ComparisonReport {
        let make_analysis = |name: &str, commits: u64, files: u64| RepositoryAnalysis {
            repository: RepositoryTarget {
                root: format!("/tmp/{name}").into(),
            },
            contributions: ContributionSummary {
                contributors: vec![ContributorStats {
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                    commits,
                    lines_added: commits * 20,
                    lines_deleted: commits * 5,
                    net_lines: (commits * 15) as i64,
                    files_touched: files,
                    file_extensions: std::collections::BTreeMap::new(),
                    active_days: 3,
                    first_commit: "2025-01-01".to_string(),
                    last_commit: "2025-01-15".to_string(),
                }],
                activity_by_contributor: vec![],
                total_commits: commits,
            },
            activity: ActivitySummary::default(),
            size: SizeMetrics {
                files,
                directories: 2,
                total_bytes: files * 100,
                total_lines: files * 10,
                non_empty_lines: files * 8,
                blank_lines: files * 2,
                by_extension: BTreeMap::new(),
                largest_files: Vec::new(),
                average_file_size: 100.0,
            },
        };

        ComparisonReport {
            repositories: vec![
                make_analysis("repo-a", 10, 20),
                make_analysis("repo-b", 5, 15),
            ],
            summary: ComparisonSummary {
                total_contributors: 1,
                total_commits: 15,
                total_lines_changed: 225,
                total_files: 35,
            },
            failures: vec![],
        }
    }

    #[test]
    fn markdown_report_contains_title() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("# Repolyze Analysis Report"));
    }

    #[test]
    fn markdown_report_contains_repository_summary() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("## Repository Summary"));
        assert!(md.contains("repo-a"));
        assert!(md.contains("repo-b"));
    }

    #[test]
    fn markdown_report_contains_contributor_section() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("## Top Contributors"));
        assert!(md.contains("Alice"));
    }

    #[test]
    fn markdown_report_contains_activity_sections() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("## Activity by Hour"));
        assert!(md.contains("## Activity by Weekday"));
    }

    #[test]
    fn markdown_report_contains_size_section() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("## Size Comparison"));
    }

    #[test]
    fn markdown_report_includes_failures_when_present() {
        let mut report = make_two_repo_report();
        report.failures.push(PartialFailure {
            path: "/tmp/bad".into(),
            reason: "not a git repository".to_string(),
        });

        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("## Failures"));
        assert!(md.contains("not a git repository"));
    }

    #[test]
    fn markdown_report_omits_failures_when_empty() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(!md.contains("## Failures"));
    }

    #[test]
    fn markdown_report_aggregates_contributors_by_key_across_repos() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());

        // Without settings, the author column shows the git name
        assert!(md.contains("| Alice | 15 | 300 | 75 | 225 |"));
    }

    #[test]
    fn markdown_report_contains_heatmap_section() {
        let report = make_two_repo_report();
        let md = render_markdown(&report, &Settings::default());
        assert!(md.contains("## Activity Heatmap"));
        assert!(md.contains("Mon"));
        assert!(md.contains("Wed"));
        assert!(md.contains("Fri"));
        assert!(md.contains("Sun"));
        // Legend shows commit-count ranges (test data has max_count=0, so all show "0")
        assert!(md.contains("\u{b7} 0"));
    }

    #[test]
    fn markdown_report_aggregates_activity_across_repos() {
        let mut report = make_two_repo_report();
        report.repositories[0].activity.by_hour[10] = 1;
        report.repositories[0].activity.by_weekday[2] = 1;
        report.repositories[0].activity.heatmap[2][10] = 1;
        report.repositories[1].activity.by_hour[10] = 2;
        report.repositories[1].activity.by_weekday[2] = 2;
        report.repositories[1].activity.heatmap[2][10] = 2;

        let md = render_markdown(&report, &Settings::default());

        assert_eq!(md.matches("| 10:00 | 3 |").count(), 1);
        assert_eq!(md.matches("| Wednesday | 3 |").count(), 1);
    }
}
