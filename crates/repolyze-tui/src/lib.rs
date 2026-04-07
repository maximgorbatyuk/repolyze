pub mod app;
pub mod event;
pub mod ui;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    event::{Event, KeyEventKind, poll as poll_event, read as read_event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use repolyze_core::analytics::{
    build_contribution_rows, build_heatmap_data, build_repo_comparison, build_user_activity_rows,
    build_user_effort_data,
};
use repolyze_core::input::{resolve_input, resolve_inputs_with_failures};
use repolyze_core::model::{ComparisonReport, HeatmapData};
use repolyze_core::service::analyze_targets_with_store;
use repolyze_core::settings::Settings;
use repolyze_git::backend::GitCliBackend;
use repolyze_git::branches;
use repolyze_metrics::FilesystemMetricsBackend;
use repolyze_report::markdown::render_markdown;
use repolyze_report::table::{
    ACTIVITY_TITLE, COMPARE_REPOS_TITLE, CONTRIBUTION_TITLE, render_analysis_header,
    render_contribution_table, render_repo_comparison_table, render_user_activity_table,
    render_user_effort_table,
};

use app::{AnalyzeView, AppAction, AppState, GitToolsMode};

struct AnalysisCompletion {
    report: ComparisonReport,
    table_text: String,
    heatmap_data: Option<HeatmapData>,
    failure_count: usize,
    error_message: Option<String>,
    elapsed: Duration,
}

enum BranchListResult {
    Ok(Vec<branches::BranchInfo>),
    Err(String),
}

/// Per-branch deletion progress sent from the background thread.
struct BranchDeleteProgress {
    name: String,
    success: bool,
    done: bool, // true on the final message
}

pub fn run(settings: &Settings) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    let mut bg_receiver: Option<mpsc::Receiver<AnalysisCompletion>> = None;
    let mut branch_list_rx: Option<mpsc::Receiver<BranchListResult>> = None;
    let mut branch_delete_rx: Option<mpsc::Receiver<BranchDeleteProgress>> = None;

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        // Non-blocking poll: 100ms timeout allows spinner animation at ~10fps
        if poll_event(Duration::from_millis(100))?
            && let Event::Key(key) = read_event()?
            && key.kind == KeyEventKind::Press
        {
            event::handle_key(&mut app, key);
        }

        // Advance spinner
        if app.is_loading {
            app.spinner_frame = app.spinner_frame.wrapping_add(1);
        }
        // Also animate spinner during deletion progress
        if branch_delete_rx.is_some() && !app.git_tools.done {
            app.spinner_frame = app.spinner_frame.wrapping_add(1);
        }

        // Start pending actions on background thread
        if let Some(action) = app.take_action() {
            match action {
                AppAction::StartAnalyze { paths, view } => {
                    app.is_loading = true;
                    app.spinner_frame = 0;
                    let settings_clone = settings.clone();
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let result = compute_analysis(paths, view, open_store, &settings_clone);
                        tx.send(result).ok();
                    });
                    bg_receiver = Some(rx);
                }
                AppAction::RenderUserEffort => {
                    render_user_effort_for_selected(&mut app, settings);
                }
                AppAction::LoadMetadata => {
                    app.metadata_text = Some(build_metadata_text(&open_store));
                }
                AppAction::ProbeWorkspace => {
                    app.workspace_info = Some(probe_workspace());
                }
                AppAction::ProbeGitToolsWorkspace => {
                    probe_git_tools_workspace(&mut app);
                }
                AppAction::ExportMarkdown => {
                    export_markdown(&mut app, settings);
                }
                AppAction::ListMergedBranches { base_branch } => {
                    let repos = app.git_tools.selected_repos.clone();
                    if repos.is_empty() {
                        app.git_tools.error = Some("No repositories selected".to_string());
                    } else {
                        app.is_loading = true;
                        app.spinner_frame = 0;
                        let (tx, rx) = mpsc::channel();
                        std::thread::spawn(move || {
                            let mut all_branches = Vec::new();
                            let mut errors = Vec::new();
                            for repo in &repos {
                                match branches::list_merged_branches(repo, &base_branch) {
                                    Ok(list) => all_branches.extend(list),
                                    Err(e) => errors.push(e.to_string()),
                                }
                            }
                            if all_branches.is_empty() && !errors.is_empty() {
                                tx.send(BranchListResult::Err(errors.join("; "))).ok();
                            } else {
                                tx.send(BranchListResult::Ok(all_branches)).ok();
                            }
                        });
                        branch_list_rx = Some(rx);
                    }
                }
                AppAction::ListStaleBranches { days } => {
                    let repos = app.git_tools.selected_repos.clone();
                    if repos.is_empty() {
                        app.git_tools.error = Some("No repositories selected".to_string());
                    } else {
                        app.is_loading = true;
                        app.spinner_frame = 0;
                        let (tx, rx) = mpsc::channel();
                        std::thread::spawn(move || {
                            let mut all_branches = Vec::new();
                            let mut errors = Vec::new();
                            for repo in &repos {
                                match branches::list_stale_branches(repo, days) {
                                    Ok(list) => all_branches.extend(list),
                                    Err(e) => errors.push(e.to_string()),
                                }
                            }
                            if all_branches.is_empty() && !errors.is_empty() {
                                tx.send(BranchListResult::Err(errors.join("; "))).ok();
                            } else {
                                tx.send(BranchListResult::Ok(all_branches)).ok();
                            }
                        });
                        branch_list_rx = Some(rx);
                    }
                }
                AppAction::DeleteBranches => {
                    let branch_list = app.git_tools.branches.clone();
                    let multi_repo = app.git_tools.selected_repos.len() > 1;
                    let force = app.git_tools.mode == Some(GitToolsMode::StaleBranches);
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let total = branch_list.len();
                        for (i, branch) in branch_list.iter().enumerate() {
                            let result = branches::delete_branch(branch, force);
                            let display_name = if multi_repo {
                                format!("[{}] {}", branch.repo_display_name(), result.branch)
                            } else {
                                result.branch
                            };
                            let success =
                                result.local_ok.unwrap_or(true) && result.remote_ok.unwrap_or(true);
                            tx.send(BranchDeleteProgress {
                                name: display_name,
                                success,
                                done: i + 1 == total,
                            })
                            .ok();
                        }
                    });
                    branch_delete_rx = Some(rx);
                }
            }
        }

        // Check for completed analysis
        if let Some(rx) = &bg_receiver {
            match rx.try_recv() {
                Ok(completion) => {
                    apply_analysis_completion(&mut app, completion, settings);
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

        // Check for completed branch listing
        if let Some(rx) = &branch_list_rx {
            match rx.try_recv() {
                Ok(result) => {
                    app.is_loading = false;
                    match result {
                        BranchListResult::Ok(list) => {
                            app.git_tools.branches = list;
                            app.active_screen = app::Screen::GitToolsBranchList;
                        }
                        BranchListResult::Err(msg) => {
                            app.git_tools.error = Some(msg);
                            // Stay on input screen to show error
                        }
                    }
                    branch_list_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.is_loading = false;
                    app.git_tools.error = Some("Branch scan failed unexpectedly".to_string());
                    branch_list_rx = None;
                }
            }
        }

        // Check for branch deletion progress
        if let Some(rx) = &branch_delete_rx {
            // If user cancelled (Esc), drop the receiver and stop processing
            if app.git_tools.done {
                branch_delete_rx = None;
            } else {
                // Drain all available messages this frame
                loop {
                    match rx.try_recv() {
                        Ok(progress) => {
                            let is_done = progress.done;
                            app.git_tools
                                .progress
                                .push((progress.name, progress.success));
                            if is_done {
                                app.git_tools.done = true;
                                branch_delete_rx = None;
                                break;
                            }
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            app.git_tools.done = true;
                            branch_delete_rx = None;
                            break;
                        }
                    }
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
    settings: &Settings,
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
                elapsed: Duration::ZERO,
            };
        }
    };
    let start = std::time::Instant::now();
    let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store, "tui", settings);
    let elapsed = start.elapsed();
    let failure_count = input_failures.len() + report.failures.len();

    if !input_failures.is_empty() {
        let mut failures = input_failures;
        failures.extend(report.failures);
        report.failures = failures;
    }

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let header = render_analysis_header(&report.repositories, elapsed, &cwd);
    let today = repolyze_core::date_util::today_ymd();
    let (table_body, heatmap_data) = match view {
        AnalyzeView::Contribution => {
            let rows = build_contribution_rows(&report.repositories, settings);
            (render_contribution_table(&rows), None)
        }
        AnalyzeView::Activity => {
            let rows = build_user_activity_rows(&report.repositories, settings);
            (render_user_activity_table(&rows), None)
        }
        AnalyzeView::ActivityHeatmap => {
            let hm = build_heatmap_data(&report.repositories, None, &today, settings);
            (String::new(), Some(hm))
        }
        AnalyzeView::UserEffort => {
            // UserEffort needs contributor selection first; table built later
            (String::new(), None)
        }
        AnalyzeView::CompareRepos => {
            let comparison = build_repo_comparison(&report.repositories);
            (render_repo_comparison_table(&comparison), None)
        }
        AnalyzeView::All => {
            let contrib_rows = build_contribution_rows(&report.repositories, settings);
            let activity_rows = build_user_activity_rows(&report.repositories, settings);
            let mut combined = format!("#1 {CONTRIBUTION_TITLE}\n\n");
            combined.push_str(&render_contribution_table(&contrib_rows));
            combined.push_str(&format!("\n\n#2 {ACTIVITY_TITLE}\n\n"));
            combined.push_str(&render_user_activity_table(&activity_rows));
            // Include repo comparison if multi-repo
            if report.repositories.len() > 1 {
                let comparison = build_repo_comparison(&report.repositories);
                let table = render_repo_comparison_table(&comparison);
                if !table.is_empty() {
                    combined.push_str(&format!("\n\n#4 {COMPARE_REPOS_TITLE}\n\n"));
                    combined.push_str(&table);
                }
            }
            let hm = build_heatmap_data(&report.repositories, None, &today, settings);
            (combined, Some(hm))
        }
    };

    AnalysisCompletion {
        report,
        table_text: format!("{header}{table_body}"),
        heatmap_data,
        failure_count,
        error_message: None,
        elapsed,
    }
}

fn apply_analysis_completion(
    app: &mut AppState,
    completion: AnalysisCompletion,
    settings: &Settings,
) {
    app.is_loading = false;
    app.analysis_elapsed = completion.elapsed;

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

    // UserEffort: populate contributor list and transition to selection screen.
    // Use lightweight collection grouped by canonical key (respects user aliases).
    if app.selected_analyze_view == AnalyzeView::UserEffort
        && app.selected_email.is_none()
        && let Some(report) = &app.analysis_result
    {
        let mut key_map: HashMap<String, (String, u64)> = HashMap::new();
        for repo in &report.repositories {
            for cs in &repo.contributions.contributors {
                let key = settings.canonical_key(&cs.email);
                let entry = key_map.entry(key).or_insert_with(|| (cs.name.clone(), 0));
                entry.1 += cs.commits;
            }
        }
        let mut entries: Vec<_> = key_map.into_iter().collect();
        entries.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(a.0.cmp(&b.0)));
        app.contributor_list = entries.into_iter().map(|(e, (n, _))| (e, n)).collect();
        app.contributor_filter.clear();
        app.contributor_selected = 0;
        app.scroll_offset = 0;
        app.active_screen = app::Screen::UserSelect;
    }
}

/// Synchronous execution for tests — keeps the existing test API working.
pub fn execute_pending_action(app: &mut AppState) -> anyhow::Result<()> {
    execute_pending_action_with_store_opener(app, open_store, &Settings::default())
}

fn execute_pending_action_with_store_opener<F>(
    app: &mut AppState,
    open_store_fn: F,
    settings: &Settings,
) -> anyhow::Result<()>
where
    F: Fn() -> anyhow::Result<repolyze_store::sqlite::SqliteStore>,
{
    let Some(action) = app.take_action() else {
        return Ok(());
    };

    match action {
        AppAction::StartAnalyze { paths, view } => {
            let completion = compute_analysis(paths, view, &open_store_fn, settings);
            apply_analysis_completion(app, completion, settings);
        }
        AppAction::RenderUserEffort => {
            render_user_effort_for_selected(app, settings);
        }
        AppAction::LoadMetadata => {
            app.metadata_text = Some(build_metadata_text(&open_store_fn));
        }
        AppAction::ProbeWorkspace => {
            app.workspace_info = Some(probe_workspace());
        }
        AppAction::ProbeGitToolsWorkspace => {
            probe_git_tools_workspace(app);
        }
        AppAction::ExportMarkdown => {
            export_markdown(app, settings);
        }
        AppAction::ListMergedBranches { base_branch } => {
            let repos = &app.git_tools.selected_repos;
            if repos.is_empty() {
                app.git_tools.error = Some("No repositories selected".to_string());
            } else {
                let mut all_branches = Vec::new();
                let mut errors = Vec::new();
                for repo in repos {
                    match branches::list_merged_branches(repo, &base_branch) {
                        Ok(list) => all_branches.extend(list),
                        Err(e) => errors.push(e.to_string()),
                    }
                }
                if all_branches.is_empty() && !errors.is_empty() {
                    app.git_tools.error = Some(errors.join("; "));
                } else {
                    app.git_tools.branches = all_branches;
                    app.active_screen = app::Screen::GitToolsBranchList;
                }
            }
        }
        AppAction::ListStaleBranches { days } => {
            let repos = &app.git_tools.selected_repos;
            if repos.is_empty() {
                app.git_tools.error = Some("No repositories selected".to_string());
            } else {
                let mut all_branches = Vec::new();
                let mut errors = Vec::new();
                for repo in repos {
                    match branches::list_stale_branches(repo, days) {
                        Ok(list) => all_branches.extend(list),
                        Err(e) => errors.push(e.to_string()),
                    }
                }
                if all_branches.is_empty() && !errors.is_empty() {
                    app.git_tools.error = Some(errors.join("; "));
                } else {
                    app.git_tools.branches = all_branches;
                    app.active_screen = app::Screen::GitToolsBranchList;
                }
            }
        }
        AppAction::DeleteBranches => {
            let multi_repo = app.git_tools.selected_repos.len() > 1;
            let force = app.git_tools.mode == Some(GitToolsMode::StaleBranches);
            for branch in &app.git_tools.branches {
                let result = branches::delete_branch(branch, force);
                let display_name = if multi_repo {
                    format!("[{}] {}", branch.repo_display_name(), result.branch)
                } else {
                    result.branch
                };
                let success = result.local_ok.unwrap_or(true) && result.remote_ok.unwrap_or(true);
                app.git_tools.progress.push((display_name, success));
            }
            app.git_tools.done = true;
        }
    }

    Ok(())
}

fn render_user_effort_for_selected(app: &mut AppState, settings: &Settings) {
    let identifier = match &app.selected_email {
        Some(e) => e.clone(),
        None => return,
    };
    let repos = match &app.analysis_result {
        Some(report) => &report.repositories,
        None => return,
    };

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let header = render_analysis_header(repos, app.analysis_elapsed, &cwd);

    let today = repolyze_core::date_util::today_ymd();

    if let Some(effort) = build_user_effort_data(repos, &identifier, settings) {
        let table_body = render_user_effort_table(&effort);
        app.analysis_table = Some(format!("{header}{table_body}"));
        let hm = build_heatmap_data(repos, Some(&identifier), &today, settings);
        app.heatmap_data = Some(hm);
    } else {
        app.analysis_table = Some(format!("{header}No data found for {identifier}"));
        app.heatmap_data = None;
    }
    app.scroll_offset = 0;
}

fn export_markdown(app: &mut AppState, settings: &Settings) {
    let report = match &app.analysis_result {
        Some(r) => r,
        None => {
            app.status_message = "No report to export".to_string();
            return;
        }
    };

    let markdown = render_markdown(report, settings);
    let date = repolyze_core::date_util::today_ymd();
    let time_suffix = {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            % 86400;
        format!(
            "{:02}{:02}{:02}",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        )
    };
    let filename = format!("repolyze-report-{date}-{time_suffix}.md");
    let filepath = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(&filename);

    match std::fs::write(&filepath, &markdown) {
        Ok(()) => {
            app.status_message = format!("Exported to {}", filepath.display());
        }
        Err(e) => {
            app.status_message = format!("Export failed: {e}");
        }
    }
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

fn probe_workspace() -> app::WorkspaceInfo {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let folder = cwd.to_string_lossy().to_string();

    match resolve_input(&cwd) {
        Ok(targets) => {
            let repo_count = targets.len();
            let is_single_repo = repo_count == 1
                && targets[0].root == cwd.canonicalize().unwrap_or_else(|_| cwd.clone());
            app::WorkspaceInfo {
                folder,
                is_single_repo,
                repo_count,
            }
        }
        Err(_) => app::WorkspaceInfo {
            folder,
            is_single_repo: false,
            repo_count: 0,
        },
    }
}

fn probe_git_tools_workspace(app: &mut AppState) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match resolve_input(&cwd) {
        Ok(targets) => {
            let repos: Vec<PathBuf> = targets.into_iter().map(|t| t.root).collect();
            if repos.len() == 1 {
                app.git_tools.selected_repos = repos.clone();
                app.git_tools.repo_checked = vec![true];
            } else {
                app.git_tools.repo_checked = vec![false; repos.len()];
            }
            app.git_tools.repos = repos;
        }
        Err(_) => {
            app.git_tools.workspace_error =
                Some("No git repositories found in this directory.".to_string());
        }
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

        let result = execute_pending_action_with_store_opener(
            &mut app,
            || Err(anyhow!("boom")),
            &Settings::default(),
        );

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

        execute_pending_action_with_store_opener(
            &mut app,
            move || {
                repolyze_store::sqlite::SqliteStore::open(&db_path_clone)
                    .map_err(|e| anyhow!("{e}"))
            },
            &Settings::default(),
        )
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

        execute_pending_action_with_store_opener(
            &mut app,
            || Err(anyhow!("boom")),
            &Settings::default(),
        )
        .unwrap();

        let text = app.metadata_text.as_ref().unwrap();
        assert!(text.contains("Failed to open database"));
    }

    #[test]
    fn export_markdown_without_result_sets_error_message() {
        let mut app = AppState::new();
        export_markdown(&mut app, &Settings::default());
        assert_eq!(app.status_message, "No report to export");
    }

    #[test]
    fn export_markdown_writes_file_and_updates_status() {
        let dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut app = AppState::new();
        app.analysis_result = Some(ComparisonReport {
            repositories: vec![],
            summary: repolyze_core::model::ComparisonSummary {
                total_contributors: 0,
                total_commits: 0,
                total_lines_changed: 0,
                total_files: 0,
            },
            failures: vec![],
        });

        export_markdown(&mut app, &Settings::default());

        // Restore cwd before assertions so cleanup works even on failure
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(app.status_message.starts_with("Exported to "));
        assert!(app.status_message.ends_with(".md"));

        // Verify the file was created in the tempdir
        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .collect();
        assert_eq!(files.len(), 1);

        let content = std::fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("# Repolyze Analysis Report"));
    }
}
