use std::collections::BTreeMap;
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
    pub active_days: u64,
    pub first_commit: String,
    pub last_commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorActivityStats {
    pub email: String,
    pub weekday_commits: [u32; 7],
    pub hour_commits: [u32; 24],
    pub active_dates: std::collections::BTreeSet<String>,
    pub active_dates_by_weekday: [std::collections::BTreeSet<String>; 7],
    pub active_hour_buckets: std::collections::BTreeSet<String>,
    pub active_hour_buckets_by_hour: [std::collections::BTreeSet<String>; 24],
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
}
