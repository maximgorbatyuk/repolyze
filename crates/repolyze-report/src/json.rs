use repolyze_core::model::ComparisonReport;

/// Render a comparison report as pretty-printed JSON.
pub fn render_json(report: &ComparisonReport) -> anyhow::Result<String> {
    let json = serde_json::to_string_pretty(report)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use repolyze_core::model::{
        ActivitySummary, ComparisonSummary, ContributionSummary, ContributorStats, PartialFailure,
        RepositoryAnalysis, RepositoryTarget, SizeMetrics, TrendsData,
    };

    use super::*;

    fn make_report() -> ComparisonReport {
        ComparisonReport {
            repositories: vec![RepositoryAnalysis {
                repository: RepositoryTarget::Local {
                    root: "/tmp/test-repo".into(),
                },
                contributions: ContributionSummary {
                    contributors: vec![ContributorStats {
                        name: "Alice".to_string(),
                        email: "alice@example.com".to_string(),
                        commits: 5,
                        lines_added: 100,
                        lines_deleted: 10,
                        net_lines: 90,
                        files_touched: 3,
                        file_extensions: std::collections::BTreeMap::new(),
                        active_days: 2,
                        first_commit: "2025-01-01".to_string(),
                        last_commit: "2025-01-15".to_string(),
                    }],
                    activity_by_contributor: vec![],
                    total_commits: 5,
                },
                activity: ActivitySummary::default(),
                size: SizeMetrics {
                    files: 10,
                    directories: 3,
                    total_bytes: 5000,
                    total_lines: 200,
                    non_empty_lines: 180,
                    blank_lines: 20,
                    by_extension: BTreeMap::from([("rs".to_string(), 5), ("md".to_string(), 2)]),
                    largest_files: Vec::new(),
                    average_file_size: 500.0,
                },
            }],
            summary: ComparisonSummary {
                total_contributors: 1,
                total_commits: 5,
                total_lines_changed: 90,
                total_files: 10,
            },
            failures: vec![],
            trends: TrendsData::default(),
        }
    }

    #[test]
    fn json_export_contains_summary_fields() {
        let report = make_report();
        let json = render_json(&report).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.get("repositories").is_some());
        assert!(parsed.get("summary").is_some());
        assert!(parsed.get("failures").is_some());

        let summary = &parsed["summary"];
        assert_eq!(summary["total_commits"], 5);
        assert_eq!(summary["total_contributors"], 1);
        assert_eq!(summary["total_files"], 10);
    }

    #[test]
    fn json_export_contains_contributor_stats() {
        let report = make_report();
        let json = render_json(&report).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let contributors = &parsed["repositories"][0]["contributions"]["contributors"];

        assert_eq!(contributors[0]["name"], "Alice");
        assert_eq!(contributors[0]["commits"], 5);
        assert_eq!(contributors[0]["lines_added"], 100);
    }

    #[test]
    fn json_export_includes_failures() {
        let mut report = make_report();
        report.failures.push(PartialFailure {
            identifier: "/tmp/bad".to_string(),
            reason: "not a git repository".to_string(),
        });

        let json = render_json(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["failures"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["failures"][0]["reason"], "not a git repository");
    }
}
