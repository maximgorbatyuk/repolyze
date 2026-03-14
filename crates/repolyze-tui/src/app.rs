use std::fmt;
use std::path::PathBuf;

use repolyze_core::model::{ComparisonReport, PartialFailure};

/// Active screen in the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Help,
    Analyze,
    Compare,
    Errors,
}

/// Menu items shown in the left sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    Help,
    Analyze,
    Compare,
    Errors,
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::Help => write!(f, "Help"),
            MenuItem::Analyze => write!(f, "Analyze"),
            MenuItem::Compare => write!(f, "Compare"),
            MenuItem::Errors => write!(f, "Errors"),
        }
    }
}

impl MenuItem {
    pub fn screen(&self) -> Screen {
        match self {
            MenuItem::Help => Screen::Help,
            MenuItem::Analyze => Screen::Analyze,
            MenuItem::Compare => Screen::Compare,
            MenuItem::Errors => Screen::Errors,
        }
    }
}

/// Actions that originate from user interaction but execute outside the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    StartAnalyze(Vec<PathBuf>),
    StartCompare(Vec<PathBuf>),
    ShowErrors,
}

/// Full TUI application state.
#[derive(Debug, Clone)]
pub struct AppState {
    pub menu_items: Vec<MenuItem>,
    pub selected: usize,
    pub active_screen: Screen,
    pub should_quit: bool,
    /// Result from the most recent analysis run, if any.
    pub analysis_result: Option<ComparisonReport>,
    /// Accumulated errors from partial failures.
    pub errors: Vec<PartialFailure>,
    /// Pending action dispatched by the user.
    pub pending_action: Option<AppAction>,
    /// Path input buffer for analyze/compare screens.
    pub input_buffer: String,
    /// Paths already added for the current operation.
    pub input_paths: Vec<PathBuf>,
    /// Status message shown in the bottom bar.
    pub status_message: String,
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
                MenuItem::Help,
                MenuItem::Analyze,
                MenuItem::Compare,
                MenuItem::Errors,
            ],
            selected: 0,
            active_screen: Screen::Help,
            should_quit: false,
            analysis_result: None,
            errors: Vec::new(),
            pending_action: None,
            input_buffer: String::new(),
            input_paths: Vec::new(),
            status_message: "Ready".to_string(),
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

    /// Activate the currently selected menu item, switching screens.
    pub fn activate_selected(&mut self) {
        if let Some(item) = self.menu_items.get(self.selected) {
            self.active_screen = item.screen();
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Dispatch an analyze action with the current input paths.
    pub fn dispatch_analyze(&mut self) {
        if !self.input_paths.is_empty() {
            self.pending_action = Some(AppAction::StartAnalyze(self.input_paths.clone()));
        }
    }

    /// Dispatch a compare action with the current input paths.
    pub fn dispatch_compare(&mut self) {
        if self.input_paths.len() >= 2 {
            self.pending_action = Some(AppAction::StartCompare(self.input_paths.clone()));
        }
    }

    /// Take the pending action, clearing it.
    pub fn take_action(&mut self) -> Option<AppAction> {
        self.pending_action.take()
    }

    /// Set analysis result and clear input state.
    pub fn set_result(&mut self, report: ComparisonReport) {
        self.errors.extend(report.failures.iter().cloned());
        self.analysis_result = Some(report);
        self.input_paths.clear();
        self.input_buffer.clear();
        self.status_message = "Analysis complete".to_string();
    }

    /// Add a path from the input buffer.
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
    fn starts_with_all_menu_items() {
        let app = AppState::new();
        assert_eq!(
            app.menu_items,
            vec![
                MenuItem::Help,
                MenuItem::Analyze,
                MenuItem::Compare,
                MenuItem::Errors,
            ]
        );
        assert_eq!(app.selected, 0);
        assert_eq!(app.active_screen, Screen::Help);
    }

    #[test]
    fn navigate_down_and_activate_analyze() {
        let mut app = AppState::new();
        app.move_down(); // selected = 1 (Analyze)
        assert_eq!(app.selected, 1);

        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Analyze);
    }

    #[test]
    fn navigate_to_compare() {
        let mut app = AppState::new();
        app.move_down(); // Analyze
        app.move_down(); // Compare
        assert_eq!(app.selected, 2);

        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Compare);
    }

    #[test]
    fn navigate_to_errors() {
        let mut app = AppState::new();
        app.move_down(); // Analyze
        app.move_down(); // Compare
        app.move_down(); // Errors
        assert_eq!(app.selected, 3);

        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Errors);
    }

    #[test]
    fn move_down_does_not_overflow() {
        let mut app = AppState::new();
        for _ in 0..10 {
            app.move_down();
        }
        assert_eq!(app.selected, 3); // last item index
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
    fn dispatch_analyze_creates_action() {
        let mut app = AppState::new();
        app.input_paths.push(PathBuf::from("/tmp/repo"));
        app.dispatch_analyze();

        assert_eq!(
            app.pending_action,
            Some(AppAction::StartAnalyze(vec![PathBuf::from("/tmp/repo")]))
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
    fn enter_on_help_stays_on_help() {
        let mut app = AppState::new();
        // selected = 0 (Help)
        app.activate_selected();
        assert_eq!(app.active_screen, Screen::Help);
    }

    #[test]
    fn menu_item_screen_mapping() {
        assert_eq!(MenuItem::Help.screen(), Screen::Help);
        assert_eq!(MenuItem::Analyze.screen(), Screen::Analyze);
        assert_eq!(MenuItem::Compare.screen(), Screen::Compare);
        assert_eq!(MenuItem::Errors.screen(), Screen::Errors);
    }
}
