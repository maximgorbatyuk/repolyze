use std::path::PathBuf;

use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::service::analyze_targets;
use repolyze_git::backend::GitCliBackend;
use repolyze_metrics::FilesystemMetricsBackend;
use repolyze_report::json::render_json;
use repolyze_report::markdown::render_markdown;

use crate::args::OutputFormat;

/// Run analysis on one or more repositories and return formatted output.
pub fn run_analyze(repos: &[PathBuf], format: &OutputFormat) -> anyhow::Result<String> {
    let (targets, input_failures) = resolve_inputs_with_failures(repos);
    let git = GitCliBackend;
    let metrics = FilesystemMetricsBackend;
    let mut report = analyze_targets(&targets, &git, &metrics);

    if !input_failures.is_empty() {
        let mut failures = input_failures;
        failures.extend(report.failures);
        report.failures = failures;
    }

    match format {
        OutputFormat::Json => render_json(&report),
        OutputFormat::Md => Ok(render_markdown(&report)),
    }
}
