use std::collections::HashMap;

use repolyze_core::analytics::{
    RepoComparisonRow, build_heatmap_data, build_hourly_chart_data, build_repo_comparison,
    build_timeline_data, build_weekday_chart_data,
};
use repolyze_core::chart_util::{fit_bar_dimensions, format_value_fit};
use repolyze_core::date_util;
use repolyze_core::model::{
    BarChartData, ComparisonReport, ContributorStats, HeatmapData, TimelineData,
};
use repolyze_core::settings::Settings;

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
        let name = analysis.repository.display_name();
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

    // Activity by hour (bar chart)
    let hourly_chart = build_hourly_chart_data(&report.repositories);
    out.push_str(&render_bar_chart_section(&hourly_chart));

    // Activity by weekday (bar chart)
    let weekday_chart = build_weekday_chart_data(&report.repositories);
    out.push_str(&render_bar_chart_section(&weekday_chart));

    // Commit timeline (weekly bar chart)
    let timeline = build_timeline_data(&report.repositories);
    out.push_str(&render_timeline_section(&timeline));

    // Size comparison
    out.push_str("## Size Comparison\n\n");
    out.push_str("| Repository | Files | Directories | Bytes | Lines | Avg File Size |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for analysis in &report.repositories {
        let name = analysis.repository.display_name();
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
                failure.identifier, failure.reason
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

const SPARKLINE_BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Target width for bar charts in Markdown output — fits a standard 80-col terminal.
const BAR_CHART_MAX_WIDTH: usize = 80;

fn format_bars(bars: &[(String, u64)]) -> String {
    let max_val = bars.iter().map(|(_, v)| *v).max().unwrap_or(0);
    if max_val == 0 {
        let labels: String = bars
            .iter()
            .map(|(l, _)| l.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        return format!("{labels}\n(no data)\n");
    }

    let (col_width, gap, display_labels) = fit_bar_dimensions(bars, BAR_CHART_MAX_WIDTH);

    let chart_height: usize = 8;
    let mut out = String::new();

    for row in (0..chart_height).rev() {
        let threshold = (row as f64 + 0.5) / chart_height as f64;
        for (i, (_, value)) in bars.iter().enumerate() {
            if i > 0 && gap > 0 {
                out.push_str(&" ".repeat(gap));
            }
            let ratio = *value as f64 / max_val as f64;
            if ratio >= threshold {
                let block = if ratio >= threshold + 1.0 / chart_height as f64 {
                    "█"
                } else {
                    let frac = ((ratio - threshold) * chart_height as f64 * 8.0).round() as usize;
                    SPARKLINE_BLOCKS[frac.min(7)]
                };
                out.push_str(&block.repeat(col_width));
            } else {
                out.push_str(&" ".repeat(col_width));
            }
        }
        out.push('\n');
    }

    for (i, (_, value)) in bars.iter().enumerate() {
        if i > 0 && gap > 0 {
            out.push_str(&" ".repeat(gap));
        }
        let text = format_value_fit(*value, col_width);
        out.push_str(&format!("{:^col_width$}", text));
    }
    out.push('\n');

    for (i, label) in display_labels.iter().enumerate() {
        if i > 0 && gap > 0 {
            out.push_str(&" ".repeat(gap));
        }
        out.push_str(&format!("{:^col_width$}", label));
    }
    out.push('\n');

    out
}

fn render_bar_chart_section(data: &BarChartData) -> String {
    let mut out = String::new();
    out.push_str(&format!("## {}\n\n", data.title));
    out.push_str("```\n");
    out.push_str(&format_bars(&data.bars));
    out.push_str("```\n\n");
    out
}

fn render_timeline_section(data: &TimelineData) -> String {
    if data.points.is_empty() {
        return String::new();
    }

    // Aggregate daily points into ISO weeks
    let mut weeks: Vec<(String, u64)> = Vec::new();
    for (date, count) in &data.points {
        let week_label = iso_week_label(date);
        if let Some(last) = weeks.last_mut()
            && last.0 == week_label
        {
            last.1 += *count as u64;
        } else {
            weeks.push((week_label, *count as u64));
        }
    }

    let max_val = weeks.iter().map(|(_, v)| *v).max().unwrap_or(0);

    // Sparkline rows (3 rows tall — each row spans 8 sub-levels, total 24)
    let chart_height: usize = 3;
    let total_levels = chart_height * 8;
    let mut sparkline_rows: Vec<String> = Vec::with_capacity(chart_height);
    for row in (0..chart_height).rev() {
        let mut row_str = String::with_capacity(weeks.len());
        for (_, value) in &weeks {
            let filled = if max_val == 0 {
                0
            } else {
                ((*value as f64 / max_val as f64) * total_levels as f64).round() as usize
            };
            let row_filled = filled.saturating_sub(row * 8).min(8);
            if row_filled == 0 {
                row_str.push(' ');
            } else {
                row_str.push_str(SPARKLINE_BLOCKS[row_filled - 1]);
            }
        }
        sparkline_rows.push(row_str);
    }

    // Date labels — first, middle, last
    let mut labels = String::new();
    let n = weeks.len();
    if n > 0 {
        let positions: Vec<usize> = if n > 2 {
            vec![0, n / 2, n - 1]
        } else if n == 2 {
            vec![0, 1]
        } else {
            vec![0]
        };
        let mut cursor = 0;
        for &pos in &positions {
            if pos >= cursor {
                labels.push_str(&" ".repeat(pos - cursor));
                labels.push_str(&weeks[pos].0);
                cursor = pos + weeks[pos].0.len();
            }
        }
    }

    let mut out = String::new();
    out.push_str(&format!("## {}\n\n", data.title));
    out.push_str("```\n");
    for row in &sparkline_rows {
        out.push_str(row);
        out.push('\n');
    }
    out.push_str(&labels);
    out.push('\n');
    out.push_str(&format!(
        "{} = 0  {} = {} commits/week\n",
        SPARKLINE_BLOCKS[0], SPARKLINE_BLOCKS[7], max_val
    ));
    out.push_str("```\n\n");
    out
}

/// Convert a "YYYY-MM-DD" date to an ISO 8601 week label "YYYY-Www".
fn iso_week_label(date: &str) -> String {
    let Some((y, m, d)) = date_util::parse_ymd(date) else {
        return date.to_string();
    };
    let month_days = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut doy = month_days[m as usize - 1] + d;
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    if is_leap && m > 2 {
        doy += 1;
    }
    // Monday=1 .. Sunday=7
    let dow = date_util::day_of_week(y, m, d) as i32 + 1;
    let week = (doy as i32 - dow + 10) / 7;
    if week < 1 {
        format!("{}-W{:02}", y - 1, iso_weeks_in_year(y - 1))
    } else if week > iso_weeks_in_year(y) as i32 {
        format!("{}-W01", y + 1)
    } else {
        format!("{y}-W{week:02}")
    }
}

/// Number of ISO weeks in a year (52 or 53).
fn iso_weeks_in_year(y: i32) -> u32 {
    let jan1_dow = date_util::day_of_week(y, 1, 1); // 0=Mon
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    // A year has 53 weeks if Jan 1 is Thursday, or Dec 31 is Thursday
    if jan1_dow == 3 || (jan1_dow == 2 && is_leap) {
        53
    } else {
        52
    }
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
            repository: RepositoryTarget::Local {
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
        assert!(md.contains("## Commits by Hour"));
        assert!(md.contains("## Commits by Weekday"));
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
            identifier: "/tmp/bad".to_string(),
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
        report.repositories[1].activity.by_hour[10] = 2;
        report.repositories[1].activity.by_weekday[2] = 2;

        let md = render_markdown(&report, &Settings::default());

        // Scope hour-chart assertions to the "## Commits by Hour" section — other tables
        // (Repository Summary, Top Contributors) also contain "10" and "3" substrings.
        let hour_section = md
            .split("## Commits by Hour")
            .nth(1)
            .and_then(|s| s.split("\n## ").next())
            .expect("hour chart section present");
        // 24 bars don't fit at "HH:00" in 80 cols, so labels collapse to "HH".
        assert!(hour_section.contains(" 10 ") || hour_section.contains(" 10\n"));
        // Aggregated value (1 + 2 = 3) appears in the chart's value row.
        assert!(hour_section.contains('3'));
        assert!(md.contains("Wednesday"));
    }

    #[test]
    fn render_bar_chart_section_formats_bars() {
        let data = BarChartData {
            title: "Test Chart".to_string(),
            bars: vec![
                ("Mon".to_string(), 10),
                ("Tue".to_string(), 5),
                ("Wed".to_string(), 0),
            ],
        };
        let result = render_bar_chart_section(&data);
        assert!(result.contains("## Test Chart"));
        assert!(result.contains("Mon"));
        assert!(result.contains("Tue"));
        assert!(result.contains("10"));
        assert!(result.contains("5"));
        // Zero-value bars still show the label
        assert!(result.contains("Wed"));
    }

    #[test]
    fn render_timeline_section_renders_sparkline() {
        let data = TimelineData {
            title: "Commit Timeline".to_string(),
            points: vec![
                ("2025-01-13".to_string(), 3),
                ("2025-01-14".to_string(), 2),
                ("2025-01-20".to_string(), 5),
            ],
            start_date: "2025-01-13".to_string(),
            end_date: "2025-01-20".to_string(),
        };
        let result = render_timeline_section(&data);
        assert!(result.contains("## Commit Timeline"));
        // Sparkline uses block characters
        assert!(result.contains('▁') || result.contains('█'));
        // Legend shows max commits/week
        assert!(result.contains("commits/week"));
    }

    #[test]
    fn render_timeline_section_empty_data() {
        let data = TimelineData {
            title: "Empty".to_string(),
            points: vec![],
            start_date: String::new(),
            end_date: String::new(),
        };
        assert!(render_timeline_section(&data).is_empty());
    }

    #[test]
    fn iso_week_label_known_dates() {
        assert_eq!(iso_week_label("2025-01-01"), "2025-W01"); // Wednesday
        assert_eq!(iso_week_label("2025-01-06"), "2025-W02"); // Monday
        assert_eq!(iso_week_label("2025-01-13"), "2025-W03"); // Monday
        assert_eq!(iso_week_label("2025-06-15"), "2025-W24"); // Sunday
    }

    #[test]
    fn iso_week_label_cross_year() {
        // 2024-12-30 is Monday — belongs to ISO week 1 of 2025
        assert_eq!(iso_week_label("2024-12-30"), "2025-W01");
        // 2024-12-29 is Sunday — still ISO week 52 of 2024
        assert_eq!(iso_week_label("2024-12-29"), "2024-W52");
    }
}
