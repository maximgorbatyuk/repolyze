use std::path::PathBuf;

use repolyze_core::aggregate::build_comparison_report;
use repolyze_core::input::resolve_inputs;
use repolyze_core::model::{PartialFailure, RepositoryAnalysis};
use repolyze_git::activity::build_activity_summary;
use repolyze_git::contributions::analyze_contributions;
use repolyze_metrics::count::analyze_size;
use repolyze_report::json::render_json;
use repolyze_report::markdown::render_markdown;

use crate::args::OutputFormat;

/// Run analysis on one or more repositories and return formatted output.
pub fn run_analyze(repos: &[PathBuf], format: &OutputFormat) -> anyhow::Result<String> {
    let targets = resolve_inputs(repos)?;

    let mut results = Vec::new();
    let mut failures = Vec::new();

    for target in &targets {
        match analyze_single(target) {
            Ok(analysis) => results.push(analysis),
            Err(e) => {
                failures.push(PartialFailure {
                    path: target.root.clone(),
                    reason: e.to_string(),
                });
            }
        }
    }

    let report = build_comparison_report(results, failures);

    match format {
        OutputFormat::Json => render_json(&report),
        OutputFormat::Md => Ok(render_markdown(&report)),
    }
}

fn analyze_single(
    target: &repolyze_core::model::RepositoryTarget,
) -> anyhow::Result<RepositoryAnalysis> {
    let (contributions, commits) = analyze_contributions(target)?;
    let activity = build_activity_summary(&commits);
    let size = analyze_size(target)?;

    Ok(RepositoryAnalysis {
        repository: target.clone(),
        contributions,
        activity,
        size,
    })
}
