pub mod count;
pub mod walk;

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{RepositoryTarget, SizeMetrics};
use repolyze_core::service::MetricsAnalyzer;

pub struct FilesystemMetricsBackend;

impl MetricsAnalyzer for FilesystemMetricsBackend {
    fn analyze_size(&self, target: &RepositoryTarget) -> Result<SizeMetrics, RepolyzeError> {
        count::analyze_size(target)
    }
}
