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
use repolyze_core::input::resolve_inputs_with_failures;
use repolyze_core::service::analyze_targets_with_store;
use repolyze_git::backend::GitCliBackend;
use repolyze_metrics::FilesystemMetricsBackend;

use app::{AppAction, AppState, Screen};

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
            event::handle_key(&mut app, key.code);
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
    let Some(action) = app.take_action() else {
        return Ok(());
    };

    match action {
        AppAction::StartAnalyze(paths) | AppAction::StartCompare(paths) => {
            let (targets, input_failures) = resolve_inputs_with_failures(&paths);
            let git = GitCliBackend;
            let metrics = FilesystemMetricsBackend;
            let store = open_store()?;
            let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store);
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
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME must be set"))?;
    let db_path = repolyze_store::path::database_path_from_home(&home);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = repolyze_store::sqlite::SqliteStore::open(&db_path)
        .map_err(|e| anyhow::anyhow!("failed to open database: {e}"))?;
    Ok(store)
}

#[cfg(test)]
mod tests {
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
}
