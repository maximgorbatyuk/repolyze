use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    Help,
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::Help => write!(f, "Help"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub menu_items: Vec<MenuItem>,
    pub selected: usize,
    pub should_quit: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            menu_items: vec![MenuItem::Help],
            selected: 0,
            should_quit: false,
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

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_help_as_the_only_menu_item() {
        let app = AppState::new();
        assert_eq!(app.menu_items, vec![MenuItem::Help]);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn move_down_does_not_overflow() {
        let mut app = AppState::new();
        app.move_down();
        assert_eq!(app.selected, 0);
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
}
