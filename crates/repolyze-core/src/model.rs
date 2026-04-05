use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AnalysisRequest {
    pub repositories: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryTarget {
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryAnalysis {
    pub repository: RepositoryTarget,
    pub contributions: ContributionSummary,
    pub activity: ActivitySummary,
    pub size: SizeMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionSummary {
    pub contributors: Vec<ContributorStats>,
    #[serde(default)]
    pub activity_by_contributor: Vec<ContributorActivityStats>,
    pub total_commits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorStats {
    pub name: String,
    pub email: String,
    pub commits: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
    pub net_lines: i64,
    pub files_touched: u64,
    #[serde(default)]
    pub file_extensions: BTreeMap<String, u64>,
    pub active_days: u64,
    pub first_commit: String,
    pub last_commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorActivityStats {
    pub email: String,
    pub weekday_commits: [u32; 7],
    pub hour_commits: [u32; 24],
    pub active_dates: BTreeSet<String>,
    pub active_dates_by_weekday: [BTreeSet<String>; 7],
    pub active_hour_buckets: BTreeSet<String>,
    pub active_hour_buckets_by_hour: [BTreeSet<String>; 24],
    #[serde(default)]
    pub commits_by_date: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub by_hour: [u32; 24],
    pub by_weekday: [u32; 7],
    pub heatmap: [[u32; 24]; 7],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeMetrics {
    pub files: u64,
    pub directories: u64,
    pub total_bytes: u64,
    pub total_lines: u64,
    pub non_empty_lines: u64,
    pub blank_lines: u64,
    pub by_extension: BTreeMap<String, u64>,
    pub largest_files: Vec<FileMetric>,
    pub average_file_size: f64,
}

impl Default for SizeMetrics {
    fn default() -> Self {
        Self {
            files: 0,
            directories: 0,
            total_bytes: 0,
            total_lines: 0,
            non_empty_lines: 0,
            blank_lines: 0,
            by_extension: BTreeMap::new(),
            largest_files: Vec::new(),
            average_file_size: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetric {
    pub path: PathBuf,
    pub bytes: u64,
    pub lines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub repositories: Vec<RepositoryAnalysis>,
    pub summary: ComparisonSummary,
    pub failures: Vec<PartialFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub total_contributors: u64,
    pub total_commits: u64,
    pub total_lines_changed: u64,
    pub total_files: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialFailure {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContributionRow {
    pub identifier: String,
    pub commits: u64,
    pub lines_modified: u64,
    pub lines_per_commit: f64,
    pub files_touched: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserActivityRow {
    pub identifier: String,
    pub most_active_week_day: String,
    pub average_commits_per_day_in_most_active_day: f64,
    pub average_commits_per_day: f64,
    pub average_commits_per_hour_in_most_active_hour: f64,
    pub average_commits_per_hour: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserEffortData {
    pub name: String,
    pub identifier: String,
    pub first_commit: String,
    pub last_commit: String,
    pub most_active_weekday: String,
    pub most_active_weekday_commits_per_day: f64,
    pub average_commits_per_day: f64,
    pub least_active_weekday: String,
    pub least_active_weekday_commits_per_day: f64,
    pub avg_files_per_commit: f64,
    pub avg_files_per_day: f64,
    pub avg_lines_per_commit: f64,
    pub avg_lines_per_day: f64,
    pub top_extensions: Vec<(String, u64)>,
}

impl fmt::Display for UserEffortData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.identifier, self.name)
    }
}

pub const HEATMAP_MAX_WEEKS: usize = 53;
pub const DAYS_IN_WEEK: usize = 7;

#[derive(Debug, Clone)]
pub struct HeatmapData {
    pub start_date: String,
    pub end_date: String,
    pub grid: [[u32; HEATMAP_MAX_WEEKS]; DAYS_IN_WEEK],
    pub week_count: usize,
    pub max_count: u32,
    pub month_labels: Vec<(usize, String)>,
}

impl HeatmapData {
    /// Returns 5 legend labels showing the commit-count range for each intensity level.
    /// Order: None, Low, Medium, High, Maximum.
    pub fn legend_labels(&self) -> [String; 5] {
        let max = self.max_count;
        if max == 0 {
            return ["0".into(), "0".into(), "0".into(), "0".into(), "0".into()];
        }

        let t1 = (max as f64 * 0.25).floor() as u32;
        let t2 = (max as f64 * 0.50).floor() as u32;
        let t3 = (max as f64 * 0.75).floor() as u32;

        let low_lo = 1;
        let low_hi = t1.max(1);
        let med_lo = low_hi + 1;
        let med_hi = t2.max(med_lo);
        let high_lo = med_hi + 1;
        let high_hi = t3.max(high_lo);
        let max_lo = high_hi + 1;

        let fmt_range = |lo: u32, hi: u32| -> String {
            if lo == hi {
                format!("{lo}")
            } else {
                format!("{lo}-{hi}")
            }
        };

        [
            "0".into(),
            fmt_range(low_lo, low_hi),
            fmt_range(med_lo, med_hi),
            fmt_range(high_lo, high_hi),
            fmt_range(max_lo, max),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_request_supports_multiple_repositories() {
        let request = AnalysisRequest {
            repositories: vec!["/tmp/a".into(), "/tmp/b".into()],
        };

        assert_eq!(request.repositories.len(), 2);
    }

    #[test]
    fn activity_summary_defaults_to_zeros() {
        let activity = ActivitySummary::default();
        assert_eq!(activity.by_hour.iter().sum::<u32>(), 0);
        assert_eq!(activity.by_weekday.iter().sum::<u32>(), 0);
    }

    #[test]
    fn size_metrics_defaults_to_zeros() {
        let size = SizeMetrics::default();
        assert_eq!(size.files, 0);
        assert_eq!(size.total_bytes, 0);
        assert!(size.by_extension.is_empty());
    }

    fn make_heatmap(max_count: u32) -> HeatmapData {
        HeatmapData {
            start_date: "2024-01-01".into(),
            end_date: "2025-01-01".into(),
            grid: [[0; HEATMAP_MAX_WEEKS]; DAYS_IN_WEEK],
            week_count: 52,
            max_count,
            month_labels: vec![],
        }
    }

    #[test]
    fn legend_labels_zero_max() {
        let labels = make_heatmap(0).legend_labels();
        assert_eq!(labels, ["0", "0", "0", "0", "0"]);
    }

    #[test]
    fn legend_labels_max_one() {
        let labels = make_heatmap(1).legend_labels();
        // All non-zero levels collapse to 1
        assert_eq!(labels[0], "0");
        assert!(labels.iter().all(|l| !l.is_empty()));
    }

    #[test]
    fn legend_labels_max_twelve() {
        let labels = make_heatmap(12).legend_labels();
        // 25% of 12 = 3, 50% = 6, 75% = 9
        assert_eq!(labels[0], "0");
        assert_eq!(labels[1], "1-3");
        assert_eq!(labels[2], "4-6");
        assert_eq!(labels[3], "7-9");
        assert_eq!(labels[4], "10-12");
    }
}
