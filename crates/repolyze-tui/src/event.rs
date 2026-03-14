use crossterm::event::KeyCode;

use crate::app::{AppState, Screen};

/// Handle a key press event, updating app state accordingly.
pub fn handle_key(app: &mut AppState, code: KeyCode) {
    match app.active_screen {
        Screen::Home => handle_home(app, code),
        Screen::Help | Screen::Errors => handle_global(app, code),
        Screen::Analyze => handle_input_screen(app, code, false),
        Screen::Compare => handle_input_screen(app, code, true),
    }
}

/// Home screen: navigate menu and activate items.
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

/// Global key bindings for non-input screens (Help, Errors).
fn handle_global(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => app.activate_selected(),
        KeyCode::Esc => app.go_home(),
        KeyCode::Char('?') => {
            app.active_screen = Screen::Help;
        }
        _ => {}
    }
}

/// Key handling for Analyze/Compare screens with path input.
fn handle_input_screen(app: &mut AppState, code: KeyCode, is_compare: bool) {
    match code {
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.go_home();
        }
        KeyCode::Enter => {
            if app.input_buffer.is_empty() && !app.input_paths.is_empty() {
                if is_compare {
                    app.dispatch_compare();
                } else {
                    app.dispatch_analyze();
                }
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
