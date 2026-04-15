use std::collections::HashMap;

use crate::analytics::{build_overall_productivity_trend, build_overall_trends};
use crate::date_util;
use crate::model::{ComparisonReport, ComparisonSummary, PartialFailure, RepositoryAnalysis};
use crate::settings::Settings;

/// Build a comparison report from multiple repository analyses.
pub fn build_comparison_report(
    results: Vec<RepositoryAnalysis>,
    failures: Vec<PartialFailure>,
    settings: &Settings,
) -> ComparisonReport {
    let summary = build_summary(&results, settings);
    let today = date_util::today_ymd();
    let trends = build_overall_trends(&results, &today);
    let productivity_trend = build_overall_productivity_trend(&results, &today);

    ComparisonReport {
        repositories: results,
        summary,
        failures,
        trends,
        productivity_trend,
    }
}

fn build_summary(results: &[RepositoryAnalysis], settings: &Settings) -> ComparisonSummary {
    let mut contributor_keys: HashMap<String, bool> = HashMap::new();
    let mut total_commits: u64 = 0;
    let mut total_lines_changed: u64 = 0;
    let mut total_files: u64 = 0;

    for analysis in results {
        total_commits += analysis.contributions.total_commits;
        total_files += analysis.size.files;

        for contributor in &analysis.contributions.contributors {
            let key = settings.canonical_key(&contributor.email);
            contributor_keys.entry(key).or_insert(true);
            total_lines_changed += contributor.lines_added + contributor.lines_deleted;
        }
    }

    ComparisonSummary {
        total_contributors: contributor_keys.len() as u64,
        total_commits,
        total_lines_changed,
        total_files,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::model::{
        ActivitySummary, ContributionSummary, ContributorStats, RepositoryTarget, SizeMetrics,
    };

    fn no_settings() -> Settings {
        Settings::default()
    }

    fn make_contributor(name: &str, email: &str, commits: u64, net_lines: i64) -> ContributorStats {
        ContributorStats {
            name: name.to_string(),
            email: email.to_string(),
            commits,
            lines_added: net_lines.unsigned_abs(),
            lines_deleted: 0,
            net_lines,
            files_touched: commits,
            file_extensions: BTreeMap::new(),
            active_days: 1,
            first_commit: "2025-01-01".to_string(),
            last_commit: "2025-01-15".to_string(),
        }
    }

    fn make_analysis(
        name: &str,
        contributors: Vec<ContributorStats>,
        files: u64,
    ) -> RepositoryAnalysis {
        let total_commits: u64 = contributors.iter().map(|c| c.commits).sum();
        RepositoryAnalysis {
            repository: RepositoryTarget::Local {
                root: format!("/tmp/{name}").into(),
            },
            contributions: ContributionSummary {
                contributors,
                activity_by_contributor: vec![],
                total_commits,
            },
            activity: ActivitySummary::default(),
            size: SizeMetrics {
                files,
                directories: 1,
                total_bytes: files * 100,
                total_lines: files * 10,
                non_empty_lines: files * 8,
                blank_lines: files * 2,
                by_extension: BTreeMap::new(),
                largest_files: Vec::new(),
                average_file_size: 100.0,
            },
        }
    }

    #[test]
    fn aggregate_sums_file_counts() {
        let repo_a = make_analysis("repo-a", vec![], 10);
        let repo_b = make_analysis("repo-b", vec![], 20);

        let report = build_comparison_report(vec![repo_a, repo_b], vec![], &no_settings());
        assert_eq!(report.summary.total_files, 30);
    }

    #[test]
    fn aggregate_merges_contributors_by_email() {
        let alice_a = make_contributor("Alice", "alice@example.com", 5, 100);
        let bob = make_contributor("Bob", "bob@example.com", 3, 50);
        let alice_b = make_contributor("Alice", "alice@example.com", 2, 30);

        let repo_a = make_analysis("repo-a", vec![alice_a, bob], 10);
        let repo_b = make_analysis("repo-b", vec![alice_b], 5);

        let report = build_comparison_report(vec![repo_a, repo_b], vec![], &no_settings());

        // Alice appears in both repos but should count as 1 unique contributor
        assert_eq!(report.summary.total_contributors, 2);
        assert_eq!(report.summary.total_commits, 10);
        assert_eq!(report.summary.total_lines_changed, 180);
    }

    #[test]
    fn aggregate_total_lines_changed_counts_additions_and_deletions() {
        let repo = RepositoryAnalysis {
            repository: RepositoryTarget::Local {
                root: "/tmp/repo-a".into(),
            },
            contributions: ContributionSummary {
                contributors: vec![ContributorStats {
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                    commits: 2,
                    lines_added: 10,
                    lines_deleted: 4,
                    net_lines: 6,
                    files_touched: 1,
                    file_extensions: BTreeMap::new(),
                    active_days: 1,
                    first_commit: "2025-01-01".to_string(),
                    last_commit: "2025-01-02".to_string(),
                }],
                activity_by_contributor: vec![],
                total_commits: 2,
            },
            activity: ActivitySummary::default(),
            size: SizeMetrics {
                files: 1,
                directories: 1,
                total_bytes: 10,
                total_lines: 1,
                non_empty_lines: 1,
                blank_lines: 0,
                by_extension: BTreeMap::new(),
                largest_files: Vec::new(),
                average_file_size: 10.0,
            },
        };

        let report = build_comparison_report(vec![repo], vec![], &no_settings());
        assert_eq!(report.summary.total_lines_changed, 14);
    }

    #[test]
    fn aggregate_preserves_per_repo_ordering() {
        let repo_a = make_analysis("repo-a", vec![], 10);
        let repo_b = make_analysis("repo-b", vec![], 20);

        let report = build_comparison_report(vec![repo_a, repo_b], vec![], &no_settings());

        assert_eq!(report.repositories.len(), 2);
        assert_eq!(
            report.repositories[0].repository.display_path(),
            "/tmp/repo-a"
        );
        assert_eq!(
            report.repositories[1].repository.display_path(),
            "/tmp/repo-b"
        );
    }

    #[test]
    fn aggregate_includes_failures() {
        let repo_a = make_analysis("repo-a", vec![], 10);
        let failure = PartialFailure {
            identifier: "/tmp/bad-repo".to_string(),
            reason: "not a git repository".to_string(),
        };

        let report = build_comparison_report(vec![repo_a], vec![failure], &no_settings());
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].reason, "not a git repository");
    }
}
