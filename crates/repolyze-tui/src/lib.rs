pub mod app;
pub mod event;
pub mod ui;

use std::io;

use crossterm::{
    event::{Event, read as read_event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use repolyze_core::analytics::{build_user_activity_rows, build_users_contribution_rows};
use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::service::analyze_targets_with_store;
use repolyze_git::backend::GitCliBackend;
use repolyze_metrics::FilesystemMetricsBackend;
use repolyze_report::table::{
    render_analysis_header, render_user_activity_table, render_users_contribution_table,
};

use app::{AnalyzeView, AppAction, AppState, Screen};

pub fn run() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if let Event::Key(key) = read_event()? {
            event::handle_key(&mut app, key);
        }

        execute_pending_action(&mut app)?;

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

pub fn execute_pending_action(app: &mut AppState) -> anyhow::Result<()> {
    execute_pending_action_with_store_opener(app, open_store)
}

fn execute_pending_action_with_store_opener<F>(
    app: &mut AppState,
    open_store_fn: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<repolyze_store::sqlite::SqliteStore>,
{
    let Some(action) = app.take_action() else {
        return Ok(());
    };

    match action {
        AppAction::StartAnalyze { paths, view } => {
            let (targets, input_failures) = resolve_inputs_with_failures(&paths);
            let git = GitCliBackend;
            let metrics = FilesystemMetricsBackend;
            let store = match open_store_fn() {
                Ok(store) => store,
                Err(error) => {
                    app.status_message =
                        format!("Analysis failed: failed to open database: {error}");
                    app.analysis_result = None;
                    app.analysis_table = None;
                    return Ok(());
                }
            };
            let start = std::time::Instant::now();
            let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store, "tui");
            let elapsed = start.elapsed();
            let current_failure_count = input_failures.len() + report.failures.len();

            if !input_failures.is_empty() {
                let mut failures = input_failures;
                failures.extend(report.failures);
                report.failures = failures;
            }

            // Build header + table for all views
            let header = render_analysis_header(&report.repositories, elapsed);
            let table_body = match view {
                AnalyzeView::UsersContribution => {
                    let rows = build_users_contribution_rows(&report.repositories);
                    render_users_contribution_table(&rows)
                }
                AnalyzeView::Activity => {
                    let rows = build_user_activity_rows(&report.repositories);
                    render_user_activity_table(&rows)
                }
                AnalyzeView::All => {
                    let rows = build_users_contribution_rows(&report.repositories);
                    render_users_contribution_table(&rows)
                }
            };
            app.analysis_table = Some(format!("{header}{table_body}"));

            app.set_result(report);
            if current_failure_count > 0 {
                app.status_message =
                    format!("Analysis complete with {current_failure_count} error(s)");
            }
        }
        AppAction::StartCompare(paths) => {
            let (targets, input_failures) = resolve_inputs_with_failures(&paths);
            let git = GitCliBackend;
            let metrics = FilesystemMetricsBackend;
            let store = match open_store_fn() {
                Ok(store) => store,
                Err(error) => {
                    app.status_message =
                        format!("Compare failed: failed to open database: {error}");
                    app.analysis_result = None;
                    app.analysis_table = None;
                    return Ok(());
                }
            };
            let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store, "tui");
            let current_failure_count = input_failures.len() + report.failures.len();

            if !input_failures.is_empty() {
                let mut failures = input_failures;
                failures.extend(report.failures);
                report.failures = failures;
            }

            app.set_result(report);
            if current_failure_count > 0 {
                app.status_message =
                    format!("Analysis complete with {current_failure_count} error(s)");
            }
        }
        AppAction::ShowErrors => {
            app.active_screen = Screen::Errors;
        }
    }

    Ok(())
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
        assert!(app.status_message.contains("failed to open database"));
        assert!(app.pending_action.is_none());
    }
}
