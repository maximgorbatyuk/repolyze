use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::app::{ANALYZE_MENU_ITEMS, AnalyzeView, AppState, Screen};

const LOGO: &str = r#"
  ____                  _
 |  _ \ ___ _ __   ___ | |_   _ _______
 | |_) / _ \ '_ \ / _ \| | | | |_  / _ \
 |  _ <  __/ |_) | (_) | | |_| |/ /  __/
 |_| \_\___| .__/ \___/|_|\__, /___\___|
            |_|            |___/
"#;

const GITHUB_URL: &str = "https://github.com/maximgorbatyuk/repolyze";
const SLOGAN: &str = "Know your code better.";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = "maximgorbatyuk";

pub fn draw(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    match app.active_screen {
        Screen::Home => draw_home(frame, app, area),
        Screen::Help => draw_help(frame, area),
        Screen::AnalyzeMenu => draw_analyze_menu(frame, app, area),
        Screen::Analyze => draw_analyze(frame, app, area),
        Screen::Metadata => draw_metadata(frame, app, area),
    }
}

fn key_hint<'a>(key: &'a str, label: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled(key, Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {label}"), Style::default().fg(Color::DarkGray)),
    ]
}

fn hints_line<'a>(hints: &'a [(&'a str, &'a str)]) -> Line<'a> {
    let dim = Style::default().fg(Color::DarkGray);
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::raw(" "));
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  \u{2502}  ", dim)); // │
        }
        spans.extend(key_hint(key, label));
    }
    Line::from(spans)
}

fn draw_home(frame: &mut Frame, app: &AppState, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // ASCII logo
    for logo_line in LOGO.lines() {
        lines.push(Line::from(Span::styled(
            logo_line.to_string(),
            Style::default().fg(Color::Cyan),
        )));
    }

    // Slogan
    lines.push(Line::from(vec![Span::styled(
        format!("  {SLOGAN}"),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC),
    )]));
    lines.push(Line::from(""));

    // GitHub link
    lines.push(Line::from(Span::styled(
        format!("  {GITHUB_URL}"),
        Style::default().fg(Color::DarkGray),
    )));

    // Version and author
    lines.push(Line::from(Span::styled(
        format!("  v{VERSION}  \u{00a9} {AUTHOR}"),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Menu items
    for (i, item) in app.menu_items.iter().enumerate() {
        let number = i + 1;
        let is_selected = i == app.selected;

        let (prefix, style) = if is_selected {
            (
                "\u{27a4} ", // ➤
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default())
        };

        let item_name = format!("{item}");
        let padding = " ".repeat(12usize.saturating_sub(item_name.len()));
        let desc = item.description();

        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}{number}. "), style),
            Span::styled(item_name, style),
            Span::raw(padding),
            Span::styled(desc, Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Key hints
    lines.push(Line::from(""));
    lines.push(hints_line(&[
        ("\u{2191}\u{2193}", "Navigate"),
        ("Enter", "Select"),
        ("?", "Help"),
        ("Q", "Quit"),
    ]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            " Help",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" Navigation:"),
        Line::from("   j/\u{2193}       Move down in menu"),
        Line::from("   k/\u{2191}       Move up in menu"),
        Line::from("   Enter     Activate selected item"),
        Line::from("   ?         Return to Help"),
        Line::from("   Esc       Return to Home"),
        Line::from("   q         Quit"),
        Line::from(""),
        Line::from(" Screens:"),
        Line::from("   Analyze   Analyze one or more repositories"),
        Line::from("   Help      This screen"),
        Line::from("   Metadata  Database info and table row counts"),
    ];

    lines.push(Line::from(""));
    lines.push(hints_line(&[("Esc", "Home"), ("Q", "Quit")]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_analyze_menu(frame: &mut Frame, app: &AppState, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            " Analyze \u{2014} Select View",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, (label, _)) in ANALYZE_MENU_ITEMS.iter().enumerate() {
        let is_selected = i == app.analyze_menu_selected;
        let (prefix, style) = if is_selected {
            (
                "\u{27a4} ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default())
        };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{}. {label}", i + 1),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(hints_line(&[
        ("\u{2191}\u{2193}", "Navigate"),
        ("Enter", "Select"),
        ("Esc", "Home"),
        ("Q", "Quit"),
    ]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_analyze(frame: &mut Frame, app: &AppState, area: Rect) {
    let view_label = match &app.selected_analyze_view {
        AnalyzeView::All => "All",
        AnalyzeView::UsersContribution => "Users contribution",
        AnalyzeView::Activity => "Most active days and hours",
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Analyze \u{2014} {view_label}"),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.pending_action.is_some() {
        // Analysis is about to run
        lines.push(Line::from(Span::styled(
            " Analyzing...",
            Style::default().fg(Color::Yellow),
        )));
    } else if let Some(table) = &app.analysis_table {
        // Analytics view with ASCII table
        for table_line in table.lines() {
            lines.push(Line::from(format!(" {table_line}")));
        }
    } else if let Some(report) = &app.analysis_result {
        // All view with summary
        for analysis in &report.repositories {
            let name = analysis
                .repository
                .root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| analysis.repository.root.to_string_lossy().to_string());
            lines.push(Line::from(format!(
                "   {} \u{2014} {} files, {} commits, {} contributors",
                name,
                analysis.size.files,
                analysis.contributions.total_commits,
                analysis.contributions.contributors.len(),
            )));
        }
    } else {
        lines.push(Line::from(" No results yet."));
    }

    if !app.status_message.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {}", app.status_message),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(hints_line(&[("Esc", "Home"), ("Q", "Quit")]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_metadata(frame: &mut Frame, app: &AppState, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            " Metadata",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    match &app.metadata_text {
        Some(text) => {
            for line in text.lines() {
                lines.push(Line::from(format!(" {line}")));
            }
        }
        None => {
            lines.push(Line::from(" Loading..."));
        }
    }

    lines.push(Line::from(""));
    lines.push(hints_line(&[("Esc", "Home"), ("Q", "Quit")]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
