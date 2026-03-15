use std::path::PathBuf;

use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::service::analyze_targets_with_store;
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
    let store = open_store()?;
    let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store);

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

fn open_store() -> anyhow::Result<repolyze_store::sqlite::SqliteStore> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME must be set"))?;
    let db_path = repolyze_store::path::database_path_from_home(&home);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = repolyze_store::sqlite::SqliteStore::open(&db_path)
        .map_err(|e| anyhow::anyhow!("failed to open database: {e}"))?;
    Ok(store)
}
