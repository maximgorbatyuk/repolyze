pub mod app;
pub mod event;
pub mod ui;

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    event::{Event, poll as poll_event, read as read_event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use repolyze_core::analytics::{
    build_heatmap_data, build_user_activity_rows, build_users_contribution_rows,
};
use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::model::{ComparisonReport, HeatmapData};
use repolyze_core::service::analyze_targets_with_store;
use repolyze_git::backend::GitCliBackend;
use repolyze_metrics::FilesystemMetricsBackend;
use repolyze_report::table::{
    render_analysis_header, render_user_activity_table, render_users_contribution_table,
};

use app::{AnalyzeView, AppAction, AppState};

struct AnalysisCompletion {
    report: ComparisonReport,
    table_text: String,
    heatmap_data: Option<HeatmapData>,
    failure_count: usize,
    error_message: Option<String>,
}

pub fn run() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    let mut bg_receiver: Option<mpsc::Receiver<AnalysisCompletion>> = None;

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        // Non-blocking poll: 100ms timeout allows spinner animation at ~10fps
        if poll_event(Duration::from_millis(100))?
            && let Event::Key(key) = read_event()?
        {
            event::handle_key(&mut app, key);
        }

        // Advance spinner
        if app.is_loading {
            app.spinner_frame = app.spinner_frame.wrapping_add(1);
        }

        // Start pending analysis on background thread
        if let Some(action) = app.take_action() {
            match action {
                AppAction::StartAnalyze { paths, view } => {
                    app.is_loading = true;
                    app.spinner_frame = 0;
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let result = compute_analysis(paths, view, open_store);
                        tx.send(result).ok();
                    });
                    bg_receiver = Some(rx);
                }
                AppAction::LoadMetadata => {
                    app.metadata_text = Some(build_metadata_text(&open_store));
                }
            }
        }

        // Check for completed analysis
        if let Some(rx) = &bg_receiver {
            match rx.try_recv() {
                Ok(completion) => {
                    apply_analysis_completion(&mut app, completion);
                    bg_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {} // still running
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Thread panicked or dropped sender
                    app.is_loading = false;
                    app.status_message = "Analysis failed unexpectedly".to_string();
                    bg_receiver = None;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn compute_analysis<F>(
    paths: Vec<PathBuf>,
    view: AnalyzeView,
    open_store_fn: F,
) -> AnalysisCompletion
where
    F: Fn() -> anyhow::Result<repolyze_store::sqlite::SqliteStore>,
{
    let (targets, input_failures) = resolve_inputs_with_failures(&paths);
    let git = GitCliBackend;
    let metrics = FilesystemMetricsBackend;
    let store = match open_store_fn() {
        Ok(store) => store,
        Err(error) => {
            return AnalysisCompletion {
                report: ComparisonReport {
                    repositories: vec![],
                    summary: repolyze_core::model::ComparisonSummary {
                        total_contributors: 0,
                        total_commits: 0,
                        total_lines_changed: 0,
                        total_files: 0,
                    },
                    failures: vec![],
                },
                table_text: String::new(),
                heatmap_data: None,
                failure_count: 0,
                error_message: Some(format!("Analysis failed: failed to open database: {error}")),
            };
        }
    };
    let start = std::time::Instant::now();
    let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store, "tui");
    let elapsed = start.elapsed();
    let failure_count = input_failures.len() + report.failures.len();

    if !input_failures.is_empty() {
        let mut failures = input_failures;
        failures.extend(report.failures);
        report.failures = failures;
    }

    let header = render_analysis_header(&report.repositories, elapsed);
    let today = repolyze_core::date_util::today_ymd();
    let (table_body, heatmap_data) = match view {
        AnalyzeView::UsersContribution => {
            let rows = build_users_contribution_rows(&report.repositories);
            (render_users_contribution_table(&rows), None)
        }
        AnalyzeView::Activity => {
            let rows = build_user_activity_rows(&report.repositories);
            (render_user_activity_table(&rows), None)
        }
        AnalyzeView::ActivityHeatmap => {
            let hm = build_heatmap_data(&report.repositories, None, &today);
            (String::new(), Some(hm))
        }
        AnalyzeView::All => {
            let contrib_rows = build_users_contribution_rows(&report.repositories);
            let activity_rows = build_user_activity_rows(&report.repositories);
            let mut combined = render_users_contribution_table(&contrib_rows);
            combined.push_str("\n\n");
            combined.push_str(&render_user_activity_table(&activity_rows));
            let hm = build_heatmap_data(&report.repositories, None, &today);
            (combined, Some(hm))
        }
    };

    AnalysisCompletion {
        report,
        table_text: format!("{header}{table_body}"),
        heatmap_data,
        failure_count,
        error_message: None,
    }
}

fn apply_analysis_completion(app: &mut AppState, completion: AnalysisCompletion) {
    app.is_loading = false;

    if let Some(msg) = completion.error_message {
        app.status_message = msg;
        app.analysis_result = None;
        app.analysis_table = None;
        return;
    }

    app.analysis_table = Some(completion.table_text);
    app.heatmap_data = completion.heatmap_data;
    app.set_result(completion.report);
    if completion.failure_count > 0 {
        app.status_message = format!(
            "Analysis complete with {} error(s)",
            completion.failure_count
        );
    }
}

/// Synchronous execution for tests — keeps the existing test API working.
pub fn execute_pending_action(app: &mut AppState) -> anyhow::Result<()> {
    execute_pending_action_with_store_opener(app, open_store)
}

fn execute_pending_action_with_store_opener<F>(
    app: &mut AppState,
    open_store_fn: F,
) -> anyhow::Result<()>
where
    F: Fn() -> anyhow::Result<repolyze_store::sqlite::SqliteStore>,
{
    let Some(action) = app.take_action() else {
        return Ok(());
    };

    match action {
        AppAction::StartAnalyze { paths, view } => {
            let completion = compute_analysis(paths, view, &open_store_fn);
            apply_analysis_completion(app, completion);
        }
        AppAction::LoadMetadata => {
            app.metadata_text = Some(build_metadata_text(&open_store_fn));
        }
    }

    Ok(())
}

fn build_metadata_text<F>(open_store_fn: &F) -> String
where
    F: Fn() -> anyhow::Result<repolyze_store::sqlite::SqliteStore>,
{
    let db_path = match repolyze_store::path::resolve_database_path() {
        Ok(p) => p,
        Err(e) => return format!("Failed to resolve database path: {e}"),
    };

    let file_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    let store = match open_store_fn() {
        Ok(s) => s,
        Err(e) => {
            return format!(
                "Database:  {}\n\nFailed to open database: {e}",
                db_path.display()
            );
        }
    };

    let meta = match store.database_metadata() {
        Ok(m) => m,
        Err(e) => {
            return format!(
                "Database:  {}\n\nFailed to query metadata: {e}",
                db_path.display()
            );
        }
    };

    let mut out = String::new();
    out.push_str(&format!("Database:  {}\n", db_path.display()));
    out.push_str(&format!(
        "Size:      {} ({file_size} bytes)\n\n",
        format_file_size(file_size)
    ));

    // Build table
    let headers = ["table", "records", "percentage"];
    let right_align = [false, true, true];

    let data: Vec<[String; 3]> = meta
        .tables
        .iter()
        .map(|t| {
            [
                t.table_name.clone(),
                t.record_count.to_string(),
                format!("{:.1}%", t.percentage),
            ]
        })
        .collect();

    let totals = [
        "Total".to_string(),
        meta.total_rows.to_string(),
        "100.0%".to_string(),
    ];

    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in &data {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }
    for (i, cell) in totals.iter().enumerate() {
        widths[i] = widths[i].max(cell.len());
    }

    // Header
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&format!("{:<w$}", h, w = widths[i]));
    }
    out.push('\n');

    // Separator
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*w));
    }
    out.push('\n');

    // Data
    for row in &data {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            if right_align[i] {
                out.push_str(&format!("{:>w$}", cell, w = widths[i]));
            } else {
                out.push_str(&format!("{:<w$}", cell, w = widths[i]));
            }
        }
        out.push('\n');
    }

    // Totals
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*w));
    }
    out.push('\n');
    for (i, cell) in totals.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        if right_align[i] {
            out.push_str(&format!("{:>w$}", cell, w = widths[i]));
        } else {
            out.push_str(&format!("{:<w$}", cell, w = widths[i]));
        }
    }
    out.push('\n');

    out
}

fn format_file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn open_store() -> anyhow::Result<repolyze_store::sqlite::SqliteStore> {
    repolyze_store::sqlite::SqliteStore::open_default()
        .map_err(|e| anyhow::anyhow!("failed to open database: {e}"))
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use std::path::PathBuf;
    use std::process::Command;

    use super::*;

    fn create_fixture_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(root)
            .output()
            .unwrap();

        std::fs::write(root.join("README.md"), "# Test\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()
            .unwrap();
        let output = Command::new("git")
            .args(["commit", "-m", "initial"])
            .env("GIT_AUTHOR_DATE", "2025-01-15T10:00:00+00:00")
            .env("GIT_COMMITTER_DATE", "2025-01-15T10:00:00+00:00")
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());

        dir
    }

    #[test]
    fn execute_pending_action_updates_app_state() {
        let dir = create_fixture_repo();
        let mut app = AppState::new();
        app.input_paths.push(dir.path().to_path_buf());
        app.dispatch_analyze();

        execute_pending_action(&mut app).unwrap();

        assert!(app.pending_action.is_none());
        assert_eq!(app.errors.len(), 0);
        let report = app.analysis_result.as_ref().unwrap();
        assert_eq!(report.repositories.len(), 1);
        assert_eq!(report.summary.total_commits, 1);
    }

    #[test]
    fn execute_pending_action_handles_store_open_failure() {
        let mut app = AppState::new();
        app.pending_action = Some(AppAction::StartAnalyze {
            paths: vec![PathBuf::from("/tmp/repo")],
            view: AnalyzeView::All,
        });

        let result = execute_pending_action_with_store_opener(&mut app, || Err(anyhow!("boom")));

        assert!(result.is_ok());
        assert!(!app.is_loading);
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn load_metadata_populates_metadata_text() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Seed DB
        let store = repolyze_store::sqlite::SqliteStore::open(&db_path).unwrap();
        store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
        drop(store);

        let db_path_clone = db_path.clone();
        let mut app = AppState::new();
        app.pending_action = Some(AppAction::LoadMetadata);

        execute_pending_action_with_store_opener(&mut app, move || {
            repolyze_store::sqlite::SqliteStore::open(&db_path_clone).map_err(|e| anyhow!("{e}"))
        })
        .unwrap();

        let text = app.metadata_text.as_ref().unwrap();
        assert!(text.contains("table"));
        assert!(text.contains("records"));
        assert!(text.contains("repositories"));
        assert!(text.contains("Total"));
    }

    #[test]
    fn load_metadata_handles_store_failure() {
        let mut app = AppState::new();
        app.pending_action = Some(AppAction::LoadMetadata);

        execute_pending_action_with_store_opener(&mut app, || Err(anyhow!("boom"))).unwrap();

        let text = app.metadata_text.as_ref().unwrap();
        assert!(text.contains("Failed to open database"));
    }
}
