use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{AppState, Screen};

pub fn draw(frame: &mut Frame, app: &AppState) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(40)])
        .split(main_area);

    draw_menu(frame, app, columns[0]);
    draw_content(frame, app, columns[1]);
    draw_status_bar(frame, app, status_area);
}

fn draw_menu(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let active = item.screen() == app.active_screen;
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let prefix = if active { "▸ " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{item}"), style)))
        })
        .collect();

    let menu = List::new(items).block(Block::default().borders(Borders::ALL).title("Repolyze"));
    frame.render_widget(menu, area);
}

fn draw_content(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    match app.active_screen {
        Screen::Help => draw_help(frame, area),
        Screen::Analyze => draw_analyze(frame, app, area),
        Screen::Compare => draw_compare(frame, app, area),
        Screen::Errors => draw_errors(frame, app, area),
    }
}

fn draw_help(frame: &mut Frame, area: ratatui::layout::Rect) {
    let text = "\
Welcome to Repolyze — repository analytics for local Git repositories.

Navigation:
  j/↓       Move down in menu
  k/↑       Move up in menu
  Enter     Activate selected item
  ?         Return to Help
  q         Quit

Screens:
  Help      This screen
  Analyze   Analyze one or more repositories
  Compare   Compare multiple repositories
  Errors    View analysis errors

In Analyze/Compare screens:
  Type a path and press Enter to add it
  Press Enter with empty input to run analysis
  Esc       Return to menu";

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_analyze(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let mut lines = vec![
        Line::from("Enter repository path(s), then press Enter with empty input to analyze."),
        Line::from(""),
    ];

    // Show added paths
    for (i, path) in app.input_paths.iter().enumerate() {
        lines.push(Line::from(format!("  {}. {}", i + 1, path.display())));
    }

    if !app.input_paths.is_empty() {
        lines.push(Line::from(""));
    }

    // Input prompt
    lines.push(Line::from(format!("Path: {}_", app.input_buffer)));

    // Show results if available
    if let Some(report) = &app.analysis_result {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Results ──",
            Style::default().fg(Color::Green),
        )));
        for analysis in &report.repositories {
            let name = analysis
                .repository
                .root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| analysis.repository.root.to_string_lossy().to_string());
            lines.push(Line::from(format!(
                "  {} — {} files, {} commits, {} contributors",
                name,
                analysis.size.files,
                analysis.contributions.total_commits,
                analysis.contributions.contributors.len(),
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Analyze"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_compare(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let mut lines = vec![
        Line::from("Enter 2+ repository paths, then press Enter with empty input to compare."),
        Line::from(""),
    ];

    for (i, path) in app.input_paths.iter().enumerate() {
        lines.push(Line::from(format!("  {}. {}", i + 1, path.display())));
    }

    if !app.input_paths.is_empty() {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(format!("Path: {}_", app.input_buffer)));

    if let Some(report) = &app.analysis_result {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Comparison Results ──",
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(format!(
            "  Repositories: {}  |  Total commits: {}  |  Contributors: {}  |  Files: {}",
            report.repositories.len(),
            report.summary.total_commits,
            report.summary.total_contributors,
            report.summary.total_files,
        )));

        for analysis in &report.repositories {
            let name = analysis
                .repository
                .root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| analysis.repository.root.to_string_lossy().to_string());
            lines.push(Line::from(format!(
                "    {} — {} files, {} lines, {} commits",
                name,
                analysis.size.files,
                analysis.size.total_lines,
                analysis.contributions.total_commits,
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Compare"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_errors(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let mut lines = Vec::new();

    if app.errors.is_empty() {
        lines.push(Line::from("No errors recorded."));
    } else {
        lines.push(Line::from(format!("{} error(s):", app.errors.len())));
        lines.push(Line::from(""));
        for error in &app.errors {
            lines.push(Line::from(Span::styled(
                format!("  {} — {}", error.path.display(), error.reason),
                Style::default().fg(Color::Red),
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Errors"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_status_bar(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let screen_name = format!("{:?}", app.active_screen);
    let error_count = if app.errors.is_empty() {
        String::new()
    } else {
        format!(" | {} error(s)", app.errors.len())
    };

    let status = format!(
        " [{}] {} | q: quit | ?: help{}",
        screen_name, app.status_message, error_count
    );

    let bar = Paragraph::new(status).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray)),
    );
    frame.render_widget(bar, area);
}
