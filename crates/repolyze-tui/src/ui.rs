use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use repolyze_core::model::HeatmapData;
use repolyze_report::table::{HEATMAP_DESC, HEATMAP_TITLE};

use crate::app::{ANALYZE_MENU_ITEMS, AnalyzeView, AppState, Screen};

const LOGO: &str = r#"
  ____                  _
 |  _ \ ___ _ __   ___ | |_   _ _______
 | |_) / _ \ '_ \ / _ \| | | | |_  / _ \
 |  _ <  __/ |_) | (_) | | |_| |/ /  __/
 |_| \_\___| .__/ \___/|_|\__, /___\___|
            |_|            |___/
"#;

const SPINNER_FRAMES: &[&str] = &[
    "\u{2801}", // ⠁
    "\u{2809}", // ⠉
    "\u{2819}", // ⠙
    "\u{2838}", // ⠸
    "\u{2830}", // ⠰
    "\u{2834}", // ⠴
    "\u{2826}", // ⠦
    "\u{2807}", // ⠇
];

const GITHUB_URL: &str = "https://github.com/maximgorbatyuk/repolyze";
const SLOGAN: &str = "Know your code better.";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = "maximgorbatyuk";

pub fn draw(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    match app.active_screen {
        Screen::Home => draw_home(frame, app, area),
        Screen::Help => draw_help(frame, area),
        Screen::AnalyzeMenu => draw_analyze_menu(frame, app, area),
        Screen::Analyze => draw_analyze(frame, app, area),
        Screen::Metadata => draw_metadata(frame, app, area),
        Screen::UserSelect => draw_user_select(frame, &mut *app, area),
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

    // Workspace info
    if let Some(info) = &app.workspace_info {
        lines.push(Line::from(vec![
            Span::styled("   Folder:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(&info.folder),
        ]));
        let mode = if info.is_single_repo {
            "Single repository".to_string()
        } else if info.repo_count > 0 {
            format!("Multi-repository ({} repos)", info.repo_count)
        } else {
            "No repositories found".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("   Mode:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(mode),
        ]));
        lines.push(Line::from(""));
    }

    let menu_len = app.effective_menu_len();
    for (i, (label, _)) in ANALYZE_MENU_ITEMS.iter().enumerate().take(menu_len) {
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

fn draw_analyze(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let view_label = match &app.selected_analyze_view {
        AnalyzeView::All => "All",
        AnalyzeView::Contribution => "Contribution",
        AnalyzeView::Activity => "Most active days and hours",
        AnalyzeView::ActivityHeatmap => "Activity heatmap",
        AnalyzeView::UserEffort => "User effort",
        AnalyzeView::CompareRepos => "Compare repositories",
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Analyze \u{2014} {view_label}"),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.is_loading {
        let frame_idx = app.spinner_frame % SPINNER_FRAMES.len();
        let spinner = SPINNER_FRAMES[frame_idx];
        lines.push(Line::from(vec![
            Span::styled(format!(" {spinner}"), Style::default().fg(Color::Cyan)),
            Span::styled(
                " Analyzing...".to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    } else if let Some(table) = &app.analysis_table {
        // Analytics view with ASCII table
        for table_line in table.lines() {
            lines.push(Line::from(format!(" {table_line}")));
        }

        // Append heatmap if present
        if let Some(data) = &app.heatmap_data {
            lines.push(Line::from(""));
            if app.selected_analyze_view == AnalyzeView::All {
                lines.push(Line::from(format!(" #3 {HEATMAP_TITLE}")));
            }
            lines.push(Line::from(format!(" {HEATMAP_DESC}")));
            lines.push(Line::from(""));
            lines.extend(heatmap_lines(data));
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
    lines.push(hints_line(&[
        ("\u{2191}\u{2193}", "Scroll"),
        ("Esc", "Home"),
        ("Q", "Quit"),
    ]));

    app.content_height = lines.len() as u16;
    app.visible_height = area.height;

    let paragraph = Paragraph::new(lines).scroll((app.scroll_offset, 0));
    frame.render_widget(paragraph, area);
}

fn heatmap_color(count: u32, max: u32) -> Color {
    if count == 0 || max == 0 {
        Color::DarkGray
    } else {
        let ratio = count as f64 / max as f64;
        if ratio <= 0.25 {
            Color::Rgb(0, 100, 100)
        } else if ratio <= 0.50 {
            Color::Cyan
        } else if ratio <= 0.75 {
            Color::LightCyan
        } else {
            Color::Yellow
        }
    }
}

fn heatmap_lines(data: &HeatmapData) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let cell = "\u{25a0} "; // ■ + space

    // Period
    lines.push(Line::from(Span::styled(
        format!("      {} .. {}", data.start_date, data.end_date),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Month label row
    let label_width = 5; // "Mon  " etc.
    let mut month_spans: Vec<Span<'static>> = Vec::new();
    month_spans.push(Span::raw(" ".repeat(label_width)));
    let mut last_col = 0;
    for (col, label) in &data.month_labels {
        let char_pos = col * 2; // each cell is 2 chars wide
        if char_pos > last_col {
            month_spans.push(Span::raw(" ".repeat(char_pos - last_col)));
        }
        month_spans.push(Span::styled(
            label.clone(),
            Style::default().fg(Color::DarkGray),
        ));
        last_col = char_pos + label.len();
    }
    lines.push(Line::from(month_spans));

    // Weekday rows
    let weekday_labels = ["Mon", "   ", "Wed", "   ", "Fri", "   ", "Sun"];
    for (weekday, label) in weekday_labels.iter().enumerate() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(
            format!(" {label:<4}"),
            Style::default().fg(Color::DarkGray),
        ));
        for week_col in 0..data.week_count {
            let count = data.grid[weekday][week_col];
            let color = heatmap_color(count, data.max_count);
            spans.push(Span::styled(cell.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }

    // Legend with commit-count ranges
    lines.push(Line::from(""));
    let labels = data.legend_labels();
    let colors = [
        Color::DarkGray,
        Color::Rgb(0, 100, 100),
        Color::Cyan,
        Color::LightCyan,
        Color::Yellow,
    ];
    let mut legend_spans: Vec<Span<'static>> = Vec::new();
    legend_spans.push(Span::raw("      "));
    for (i, (label, color)) in labels.iter().zip(colors.iter()).enumerate() {
        if i > 0 {
            legend_spans.push(Span::raw("  "));
        }
        legend_spans.push(Span::styled(
            "\u{25a0}".to_string(),
            Style::default().fg(*color),
        ));
        legend_spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(legend_spans));

    lines
}

fn draw_user_select(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            " User Effort \u{2014} Select Contributor",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Filter input
    lines.push(Line::from(vec![
        Span::styled(" Filter: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&app.contributor_filter),
        Span::styled("_", Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(""));

    let filtered = app.filtered_contributors();
    if filtered.is_empty() {
        lines.push(Line::from(Span::styled(
            " No contributors match the filter.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, (email, name)) in filtered.iter().enumerate() {
            let is_selected = i == app.contributor_selected;
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
                format!("{prefix}{}. {} ({})", i + 1, email, name),
                style,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(hints_line(&[
        ("Type", "Filter"),
        ("\u{2191}\u{2193}", "Navigate"),
        ("Enter", "Select"),
        ("Esc", "Home"),
    ]));

    app.content_height = lines.len() as u16;
    app.visible_height = area.height;

    let paragraph = Paragraph::new(lines).scroll((app.scroll_offset, 0));
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
