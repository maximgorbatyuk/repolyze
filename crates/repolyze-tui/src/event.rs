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
        Screen::Compare => handle_input_screen(app, key.code),
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
        _ => {}
    }
}

fn handle_input_screen(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.go_home();
        }
        KeyCode::Enter => {
            if app.input_buffer.is_empty() && !app.input_paths.is_empty() {
                app.dispatch_compare();
            } else {
                app.add_input_path();
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}
