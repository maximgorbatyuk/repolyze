use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{AppState, Screen};

pub fn handle_key(app: &mut AppState, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    match app.active_screen {
        Screen::Home => handle_home(app, key.code),
        Screen::Help | Screen::Metadata => handle_static_screen(app, key.code),
        Screen::AnalyzeMenu => handle_analyze_menu(app, key.code),
        Screen::Analyze => handle_results_screen(app, key.code),
        Screen::UserSelect => handle_user_select(app, key.code),
    }
}

fn handle_home(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => app.activate_selected(),
        KeyCode::Char('?') => {
            app.active_screen = Screen::Help;
        }
        _ => {}
    }
}

fn handle_static_screen(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.go_home(),
        KeyCode::Char('?') => {
            app.active_screen = Screen::Help;
        }
        _ => {}
    }
}

fn handle_analyze_menu(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Up | KeyCode::Char('k') => app.analyze_menu_up(),
        KeyCode::Down | KeyCode::Char('j') => app.analyze_menu_down(),
        KeyCode::Enter => app.select_analyze_view(),
        KeyCode::Esc => app.go_home(),
        _ => {}
    }
}

fn handle_results_screen(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.go_home(),
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
        _ => {}
    }
}

// No KeyCode::Char('q') quit — all characters go to the filter input.
// Quit via Ctrl+C; go home via Esc.
fn handle_user_select(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Esc => app.go_home(),
        KeyCode::Up => app.contributor_select_up(),
        KeyCode::Down => app.contributor_select_down(),
        KeyCode::Enter => app.select_contributor(),
        KeyCode::Backspace => {
            app.contributor_filter.pop();
            app.contributor_selected = 0;
            app.scroll_offset = 0;
        }
        KeyCode::Char(c) => {
            app.contributor_filter.push(c);
            app.contributor_selected = 0;
            app.scroll_offset = 0;
        }
        _ => {}
    }
}
