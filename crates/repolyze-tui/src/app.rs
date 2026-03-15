use std::fmt;
use std::path::PathBuf;

use repolyze_core::model::{ComparisonReport, PartialFailure};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Home,
    Help,
    AnalyzeMenu,
    Analyze,
    Compare,
    Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyzeView {
    All,
    UsersContribution,
    Activity,
}

pub const ANALYZE_MENU_ITEMS: [(&str, AnalyzeView); 3] = [
    ("All (full report)", AnalyzeView::All),
    ("Users contribution", AnalyzeView::UsersContribution),
    ("Most active days and hours", AnalyzeView::Activity),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    Analyze,
    Compare,
    Help,
    Metadata,
}

impl MenuItem {
    pub fn description(&self) -> &'static str {
        match self {
            MenuItem::Analyze => "Analyze one or more repositories",
            MenuItem::Compare => "Compare multiple repositories",
            MenuItem::Help => "Keybindings and usage guide",
            MenuItem::Metadata => "Database info and table row counts",
        }
    }

    pub fn screen(&self) -> Screen {
        match self {
            MenuItem::Analyze => Screen::AnalyzeMenu,
            MenuItem::Compare => Screen::Compare,
            MenuItem::Help => Screen::Help,
            MenuItem::Metadata => Screen::Metadata,
        }
    }
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::Analyze => write!(f, "Analyze"),
            MenuItem::Compare => write!(f, "Compare"),
            MenuItem::Help => write!(f, "Help"),
            MenuItem::Metadata => write!(f, "Metadata"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    StartAnalyze {
        paths: Vec<PathBuf>,
        view: AnalyzeView,
    },
    StartCompare(Vec<PathBuf>),
    LoadMetadata,
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
    pub input_paths: Vec<PathBuf>,
    pub status_message: String,
    pub analyze_menu_selected: usize,
    pub selected_analyze_view: AnalyzeView,
    pub analysis_table: Option<String>,
    pub metadata_text: Option<String>,
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
                MenuItem::Compare,
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
            metadata_text: None,
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
            if screen == Screen::Metadata {
                self.pending_action = Some(AppAction::LoadMetadata);
            }
            self.active_screen = screen;
        }
    }

    pub fn go_home(&mut self) {
        self.active_screen = Screen::Home;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn dispatch_analyze(&mut self) {
        if !self.input_paths.is_empty() {
            self.analysis_table = None;
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
            self.input_paths.clear();
            self.input_buffer.clear();
            self.input_paths.push(PathBuf::from("."));
            self.active_screen = Screen::Analyze;
            self.dispatch_analyze();
        }
    }

    pub fn analyze_menu_up(&mut self) {
        if self.analyze_menu_selected > 0 {
            self.analyze_menu_selected -= 1;
        }
    }

    pub fn analyze_menu_down(&mut self) {
        if self.analyze_menu_selected + 1 < ANALYZE_MENU_ITEMS.len() {
            self.analyze_menu_selected += 1;
        }
    }

    pub fn dispatch_compare(&mut self) {
        if self.input_paths.len() >= 2 {
            self.pending_action = Some(AppAction::StartCompare(self.input_paths.clone()));
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
            self.input_paths.push(PathBuf::from(path));
            self.input_buffer.clear();
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
                MenuItem::Compare,
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
    fn navigate_to_compare() {
        let mut app = AppState::new();
        app.move_down();
        assert_eq!(app.selected, 1);
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Compare);
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
        app.input_paths.push(PathBuf::from("/tmp/repo"));
        app.dispatch_analyze();
        assert_eq!(
            app.pending_action,
            Some(AppAction::StartAnalyze {
                paths: vec![PathBuf::from("/tmp/repo")],
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
    fn dispatch_compare_requires_two_paths() {
        let mut app = AppState::new();
        app.input_paths.push(PathBuf::from("/tmp/a"));
        app.dispatch_compare();
        assert!(app.pending_action.is_none());
        app.input_paths.push(PathBuf::from("/tmp/b"));
        app.dispatch_compare();
        assert!(app.pending_action.is_some());
    }

    #[test]
    fn take_action_clears_pending() {
        let mut app = AppState::new();
        app.input_paths.push(PathBuf::from("/tmp/repo"));
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
        assert_eq!(app.input_paths, vec![PathBuf::from("/tmp/repo")]);
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
        assert_eq!(MenuItem::Compare.screen(), Screen::Compare);
        assert_eq!(MenuItem::Metadata.screen(), Screen::Metadata);
    }

    #[test]
    fn analyze_menu_opens_before_path_entry() {
        let mut app = AppState::new();
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::AnalyzeMenu);
    }

    #[test]
    fn analyze_users_contribution_dispatches_immediately() {
        let mut app = AppState::new();
        app.active_screen = Screen::AnalyzeMenu;
        app.analyze_menu_selected = 1;
        app.select_analyze_view();
        assert_eq!(app.active_screen, Screen::Analyze);
        assert_eq!(app.selected_analyze_view, AnalyzeView::UsersContribution);
        assert_eq!(
            app.pending_action,
            Some(AppAction::StartAnalyze {
                paths: vec![PathBuf::from(".")],
                view: AnalyzeView::UsersContribution,
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
        });
        app.analysis_table = Some("stale".to_string());
        app.input_buffer = "old".to_string();
        app.input_paths.push(PathBuf::from("/tmp/old"));
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
            MenuItem::Compare,
            MenuItem::Help,
            MenuItem::Metadata,
        ] {
            assert!(!item.description().is_empty());
        }
    }
}
