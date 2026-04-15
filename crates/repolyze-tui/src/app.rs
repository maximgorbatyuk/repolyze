use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use repolyze_core::model::{
    BarChartData, ComparisonReport, HeatmapData, PartialFailure, TimelineData,
};
use repolyze_git::branches::BranchInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Home,
    Help,
    AnalyzeMenu,
    Analyze,
    Metadata,
    UserSelect,
    GitToolsMenu,
    GitToolsRepoSelect,
    GitToolsInput,
    GitToolsBranchList,
    GitToolsProgress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyzeView {
    All,
    Contribution,
    Activity,
    ActivityHeatmap,
    WeekdayChart,
    HourlyChart,
    TimelineChart,
    UserEffort,
    CompareRepos,
}

pub const ANALYZE_MENU_ITEMS: [(&str, AnalyzeView); 9] = [
    ("Full report", AnalyzeView::All),
    ("Contribution", AnalyzeView::Contribution),
    ("Most active days and hours", AnalyzeView::Activity),
    ("Activity heatmap", AnalyzeView::ActivityHeatmap),
    ("Commits by weekday", AnalyzeView::WeekdayChart),
    ("Commits by hour", AnalyzeView::HourlyChart),
    ("Commit timeline", AnalyzeView::TimelineChart),
    ("User effort", AnalyzeView::UserEffort),
    ("Compare repositories", AnalyzeView::CompareRepos),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    Analyze,
    GitTools,
    Help,
    Metadata,
}

impl MenuItem {
    pub fn description(&self) -> &'static str {
        match self {
            MenuItem::Analyze => "Analyze one or more repositories",
            MenuItem::GitTools => "Git repository maintenance tools",
            MenuItem::Help => "Keybindings and usage guide",
            MenuItem::Metadata => "Database info and table row counts",
        }
    }

    pub fn screen(&self) -> Screen {
        match self {
            MenuItem::Analyze => Screen::AnalyzeMenu,
            MenuItem::GitTools => Screen::GitToolsMenu,
            MenuItem::Help => Screen::Help,
            MenuItem::Metadata => Screen::Metadata,
        }
    }
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::Analyze => write!(f, "Analyze"),
            MenuItem::GitTools => write!(f, "Git Tools"),
            MenuItem::Help => write!(f, "Help"),
            MenuItem::Metadata => write!(f, "Metadata"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitToolsMode {
    MergedBranches,
    StaleBranches,
}

pub const GIT_TOOLS_MENU_ITEMS: [(&str, &str, GitToolsMode); 2] = [
    (
        "Remove merged branches",
        "Delete branches already merged into a base branch",
        GitToolsMode::MergedBranches,
    ),
    (
        "Remove stale branches",
        "Delete branches with no activity for N days",
        GitToolsMode::StaleBranches,
    ),
];

/// Rich progress info for a single branch deletion.
#[derive(Debug, Clone)]
pub struct BranchProgress {
    pub name: String,
    pub local_ok: Option<bool>,
    pub remote_ok: Option<bool>,
    pub processed: bool,
}

#[derive(Debug, Clone)]
pub struct GitToolsState {
    pub selected: usize,
    pub mode: Option<GitToolsMode>,
    pub input: String,
    pub branches: Vec<BranchInfo>,
    /// Protected branches found in selected repos: (repo_display_name, branch_name).
    pub protected_branches: Vec<(String, String)>,
    pub progress: Vec<BranchProgress>,
    /// Index of the branch currently being processed by the background thread.
    pub current_index: usize,
    /// Display name of the repo currently being processed.
    pub current_repo: String,
    pub done: bool,
    pub error: Option<String>,
    pub scroll: u16,
    // Workspace discovery
    pub repos: Vec<PathBuf>,
    pub repo_checked: Vec<bool>,
    pub selected_repos: Vec<PathBuf>,
    /// Cursor position in repo picker. 0 = "Select all" row, 1.. = individual repos.
    pub repo_select_idx: usize,
    pub workspace_error: Option<String>,
}

impl Default for GitToolsState {
    fn default() -> Self {
        Self::new()
    }
}

impl GitToolsState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            mode: None,
            input: String::new(),
            branches: Vec::new(),
            protected_branches: Vec::new(),
            progress: Vec::new(),
            current_index: 0,
            current_repo: String::new(),
            done: false,
            error: None,
            scroll: 0,
            repos: Vec::new(),
            repo_checked: Vec::new(),
            selected_repos: Vec::new(),
            repo_select_idx: 0,
            workspace_error: None,
        }
    }

    /// Full reset — used when leaving Git Tools entirely.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Reset tool-specific state but keep workspace info (repos, repo_checked, selected_repos)
    /// and menu cursor (`selected`) so the user returns to the same menu position.
    pub fn clear_tool(&mut self) {
        self.mode = None;
        self.input.clear();
        self.branches.clear();
        self.protected_branches.clear();
        self.progress.clear();
        self.current_index = 0;
        self.current_repo.clear();
        self.done = false;
        self.error = None;
        self.scroll = 0;
        self.repo_select_idx = 0;
    }

    pub fn menu_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn menu_down(&mut self) {
        if self.selected + 1 < GIT_TOOLS_MENU_ITEMS.len() {
            self.selected += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, content_height: u16, visible_height: u16) {
        let max_offset = content_height.saturating_sub(visible_height);
        if self.scroll < max_offset {
            self.scroll += 1;
        }
    }

    pub fn repo_select_up(&mut self) {
        if self.repo_select_idx > 0 {
            self.repo_select_idx -= 1;
        }
    }

    pub fn repo_select_down(&mut self) {
        // Row 0 = "Select all", rows 1..=repos.len() = individual repos
        if self.repo_select_idx < self.repos.len() {
            self.repo_select_idx += 1;
        }
    }

    /// Toggle checkbox for the repo at the current cursor position.
    pub fn toggle_repo(&mut self) {
        if self.repo_select_idx == 0 {
            self.toggle_all_repos();
        } else {
            let i = self.repo_select_idx - 1;
            if i < self.repo_checked.len() {
                self.repo_checked[i] = !self.repo_checked[i];
            }
        }
    }

    /// If all repos are checked, uncheck all; otherwise check all.
    pub fn toggle_all_repos(&mut self) {
        let all_checked = self.repo_checked.iter().all(|c| *c);
        let new_val = !all_checked;
        for c in &mut self.repo_checked {
            *c = new_val;
        }
    }

    /// Returns true if all repos are checked.
    pub fn all_repos_checked(&self) -> bool {
        !self.repo_checked.is_empty() && self.repo_checked.iter().all(|c| *c)
    }

    /// Keep the selected repo visible in the scrollable viewport.
    pub fn ensure_repo_visible(&mut self, visible_height: u16) {
        // 2 header lines (title, blank), 2 footer lines (blank, hints)
        let header: u16 = 2;
        let footer: u16 = 2;
        let item_line = header + self.repo_select_idx as u16;
        let visible = visible_height.saturating_sub(footer);
        if visible > 0 && item_line >= self.scroll + visible {
            self.scroll = item_line - visible + 1;
        } else if item_line < self.scroll {
            self.scroll = item_line;
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub folder: String,
    pub is_single_repo: bool,
    pub repo_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    StartAnalyze {
        paths: Vec<String>,
        view: AnalyzeView,
    },
    RenderUserEffort,
    LoadMetadata,
    ProbeWorkspace,
    ListMergedBranches {
        base_branch: String,
    },
    ListStaleBranches {
        days: u64,
    },
    DeleteBranches,
    ProbeGitToolsWorkspace,
    ExportMarkdown,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub menu_items: Vec<MenuItem>,
    pub selected: usize,
    pub active_screen: Screen,
    pub should_quit: bool,
    pub analysis_result: Option<ComparisonReport>,
    pub errors: Vec<PartialFailure>,
    pub pending_action: Option<AppAction>,
    pub input_buffer: String,
    pub input_paths: Vec<String>,
    pub status_message: String,
    pub analyze_menu_selected: usize,
    pub selected_analyze_view: AnalyzeView,
    pub analysis_table: Option<String>,
    pub heatmap_data: Option<HeatmapData>,
    pub metadata_text: Option<String>,
    pub workspace_info: Option<WorkspaceInfo>,
    pub is_loading: bool,
    pub spinner_frame: usize,
    pub progress_log: Vec<String>,
    pub scroll_offset: u16,
    pub content_height: u16,
    pub visible_height: u16,
    pub contributor_list: Vec<(String, String)>,
    pub contributor_filter: String,
    pub contributor_selected: usize,
    pub selected_email: Option<String>,
    pub analysis_elapsed: Duration,
    pub weekday_chart: Option<BarChartData>,
    pub hourly_chart: Option<BarChartData>,
    pub timeline_data: Option<TimelineData>,
    pub git_tools: GitToolsState,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            menu_items: vec![
                MenuItem::Analyze,
                MenuItem::GitTools,
                MenuItem::Help,
                MenuItem::Metadata,
            ],
            selected: 0,
            active_screen: Screen::Home,
            should_quit: false,
            analysis_result: None,
            errors: Vec::new(),
            pending_action: None,
            input_buffer: String::new(),
            input_paths: Vec::new(),
            status_message: "Ready".to_string(),
            analyze_menu_selected: 0,
            selected_analyze_view: AnalyzeView::All,
            analysis_table: None,
            heatmap_data: None,
            metadata_text: None,
            workspace_info: None,
            is_loading: false,
            spinner_frame: 0,
            progress_log: Vec::new(),
            scroll_offset: 0,
            content_height: 0,
            visible_height: 0,
            contributor_list: Vec::new(),
            contributor_filter: String::new(),
            contributor_selected: 0,
            selected_email: None,
            analysis_elapsed: Duration::ZERO,
            weekday_chart: None,
            hourly_chart: None,
            timeline_data: None,
            git_tools: GitToolsState::new(),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.menu_items.len() {
            self.selected += 1;
        }
    }

    pub fn activate_selected(&mut self) {
        if let Some(item) = self.menu_items.get(self.selected) {
            let screen = item.screen();
            match screen {
                Screen::Metadata => {
                    self.pending_action = Some(AppAction::LoadMetadata);
                }
                Screen::AnalyzeMenu => {
                    self.pending_action = Some(AppAction::ProbeWorkspace);
                }
                Screen::GitToolsMenu => {
                    self.git_tools.clear();
                    self.pending_action = Some(AppAction::ProbeGitToolsWorkspace);
                }
                _ => {}
            }
            self.active_screen = screen;
        }
    }

    pub fn go_home(&mut self) {
        self.active_screen = Screen::Home;
        self.input_buffer.clear();
        self.input_paths.clear();
        self.heatmap_data = None;
        self.weekday_chart = None;
        self.hourly_chart = None;
        self.timeline_data = None;
        self.metadata_text = None;
        self.workspace_info = None;
        self.is_loading = false;
        self.spinner_frame = 0;
        self.scroll_offset = 0;
        self.status_message = "Ready".to_string();
        self.contributor_list.clear();
        self.contributor_filter.clear();
        self.contributor_selected = 0;
        self.selected_email = None;
        self.git_tools.clear();
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn dispatch_analyze(&mut self) {
        if !self.input_paths.is_empty() {
            self.analysis_table = None;
            self.heatmap_data = None;
            self.weekday_chart = None;
            self.hourly_chart = None;
            self.timeline_data = None;
            self.pending_action = Some(AppAction::StartAnalyze {
                paths: self.input_paths.clone(),
                view: self.selected_analyze_view.clone(),
            });
        }
    }

    pub fn select_analyze_view(&mut self) {
        if let Some((_, view)) = ANALYZE_MENU_ITEMS.get(self.analyze_menu_selected) {
            self.selected_analyze_view = view.clone();
            self.analysis_result = None;
            self.analysis_table = None;
            self.heatmap_data = None;
            self.weekday_chart = None;
            self.hourly_chart = None;
            self.timeline_data = None;
            self.scroll_offset = 0;
            self.input_paths.clear();
            self.input_buffer.clear();
            self.input_paths.push(".".to_string());
            self.active_screen = Screen::Analyze;
            self.dispatch_analyze();
        }
    }

    pub fn effective_menu_len(&self) -> usize {
        let is_multi = self
            .workspace_info
            .as_ref()
            .is_some_and(|w| !w.is_single_repo && w.repo_count > 1);
        if is_multi {
            ANALYZE_MENU_ITEMS.len() // includes "Compare repositories"
        } else {
            ANALYZE_MENU_ITEMS.len() - 1 // hide "Compare repositories"
        }
    }

    pub fn analyze_menu_up(&mut self) {
        if self.analyze_menu_selected > 0 {
            self.analyze_menu_selected -= 1;
        }
    }

    pub fn analyze_menu_down(&mut self) {
        if self.analyze_menu_selected + 1 < self.effective_menu_len() {
            self.analyze_menu_selected += 1;
        }
    }

    pub fn take_action(&mut self) -> Option<AppAction> {
        self.pending_action.take()
    }

    pub fn set_result(&mut self, report: ComparisonReport) {
        self.errors.clear();
        self.errors.extend(report.failures.iter().cloned());
        self.analysis_result = Some(report);
        self.input_paths.clear();
        self.input_buffer.clear();
        self.status_message = "Analysis complete".to_string();
    }

    pub fn add_input_path(&mut self) {
        let path = self.input_buffer.trim().to_string();
        if !path.is_empty() {
            self.input_paths.push(path);
            self.input_buffer.clear();
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        let max_offset = self.content_height.saturating_sub(self.visible_height);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }

    pub fn request_export(&mut self) {
        if self.analysis_result.is_some() && !self.is_loading {
            self.pending_action = Some(AppAction::ExportMarkdown);
        }
    }

    pub fn filtered_contributors(&self) -> Vec<&(String, String)> {
        if self.contributor_filter.is_empty() {
            return self.contributor_list.iter().collect();
        }
        let filter = self.contributor_filter.to_lowercase();
        self.contributor_list
            .iter()
            .filter(|(email, name)| {
                email.to_lowercase().contains(&filter) || name.to_lowercase().contains(&filter)
            })
            .collect()
    }

    pub fn select_contributor(&mut self) {
        let filtered = self.filtered_contributors();
        if let Some((email, _)) = filtered.get(self.contributor_selected) {
            self.selected_email = Some(email.to_string());
            self.active_screen = Screen::Analyze;
            self.pending_action = Some(AppAction::RenderUserEffort);
        }
    }

    pub fn contributor_select_up(&mut self) {
        if self.contributor_selected > 0 {
            self.contributor_selected -= 1;
        }
        self.ensure_contributor_visible();
    }

    pub fn contributor_select_down(&mut self) {
        let len = self.filtered_contributors().len();
        if self.contributor_selected + 1 < len {
            self.contributor_selected += 1;
        }
        self.ensure_contributor_visible();
    }

    // --- Git Tools ---

    pub fn git_tools_select(&mut self) {
        if self.git_tools.workspace_error.is_some() {
            return;
        }
        let (_, _, mode) = &GIT_TOOLS_MENU_ITEMS[self.git_tools.selected];
        let mode = mode.clone();
        self.git_tools.clear_tool();
        self.git_tools.mode = Some(mode.clone());
        // Pre-fill default for stale branches
        if mode == GitToolsMode::StaleBranches {
            self.git_tools.input = "90".to_string();
        }
        // Multi-repo: show repo picker first if no repos selected yet
        if self.git_tools.repos.len() > 1 && self.git_tools.selected_repos.is_empty() {
            self.active_screen = Screen::GitToolsRepoSelect;
        } else {
            self.active_screen = Screen::GitToolsInput;
        }
    }

    /// Confirm repo selection: collect all checked repos into `selected_repos`.
    pub fn git_tools_select_repo(&mut self) {
        let checked: Vec<PathBuf> = self
            .git_tools
            .repos
            .iter()
            .zip(self.git_tools.repo_checked.iter())
            .filter(|(_, checked)| **checked)
            .map(|(path, _)| path.clone())
            .collect();
        if checked.is_empty() {
            return; // nothing selected, stay on picker
        }
        self.git_tools.selected_repos = checked;
        self.active_screen = Screen::GitToolsInput;
    }

    pub fn git_tools_submit_input(&mut self) {
        match &self.git_tools.mode {
            Some(GitToolsMode::MergedBranches) => {
                let base = self.git_tools.input.trim().to_string();
                if base.is_empty() {
                    return;
                }
                self.pending_action = Some(AppAction::ListMergedBranches { base_branch: base });
            }
            Some(GitToolsMode::StaleBranches) => {
                let input = self.git_tools.input.trim();
                let days: u64 = if input.is_empty() {
                    90
                } else {
                    match input.parse() {
                        Ok(d) if d > 0 => d,
                        _ => return, // invalid input, ignore
                    }
                };
                self.pending_action = Some(AppAction::ListStaleBranches { days });
            }
            None => {}
        }
    }

    pub fn git_tools_confirm_delete(&mut self) {
        if !self.git_tools.branches.is_empty() {
            let multi_repo = self.git_tools.selected_repos.len() > 1;
            self.git_tools.progress = self
                .git_tools
                .branches
                .iter()
                .map(|b| {
                    let name = if multi_repo {
                        format!("[{}] {}", b.repo_display_name(), b.name)
                    } else {
                        b.name.clone()
                    };
                    BranchProgress {
                        name,
                        local_ok: None,
                        remote_ok: None,
                        processed: false,
                    }
                })
                .collect();
            self.git_tools.current_index = 0;
            self.git_tools.current_repo.clear();
            self.git_tools.done = false;
            self.git_tools.scroll = 0;
            self.active_screen = Screen::GitToolsProgress;
            self.pending_action = Some(AppAction::DeleteBranches);
        }
    }

    pub fn git_tools_scroll_down(&mut self) {
        self.git_tools
            .scroll_down(self.content_height, self.visible_height);
    }

    fn ensure_contributor_visible(&mut self) {
        // 4 header lines (title, blank, filter, blank), 2 footer lines (blank, hints)
        let header_lines: u16 = 4;
        let footer_lines: u16 = 2;
        let item_line = header_lines + self.contributor_selected as u16;
        let visible = self.visible_height.saturating_sub(footer_lines);
        if visible > 0 && item_line >= self.scroll_offset + visible {
            self.scroll_offset = item_line - visible + 1;
        } else if item_line < self.scroll_offset {
            self.scroll_offset = item_line;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_on_home_screen() {
        let app = AppState::new();
        assert_eq!(app.active_screen, Screen::Home);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn starts_with_all_menu_items() {
        let app = AppState::new();
        assert_eq!(
            app.menu_items,
            vec![
                MenuItem::Analyze,
                MenuItem::GitTools,
                MenuItem::Help,
                MenuItem::Metadata,
            ]
        );
    }

    #[test]
    fn navigate_down_and_activate_analyze() {
        let mut app = AppState::new();
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::AnalyzeMenu);
    }

    #[test]
    fn navigate_to_git_tools() {
        let mut app = AppState::new();
        app.move_down();
        assert_eq!(app.selected, 1);
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::GitToolsMenu);
        assert_eq!(app.pending_action, Some(AppAction::ProbeGitToolsWorkspace));
    }

    #[test]
    fn navigate_to_help() {
        let mut app = AppState::new();
        app.move_down();
        app.move_down();
        assert_eq!(app.selected, 2);
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Help);
    }

    #[test]
    fn navigate_to_metadata() {
        let mut app = AppState::new();
        app.move_down();
        app.move_down();
        app.move_down();
        assert_eq!(app.selected, 3);
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Metadata);
        assert_eq!(app.pending_action, Some(AppAction::LoadMetadata));
    }

    #[test]
    fn move_down_does_not_overflow() {
        let mut app = AppState::new();
        for _ in 0..10 {
            app.move_down();
        }
        assert_eq!(app.selected, 3);
    }

    #[test]
    fn move_up_does_not_underflow() {
        let mut app = AppState::new();
        app.move_up();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = AppState::new();
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn go_home_returns_to_home_screen() {
        let mut app = AppState::new();
        app.activate_selected();
        assert_ne!(app.active_screen, Screen::Home);
        app.go_home();
        assert_eq!(app.active_screen, Screen::Home);
    }

    #[test]
    fn dispatch_analyze_creates_action() {
        let mut app = AppState::new();
        app.input_paths.push("/tmp/repo".to_string());
        app.dispatch_analyze();
        assert_eq!(
            app.pending_action,
            Some(AppAction::StartAnalyze {
                paths: vec!["/tmp/repo".to_string()],
                view: AnalyzeView::All,
            })
        );
    }

    #[test]
    fn dispatch_analyze_ignored_without_paths() {
        let mut app = AppState::new();
        app.dispatch_analyze();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn take_action_clears_pending() {
        let mut app = AppState::new();
        app.input_paths.push("/tmp/repo".to_string());
        app.dispatch_analyze();
        let action = app.take_action();
        assert!(action.is_some());
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn add_input_path_from_buffer() {
        let mut app = AppState::new();
        app.input_buffer = "/tmp/repo".to_string();
        app.add_input_path();
        assert_eq!(app.input_paths, vec!["/tmp/repo".to_string()]);
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn add_empty_input_path_is_ignored() {
        let mut app = AppState::new();
        app.input_buffer = "   ".to_string();
        app.add_input_path();
        assert!(app.input_paths.is_empty());
    }

    #[test]
    fn menu_item_screen_mapping() {
        assert_eq!(MenuItem::Help.screen(), Screen::Help);
        assert_eq!(MenuItem::Analyze.screen(), Screen::AnalyzeMenu);
        assert_eq!(MenuItem::GitTools.screen(), Screen::GitToolsMenu);
        assert_eq!(MenuItem::Metadata.screen(), Screen::Metadata);
    }

    #[test]
    fn analyze_menu_opens_before_path_entry() {
        let mut app = AppState::new();
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::AnalyzeMenu);
    }

    #[test]
    fn analyze_contribution_dispatches_immediately() {
        let mut app = AppState::new();
        app.active_screen = Screen::AnalyzeMenu;
        app.analyze_menu_selected = 1;
        app.select_analyze_view();
        assert_eq!(app.active_screen, Screen::Analyze);
        assert_eq!(app.selected_analyze_view, AnalyzeView::Contribution);
        assert_eq!(
            app.pending_action,
            Some(AppAction::StartAnalyze {
                paths: vec![".".to_string()],
                view: AnalyzeView::Contribution,
            })
        );
    }

    #[test]
    fn select_analyze_view_clears_stale_analysis_state() {
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
            trends: repolyze_core::model::TrendsData::default(),
        });
        app.analysis_table = Some("stale".to_string());
        app.input_buffer = "old".to_string();
        app.input_paths.push("/tmp/old".to_string());
        app.analyze_menu_selected = 2;
        app.select_analyze_view();
        assert_eq!(app.active_screen, Screen::Analyze);
        assert!(app.analysis_result.is_none());
        assert!(app.input_buffer.is_empty());
        assert!(app.pending_action.is_some());
    }

    #[test]
    fn menu_items_have_descriptions() {
        for item in &[
            MenuItem::Analyze,
            MenuItem::GitTools,
            MenuItem::Help,
            MenuItem::Metadata,
        ] {
            assert!(!item.description().is_empty());
        }
    }

    #[test]
    fn user_effort_menu_item_exists() {
        let found = ANALYZE_MENU_ITEMS
            .iter()
            .any(|(label, view)| *label == "User effort" && *view == AnalyzeView::UserEffort);
        assert!(found);
    }

    #[test]
    fn filtered_contributors_matches_email_and_name() {
        let mut app = AppState::new();
        app.contributor_list = vec![
            ("alice@example.com".to_string(), "Alice".to_string()),
            ("bob@example.com".to_string(), "Bob".to_string()),
        ];

        app.contributor_filter = "ali".to_string();
        let filtered = app.filtered_contributors();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "alice@example.com");

        app.contributor_filter = "Bob".to_string();
        let filtered = app.filtered_contributors();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "bob@example.com");

        app.contributor_filter.clear();
        assert_eq!(app.filtered_contributors().len(), 2);
    }

    #[test]
    fn select_contributor_sets_state() {
        let mut app = AppState::new();
        app.contributor_list = vec![
            ("alice@example.com".to_string(), "Alice".to_string()),
            ("bob@example.com".to_string(), "Bob".to_string()),
        ];
        app.active_screen = Screen::UserSelect;
        app.contributor_selected = 1;
        app.select_contributor();

        assert_eq!(app.selected_email, Some("bob@example.com".to_string()));
        assert_eq!(app.active_screen, Screen::Analyze);
        assert_eq!(app.pending_action, Some(AppAction::RenderUserEffort));
    }

    #[test]
    fn git_tools_select_merged_transitions_to_input() {
        let mut app = AppState::new();
        app.active_screen = Screen::GitToolsMenu;
        app.git_tools.repos = vec![PathBuf::from("/tmp/repo")];
        app.git_tools.selected_repos = vec![PathBuf::from("/tmp/repo")];
        app.git_tools.selected = 0;
        app.git_tools_select();
        assert_eq!(app.active_screen, Screen::GitToolsInput);
        assert_eq!(app.git_tools.mode, Some(GitToolsMode::MergedBranches));
        assert!(app.git_tools.input.is_empty());
    }

    #[test]
    fn git_tools_select_stale_prefills_default() {
        let mut app = AppState::new();
        app.active_screen = Screen::GitToolsMenu;
        app.git_tools.repos = vec![PathBuf::from("/tmp/repo")];
        app.git_tools.selected_repos = vec![PathBuf::from("/tmp/repo")];
        app.git_tools.selected = 1;
        app.git_tools_select();
        assert_eq!(app.active_screen, Screen::GitToolsInput);
        assert_eq!(app.git_tools.mode, Some(GitToolsMode::StaleBranches));
        assert_eq!(app.git_tools.input, "90");
    }

    #[test]
    fn git_tools_submit_merged_creates_action() {
        let mut app = AppState::new();
        app.git_tools.mode = Some(GitToolsMode::MergedBranches);
        app.git_tools.input = "main".to_string();
        app.git_tools_submit_input();
        assert_eq!(
            app.pending_action,
            Some(AppAction::ListMergedBranches {
                base_branch: "main".to_string()
            })
        );
    }

    #[test]
    fn git_tools_submit_stale_creates_action() {
        let mut app = AppState::new();
        app.git_tools.mode = Some(GitToolsMode::StaleBranches);
        app.git_tools.input = "60".to_string();
        app.git_tools_submit_input();
        assert_eq!(
            app.pending_action,
            Some(AppAction::ListStaleBranches { days: 60 })
        );
    }

    #[test]
    fn git_tools_submit_empty_merged_is_ignored() {
        let mut app = AppState::new();
        app.git_tools.mode = Some(GitToolsMode::MergedBranches);
        app.git_tools.input = "  ".to_string();
        app.git_tools_submit_input();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn git_tools_submit_invalid_days_is_ignored() {
        let mut app = AppState::new();
        app.git_tools.mode = Some(GitToolsMode::StaleBranches);
        app.git_tools.input = "abc".to_string();
        app.git_tools_submit_input();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn git_tools_clear_resets_state() {
        let mut app = AppState::new();
        app.git_tools.mode = Some(GitToolsMode::MergedBranches);
        app.git_tools.input = "main".to_string();
        app.git_tools.done = true;
        app.git_tools.repos = vec![PathBuf::from("/tmp/repo")];
        app.git_tools.clear();
        assert!(app.git_tools.mode.is_none());
        assert!(app.git_tools.input.is_empty());
        assert!(!app.git_tools.done);
        assert!(app.git_tools.repos.is_empty());
    }

    #[test]
    fn git_tools_clear_tool_preserves_repos() {
        let mut app = AppState::new();
        app.git_tools.repos = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        app.git_tools.selected_repos = vec![PathBuf::from("/tmp/a")];
        app.git_tools.mode = Some(GitToolsMode::MergedBranches);
        app.git_tools.input = "main".to_string();
        app.git_tools.done = true;
        app.git_tools.clear_tool();
        // Tool state is reset
        assert!(app.git_tools.mode.is_none());
        assert!(app.git_tools.input.is_empty());
        assert!(!app.git_tools.done);
        // Workspace state is preserved
        assert_eq!(app.git_tools.repos.len(), 2);
        assert_eq!(app.git_tools.selected_repos, vec![PathBuf::from("/tmp/a")]);
    }

    #[test]
    fn git_tools_select_blocked_on_workspace_error() {
        let mut app = AppState::new();
        app.active_screen = Screen::GitToolsMenu;
        app.git_tools.workspace_error = Some("No repos".to_string());
        app.git_tools_select();
        // Should stay on menu, not transition
        assert_eq!(app.active_screen, Screen::GitToolsMenu);
    }

    #[test]
    fn git_tools_multi_repo_shows_picker() {
        let mut app = AppState::new();
        app.active_screen = Screen::GitToolsMenu;
        app.git_tools.repos = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        // No selected_repos yet
        app.git_tools.selected = 0;
        app.git_tools_select();
        assert_eq!(app.active_screen, Screen::GitToolsRepoSelect);
    }

    #[test]
    fn git_tools_multi_repo_skips_picker_when_repo_already_selected() {
        let mut app = AppState::new();
        app.active_screen = Screen::GitToolsMenu;
        app.git_tools.repos = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        app.git_tools.selected_repos = vec![PathBuf::from("/tmp/a")];
        app.git_tools.selected = 0;
        app.git_tools_select();
        assert_eq!(app.active_screen, Screen::GitToolsInput);
    }

    #[test]
    fn git_tools_select_repo_collects_checked_and_transitions() {
        let mut app = AppState::new();
        app.git_tools.repos = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        app.git_tools.repo_checked = vec![false, true];
        app.git_tools.mode = Some(GitToolsMode::MergedBranches);
        app.active_screen = Screen::GitToolsRepoSelect;
        app.git_tools_select_repo();
        assert_eq!(app.git_tools.selected_repos, vec![PathBuf::from("/tmp/b")]);
        assert_eq!(app.active_screen, Screen::GitToolsInput);
    }

    #[test]
    fn git_tools_select_repo_does_nothing_when_none_checked() {
        let mut app = AppState::new();
        app.git_tools.repos = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        app.git_tools.repo_checked = vec![false, false];
        app.active_screen = Screen::GitToolsRepoSelect;
        app.git_tools_select_repo();
        assert!(app.git_tools.selected_repos.is_empty());
        assert_eq!(app.active_screen, Screen::GitToolsRepoSelect);
    }

    #[test]
    fn toggle_repo_toggles_individual() {
        let mut state = GitToolsState::new();
        state.repos = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        state.repo_checked = vec![false, false];
        state.repo_select_idx = 1; // first repo (idx 0 = select all)
        state.toggle_repo();
        assert_eq!(state.repo_checked, vec![true, false]);
        state.toggle_repo();
        assert_eq!(state.repo_checked, vec![false, false]);
    }

    #[test]
    fn toggle_repo_on_select_all_row() {
        let mut state = GitToolsState::new();
        state.repos = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        state.repo_checked = vec![false, false];
        state.repo_select_idx = 0; // "Select all" row
        state.toggle_repo();
        assert_eq!(state.repo_checked, vec![true, true]);
        state.toggle_repo();
        assert_eq!(state.repo_checked, vec![false, false]);
    }

    #[test]
    fn request_export_ignored_without_analysis_result() {
        let mut app = AppState::new();
        app.request_export();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn request_export_ignored_while_loading() {
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
            trends: repolyze_core::model::TrendsData::default(),
        });
        app.is_loading = true;
        app.request_export();
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn request_export_sets_pending_action() {
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
            trends: repolyze_core::model::TrendsData::default(),
        });
        app.request_export();
        assert_eq!(app.pending_action, Some(AppAction::ExportMarkdown));
    }
}
