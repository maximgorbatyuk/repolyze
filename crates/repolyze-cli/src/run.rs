use repolyze_core::analytics::{
    build_contribution_rows, build_user_activity_rows, build_user_effort_data,
};
use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::service::{RemoteAnalyzer, analyze_targets_with_store};
use repolyze_core::settings::Settings;
use repolyze_git::backend::GitCliBackend;
use repolyze_github::GitHubBackend;
use repolyze_metrics::FilesystemMetricsBackend;
use repolyze_report::json::render_json;
use repolyze_report::markdown::render_markdown;
use repolyze_report::table::{
    render_analysis_header, render_contribution_table, render_user_activity_table,
    render_user_effort_table,
};

use crate::args::{AnalyzeView, OutputFormat};

/// Run analysis on one or more repositories and return formatted output.
pub fn run_analyze(
    repos: &[String],
    view: &AnalyzeView,
    format: &OutputFormat,
    email: Option<&str>,
    settings: &Settings,
) -> anyhow::Result<String> {
    validate_view_format(view, format)?;

    let (targets, input_failures) = resolve_inputs_with_failures(repos);
    let git = GitCliBackend;
    let metrics = FilesystemMetricsBackend;
    let store = open_store()?;
    let github = GitHubBackend::new(None);
    let has_github_targets = targets.iter().any(|t| t.is_github());
    let remote: Option<&dyn RemoteAnalyzer> = if has_github_targets {
        Some(&github)
    } else {
        None
    };
    let start = std::time::Instant::now();
    let mut report =
        analyze_targets_with_store(&targets, &git, &metrics, &store, remote, "cli", settings);
    let elapsed = start.elapsed();

    if !input_failures.is_empty() {
        let mut failures = input_failures;
        failures.extend(report.failures);
        report.failures = failures;
    }

    match (view, format) {
        (AnalyzeView::All, OutputFormat::Json) => render_json(&report),
        (AnalyzeView::All, OutputFormat::Md) => Ok(render_markdown(&report, settings)),
        (AnalyzeView::Contribution, OutputFormat::Table) => {
            let folder = folder_display(repos);
            let header = render_analysis_header(&report.repositories, elapsed, &folder);
            let rows = build_contribution_rows(&report.repositories, settings);
            Ok(format!("{header}{}", render_contribution_table(&rows)))
        }
        (AnalyzeView::Activity, OutputFormat::Table) => {
            let folder = folder_display(repos);
            let header = render_analysis_header(&report.repositories, elapsed, &folder);
            let rows = build_user_activity_rows(&report.repositories, settings);
            Ok(format!("{header}{}", render_user_activity_table(&rows)))
        }
        (AnalyzeView::UserEffort, OutputFormat::Table) => {
            let email =
                email.ok_or_else(|| anyhow::anyhow!("--email is required for user-effort view"))?;
            let folder = folder_display(repos);
            let header = render_analysis_header(&report.repositories, elapsed, &folder);
            let effort = build_user_effort_data(&report.repositories, email, settings)
                .ok_or_else(|| anyhow::anyhow!("no data found for email '{email}'"))?;
            Ok(format!("{header}{}", render_user_effort_table(&effort)))
        }
        _ => Err(anyhow::anyhow!("unsupported view/format combination")),
    }
}

fn validate_view_format(view: &AnalyzeView, format: &OutputFormat) -> anyhow::Result<()> {
    match (view, format) {
        (AnalyzeView::All, OutputFormat::Json | OutputFormat::Md) => Ok(()),
        (
            AnalyzeView::Contribution | AnalyzeView::Activity | AnalyzeView::UserEffort,
            OutputFormat::Table,
        ) => Ok(()),
        (AnalyzeView::All, OutputFormat::Table) => Err(anyhow::anyhow!(
            "'all' view does not support table format; use json or md"
        )),
        (
            AnalyzeView::Contribution | AnalyzeView::Activity | AnalyzeView::UserEffort,
            OutputFormat::Json | OutputFormat::Md,
        ) => Err(anyhow::anyhow!(
            "analytics views only support table format; use --format table"
        )),
    }
}

fn folder_display(repos: &[String]) -> String {
    if repos.len() == 1 {
        if repos[0].contains("github.com/") {
            return repos[0].clone();
        }
        let path = std::path::Path::new(&repos[0]);
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string()
    } else {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    }
}

fn open_store() -> anyhow::Result<repolyze_store::sqlite::SqliteStore> {
    repolyze_store::sqlite::SqliteStore::open_default()
        .map_err(|e| anyhow::anyhow!("failed to open database: {e}"))
}
