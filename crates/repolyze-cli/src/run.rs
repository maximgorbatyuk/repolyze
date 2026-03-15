use std::path::PathBuf;

use repolyze_core::analytics::{build_user_activity_rows, build_users_contribution_rows};
use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::service::analyze_targets_with_store;
use repolyze_git::backend::GitCliBackend;
use repolyze_metrics::FilesystemMetricsBackend;
use repolyze_report::json::render_json;
use repolyze_report::markdown::render_markdown;
use repolyze_report::table::{render_user_activity_table, render_users_contribution_table};

use crate::args::{AnalyzeView, OutputFormat};

/// Run analysis on one or more repositories and return formatted output.
pub fn run_analyze(
    repos: &[PathBuf],
    view: &AnalyzeView,
    format: &OutputFormat,
) -> anyhow::Result<String> {
    // Validate view/format combination before expensive work
    validate_view_format(view, format)?;

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

    match (view, format) {
        (AnalyzeView::All, OutputFormat::Json) => render_json(&report),
        (AnalyzeView::All, OutputFormat::Md) => Ok(render_markdown(&report)),
        (AnalyzeView::UsersContribution, OutputFormat::Table) => {
            let rows = build_users_contribution_rows(&report.repositories);
            Ok(render_users_contribution_table(&rows))
        }
        (AnalyzeView::Activity, OutputFormat::Table) => {
            let rows = build_user_activity_rows(&report.repositories);
            Ok(render_user_activity_table(&rows))
        }
        _ => unreachable!("validate_view_format should have caught this"),
    }
}

fn validate_view_format(view: &AnalyzeView, format: &OutputFormat) -> anyhow::Result<()> {
    match (view, format) {
        (AnalyzeView::All, OutputFormat::Json | OutputFormat::Md) => Ok(()),
        (AnalyzeView::UsersContribution | AnalyzeView::Activity, OutputFormat::Table) => Ok(()),
        (AnalyzeView::All, OutputFormat::Table) => Err(anyhow::anyhow!(
            "'all' view does not support table format; use json or md"
        )),
        (
            AnalyzeView::UsersContribution | AnalyzeView::Activity,
            OutputFormat::Json | OutputFormat::Md,
        ) => Err(anyhow::anyhow!(
            "analytics views only support table format; use --format table"
        )),
    }
}

fn open_store() -> anyhow::Result<repolyze_store::sqlite::SqliteStore> {
    let db_path = repolyze_store::path::resolve_database_path()?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = repolyze_store::sqlite::SqliteStore::open(&db_path)
        .map_err(|e| anyhow::anyhow!("failed to open database: {e}"))?;
    Ok(store)
}
