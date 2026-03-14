use repolyze_core::model::ComparisonReport;

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
pub fn render_markdown(report: &ComparisonReport) -> String {
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
    out.push_str("| Name | Email | Commits | Lines Added | Lines Deleted | Net |\n");
    out.push_str("|---|---|---|---|---|---|\n");

    // Collect all contributors across repos, dedup by email (show highest commit count)
    let mut all_contributors: Vec<_> = report
        .repositories
        .iter()
        .flat_map(|a| a.contributions.contributors.iter())
        .collect();
    all_contributors.sort_by(|a, b| b.commits.cmp(&a.commits));

    // Show top 20
    for contributor in all_contributors.iter().take(20) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            contributor.name,
            contributor.email,
            contributor.commits,
            contributor.lines_added,
            contributor.lines_deleted,
            contributor.net_lines,
        ));
    }
    out.push('\n');

    // Activity by hour
    out.push_str("## Activity by Hour\n\n");
    out.push_str("| Hour | Commits |\n");
    out.push_str("|---|---|\n");
    for analysis in &report.repositories {
        for (hour, &count) in analysis.activity.by_hour.iter().enumerate() {
            if count > 0 {
                out.push_str(&format!("| {:02}:00 | {} |\n", hour, count));
            }
        }
    }
    out.push('\n');

    // Activity by weekday
    out.push_str("## Activity by Weekday\n\n");
    out.push_str("| Day | Commits |\n");
    out.push_str("|---|---|\n");
    for analysis in &report.repositories {
        for (day, &count) in analysis.activity.by_weekday.iter().enumerate() {
            if count > 0 {
                out.push_str(&format!("| {} | {} |\n", WEEKDAY_NAMES[day], count));
            }
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
                    active_days: 3,
                    first_commit: "2025-01-01".to_string(),
                    last_commit: "2025-01-15".to_string(),
                }],
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
        let md = render_markdown(&report);
        assert!(md.contains("# Repolyze Analysis Report"));
    }

    #[test]
    fn markdown_report_contains_repository_summary() {
        let report = make_two_repo_report();
        let md = render_markdown(&report);
        assert!(md.contains("## Repository Summary"));
        assert!(md.contains("repo-a"));
        assert!(md.contains("repo-b"));
    }

    #[test]
    fn markdown_report_contains_contributor_section() {
        let report = make_two_repo_report();
        let md = render_markdown(&report);
        assert!(md.contains("## Top Contributors"));
        assert!(md.contains("Alice"));
    }

    #[test]
    fn markdown_report_contains_activity_sections() {
        let report = make_two_repo_report();
        let md = render_markdown(&report);
        assert!(md.contains("## Activity by Hour"));
        assert!(md.contains("## Activity by Weekday"));
    }

    #[test]
    fn markdown_report_contains_size_section() {
        let report = make_two_repo_report();
        let md = render_markdown(&report);
        assert!(md.contains("## Size Comparison"));
    }

    #[test]
    fn markdown_report_includes_failures_when_present() {
        let mut report = make_two_repo_report();
        report.failures.push(PartialFailure {
            path: "/tmp/bad".into(),
            reason: "not a git repository".to_string(),
        });

        let md = render_markdown(&report);
        assert!(md.contains("## Failures"));
        assert!(md.contains("not a git repository"));
    }

    #[test]
    fn markdown_report_omits_failures_when_empty() {
        let report = make_two_repo_report();
        let md = render_markdown(&report);
        assert!(!md.contains("## Failures"));
    }
}
