use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{AppState, Screen};

/// Map non-Latin characters to their QWERTY physical-key equivalents.
/// This lets shortcuts (q, j, k, y, n, …) work regardless of keyboard layout.
fn normalize_to_qwerty(c: char) -> char {
    match c {
        // Russian (ЙЦУКЕН) → QWERTY
        'й' => 'q',
        'ц' => 'w',
        'у' => 'e',
        'к' => 'r',
        'е' => 't',
        'н' => 'y',
        'г' => 'u',
        'ш' => 'i',
        'щ' => 'o',
        'з' => 'p',
        'ф' => 'a',
        'ы' => 's',
        'в' => 'd',
        'а' => 'f',
        'п' => 'g',
        'р' => 'h',
        'о' => 'j',
        'л' => 'k',
        'д' => 'l',
        'я' => 'z',
        'ч' => 'x',
        'с' => 'c',
        'м' => 'v',
        'и' => 'b',
        'т' => 'n',
        'ь' => 'm',
        _ => c,
    }
}

pub fn handle_key(app: &mut AppState, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    // Normalize non-Latin chars to QWERTY for shortcut-based screens.
    // Text-input screens (UserSelect, GitToolsInput) pass raw characters through.
    let code = match (&app.active_screen, key.code) {
        (Screen::UserSelect | Screen::GitToolsInput, _) => key.code,
        (_, KeyCode::Char(c)) => KeyCode::Char(normalize_to_qwerty(c)),
        _ => key.code,
    };

    match app.active_screen {
        Screen::Home => handle_home(app, code),
        Screen::Help | Screen::Metadata => handle_static_screen(app, code),
        Screen::AnalyzeMenu => handle_analyze_menu(app, code),
        Screen::Analyze => handle_results_screen(app, code),
        Screen::UserSelect => handle_user_select(app, code),
        Screen::GitToolsMenu => handle_git_tools_menu(app, code),
        Screen::GitToolsRepoSelect => handle_git_tools_repo_select(app, code),
        Screen::GitToolsInput => handle_git_tools_input(app, code),
        Screen::GitToolsBranchList => handle_git_tools_branch_list(app, code),
        Screen::GitToolsProgress => handle_git_tools_progress(app, code),
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
        KeyCode::Char('e') => app.request_export(),
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

fn handle_git_tools_menu(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Up | KeyCode::Char('k') => app.git_tools.menu_up(),
        KeyCode::Down | KeyCode::Char('j') => app.git_tools.menu_down(),
        KeyCode::Enter => app.git_tools_select(),
        KeyCode::Esc => app.go_home(),
        _ => {}
    }
}

// Text input screen — all characters go to the input buffer.
fn handle_git_tools_input(app: &mut AppState, code: KeyCode) {
    if app.is_loading {
        return; // ignore input while loading
    }
    match code {
        KeyCode::Esc => {
            app.git_tools.input.clear();
            app.active_screen = Screen::GitToolsMenu;
        }
        KeyCode::Enter => app.git_tools_submit_input(),
        KeyCode::Backspace => {
            app.git_tools.input.pop();
        }
        KeyCode::Char(c) => {
            app.git_tools.input.push(c);
        }
        _ => {}
    }
}

fn handle_git_tools_repo_select(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Up | KeyCode::Char('k') => {
            app.git_tools.repo_select_up();
            let h = app.visible_height;
            app.git_tools.ensure_repo_visible(h);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.git_tools.repo_select_down();
            let h = app.visible_height;
            app.git_tools.ensure_repo_visible(h);
        }
        KeyCode::Enter => app.git_tools_select_repo(),
        KeyCode::Esc => {
            app.git_tools.clear_tool();
            app.active_screen = Screen::GitToolsMenu;
        }
        _ => {}
    }
}

fn handle_git_tools_branch_list(app: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Enter => app.git_tools_confirm_delete(),
        KeyCode::Char('n') | KeyCode::Esc => {
            app.git_tools.clear_tool();
            app.active_screen = Screen::GitToolsMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => app.git_tools.scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => app.git_tools_scroll_down(),
        KeyCode::Char('q') => app.quit(),
        _ => {}
    }
}

fn handle_git_tools_progress(app: &mut AppState, code: KeyCode) {
    if app.git_tools.done {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                app.git_tools.clear_tool();
                app.active_screen = Screen::GitToolsMenu;
            }
            KeyCode::Char('q') => app.quit(),
            KeyCode::Up | KeyCode::Char('k') => app.git_tools.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => app.git_tools_scroll_down(),
            _ => {}
        }
    } else {
        // Allow Esc to cancel in-progress deletion
        match code {
            KeyCode::Esc => {
                app.git_tools.done = true;
            }
            KeyCode::Up | KeyCode::Char('k') => app.git_tools.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => app.git_tools_scroll_down(),
            _ => {}
        }
    }
}
