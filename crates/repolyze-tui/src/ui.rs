use std::path::Path;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Axis, Bar, BarChart as RatatuiBarChart, BarGroup, Block, Borders, Chart, Dataset,
        GraphType, Paragraph, Wrap,
    },
};

use repolyze_core::chart_util::{fit_bar_dimensions, format_value_fit};
use repolyze_core::model::{BarChartData, HeatmapData, ProductivityTrendData, TimelineData};
use repolyze_report::table::{HEATMAP_DESC, HEATMAP_TITLE, PRODUCTIVITY_TREND_TITLE};

use crate::app::{
    ANALYZE_MENU_ITEMS, AnalyzeView, AppState, BranchProgress, GIT_TOOLS_MENU_ITEMS, GitToolsMode,
    Screen,
};

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

/// Short display name for a path (directory basename, or full path as fallback).
fn path_display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

pub fn draw(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    match app.active_screen {
        Screen::Home => draw_home(frame, app, area),
        Screen::Help => draw_help(frame, area),
        Screen::AnalyzeMenu => draw_analyze_menu(frame, app, area),
        Screen::Analyze => draw_analyze(frame, app, area),
        Screen::Metadata => draw_metadata(frame, app, area),
        Screen::UserSelect => draw_user_select(frame, &mut *app, area),
        Screen::GitToolsMenu => draw_git_tools_menu(frame, app, area),
        Screen::GitToolsRepoSelect => draw_git_tools_repo_select(frame, &mut *app, area),
        Screen::GitToolsInput => draw_git_tools_input(frame, app, area),
        Screen::GitToolsBranchList => draw_git_tools_branch_list(frame, app, area),
        Screen::GitToolsProgress => draw_git_tools_progress(frame, app, area),
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
        Line::from(" Analyze results:"),
        Line::from("   e         Export report as Markdown"),
        Line::from("   j/\u{2193}       Scroll down"),
        Line::from("   k/\u{2191}       Scroll up"),
        Line::from(""),
        Line::from(" Screens:"),
        Line::from("   Analyze    Analyze one or more repositories"),
        Line::from("   Git Tools  Git repository maintenance tools"),
        Line::from("   Help       This screen"),
        Line::from("   Metadata   Database info and table row counts"),
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
    if app.is_loading {
        draw_analyze_text(frame, app, area);
        return;
    }

    match &app.selected_analyze_view {
        AnalyzeView::WeekdayChart => {
            if let Some(data) = &app.weekday_chart {
                draw_bar_chart(frame, data, area);
            } else {
                draw_analyze_text(frame, app, area);
            }
        }
        AnalyzeView::HourlyChart => {
            if let Some(data) = &app.hourly_chart {
                draw_bar_chart(frame, data, area);
            } else {
                draw_analyze_text(frame, app, area);
            }
        }
        AnalyzeView::TimelineChart => {
            if let Some(data) = &app.timeline_data {
                draw_timeline_chart(frame, data, area);
            } else {
                draw_analyze_text(frame, app, area);
            }
        }
        _ => draw_analyze_text(frame, app, area),
    }
}

fn draw_analyze_text(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let view_label = match &app.selected_analyze_view {
        AnalyzeView::All => "All",
        AnalyzeView::Contribution => "Contribution",
        AnalyzeView::Activity => "Most active days and hours",
        AnalyzeView::ActivityHeatmap => "Activity heatmap",
        AnalyzeView::WeekdayChart => "Commits by weekday",
        AnalyzeView::HourlyChart => "Commits by hour",
        AnalyzeView::TimelineChart => "Commit timeline",
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
        // Show progress log from GitHub analysis
        for msg in &app.progress_log {
            lines.push(Line::from(Span::styled(
                format!("   {msg}"),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else if let Some(table) = &app.analysis_table {
        // Analytics view with ASCII table
        for table_line in table.lines() {
            lines.push(Line::from(format!(" {table_line}")));
        }

        // Append productivity-trend chart if present (styled cyan bars)
        if let Some(data) = &app.productivity_trend_data {
            lines.push(Line::from(""));
            if app.selected_analyze_view == AnalyzeView::All {
                lines.push(Line::from(format!(" #4 {PRODUCTIVITY_TREND_TITLE}")));
            } else {
                lines.push(Line::from(format!(" {PRODUCTIVITY_TREND_TITLE}")));
            }
            lines.push(Line::from(
                " Commits per week over the last 13 weeks; final bar may be a partial week.",
            ));
            if !data.reference_date.is_empty() {
                lines.push(Line::from(format!(
                    " Reference date: {}",
                    data.reference_date
                )));
            }
            lines.push(Line::from(""));
            lines.extend(text_productivity_trend_lines(data, area.width));
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

        // Append text-based charts in All view
        if app.selected_analyze_view == AnalyzeView::All {
            if let Some(data) = &app.weekday_chart {
                lines.push(Line::from(""));
                lines.push(Line::from(format!(" #5 {}", data.title)));
                lines.push(Line::from(""));
                lines.extend(text_bar_chart_lines(data, area.width));
            }
            if let Some(data) = &app.hourly_chart {
                lines.push(Line::from(""));
                lines.push(Line::from(format!(" #6 {}", data.title)));
                lines.push(Line::from(""));
                lines.extend(text_bar_chart_lines(data, area.width));
            }
            if let Some(data) = &app.timeline_data {
                lines.push(Line::from(""));
                lines.push(Line::from(format!(" #7 {}", data.title)));
                lines.push(Line::from(""));
                lines.extend(text_timeline_lines(data, area.width));
            }
        }
    } else if let Some(report) = &app.analysis_result {
        // All view with summary
        for analysis in &report.repositories {
            let name = analysis.repository.display_name();
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
    if app.analysis_result.is_some() && !app.is_loading {
        lines.push(hints_line(&[
            ("\u{2191}\u{2193}", "Scroll"),
            ("e", "Export"),
            ("Esc", "Home"),
            ("Q", "Quit"),
        ]));
    } else {
        lines.push(hints_line(&[
            ("\u{2191}\u{2193}", "Scroll"),
            ("Esc", "Home"),
            ("Q", "Quit"),
        ]));
    }

    app.content_height = lines.len() as u16;
    app.visible_height = area.height;

    let paragraph = Paragraph::new(lines).scroll((app.scroll_offset, 0));
    frame.render_widget(paragraph, area);
}

const SPARKLINE_BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

fn vertical_bar_lines_from_bars(bars: &[(String, u64)], max_width: u16) -> Vec<Line<'static>> {
    let max_val = bars.iter().map(|(_, v)| *v).max().unwrap_or(0);
    if max_val == 0 {
        let labels: String = bars
            .iter()
            .map(|(l, _)| l.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        return vec![
            Line::from(Span::raw(format!(" {labels}"))),
            Line::from(Span::styled(
                " (no data)",
                Style::default().fg(Color::DarkGray),
            )),
        ];
    }

    // Reserve 1 char for the leading padding space emitted on every row below.
    let chart_area = max_width.saturating_sub(1) as usize;
    let (col_width, gap, display_labels) = fit_bar_dimensions(bars, chart_area);

    let chart_height: usize = 8;
    let mut lines = Vec::new();
    for row in (0..chart_height).rev() {
        let threshold = (row as f64 + 0.5) / chart_height as f64;
        let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
        for (i, (_, value)) in bars.iter().enumerate() {
            if i > 0 && gap > 0 {
                spans.push(Span::raw(" ".repeat(gap)));
            }
            let ratio = *value as f64 / max_val as f64;
            if ratio >= threshold {
                let block = if ratio >= threshold + 1.0 / chart_height as f64 {
                    "\u{2588}"
                } else {
                    let frac = ((ratio - threshold) * chart_height as f64 * 8.0).round() as usize;
                    ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][frac.min(7)]
                };
                let bar_str = block.repeat(col_width);
                spans.push(Span::styled(bar_str, Style::default().fg(Color::Cyan)));
            } else {
                spans.push(Span::raw(" ".repeat(col_width)));
            }
        }
        lines.push(Line::from(spans));
    }

    let mut val_spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (i, (_, value)) in bars.iter().enumerate() {
        if i > 0 && gap > 0 {
            val_spans.push(Span::raw(" ".repeat(gap)));
        }
        let text = format_value_fit(*value, col_width);
        val_spans.push(Span::styled(
            format!("{:^col_width$}", text),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(val_spans));

    let mut label_spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (i, label) in display_labels.iter().enumerate() {
        if i > 0 && gap > 0 {
            label_spans.push(Span::raw(" ".repeat(gap)));
        }
        label_spans.push(Span::raw(format!("{:^col_width$}", label)));
    }
    lines.push(Line::from(label_spans));

    lines
}

fn text_bar_chart_lines(data: &BarChartData, max_width: u16) -> Vec<Line<'static>> {
    vertical_bar_lines_from_bars(&data.bars, max_width)
}

/// Render a productivity-trend chart with cyan-styled bars, matching the other TUI charts.
///
/// Takes the weekly series, converts it to `MM-DD`-labeled bars, and delegates to the
/// shared bar renderer. Returns an empty vec when the data has no weeks.
fn text_productivity_trend_lines(
    data: &ProductivityTrendData,
    max_width: u16,
) -> Vec<Line<'static>> {
    if data.weeks.is_empty() {
        return Vec::new();
    }
    let bars: Vec<(String, u64)> = data
        .weeks
        .iter()
        .map(|w| {
            let label = if w.week_start.len() >= 10 {
                w.week_start[5..10].to_string()
            } else {
                w.week_start.clone()
            };
            (label, w.commits as u64)
        })
        .collect();
    vertical_bar_lines_from_bars(&bars, max_width)
}

fn text_timeline_lines(data: &TimelineData, width: u16) -> Vec<Line<'static>> {
    if data.points.is_empty() {
        return vec![Line::from(" No commit data available.")];
    }

    let available = width.saturating_sub(2) as usize; // 1 padding each side
    let weeks = aggregate_weekly(&data.points);
    let max_val = weeks.iter().map(|(_, v)| *v).max().unwrap_or(0);

    // Resample to fill available width
    let n = weeks.len();
    let cols = available.max(1);
    let mut sampled: Vec<u32> = Vec::with_capacity(cols);
    for c in 0..cols {
        let src_idx = c * n / cols;
        sampled.push(weeks.get(src_idx).map(|(_, v)| *v).unwrap_or(0));
    }

    // Sparkline rows (3 rows tall — each row spans 8 sub-levels, total 24)
    let chart_height: usize = 3;
    let total_levels = chart_height * 8;
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(chart_height + 2);
    for row in (0..chart_height).rev() {
        let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
        for value in &sampled {
            let filled = if max_val == 0 {
                0
            } else {
                ((*value as f64 / max_val as f64) * total_levels as f64).round() as usize
            };
            let row_filled = filled.saturating_sub(row * 8).min(8);
            let ch: char = if row_filled == 0 {
                ' '
            } else {
                SPARKLINE_BLOCKS[row_filled - 1]
            };
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(Color::Cyan),
            ));
        }
        lines.push(Line::from(spans));
    }

    // Date labels below — first, middle, last
    if !weeks.is_empty() {
        let mut label_line = " ".to_string();
        let positions: Vec<usize> = if cols > 2 {
            vec![0, cols / 2, cols - 1]
        } else {
            vec![0]
        };
        let mut cursor = 0;
        for &pos in &positions {
            let src_idx = (pos * n / cols).min(n.saturating_sub(1));
            let label = &weeks[src_idx].0;
            if pos >= cursor {
                label_line.push_str(&" ".repeat(pos - cursor));
                label_line.push_str(label);
                cursor = pos + label.len();
            }
        }
        lines.push(Line::from(Span::styled(
            label_line,
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(Span::styled(
        format!(
            " {} = 0  {} = {max_val} commits/week",
            SPARKLINE_BLOCKS[0], SPARKLINE_BLOCKS[7]
        ),
        Style::default().fg(Color::DarkGray),
    )));

    lines
}

fn aggregate_weekly(points: &[(String, u32)]) -> Vec<(String, u32)> {
    let mut weeks: Vec<(String, u32)> = Vec::new();
    let mut day_count = 0u32;

    for (date, count) in points {
        if day_count >= 7 || weeks.is_empty() {
            weeks.push((date.clone(), *count));
            day_count = 1;
        } else {
            if let Some(last) = weeks.last_mut() {
                last.1 = last.1.saturating_add(*count);
            }
            day_count += 1;
        }
    }
    weeks
}

fn draw_bar_chart(frame: &mut Frame, data: &BarChartData, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let n = data.bars.len() as u16;
    // Available width inside borders (2 for left+right border)
    let inner_width = chunks[0].width.saturating_sub(2);
    // Calculate bar_width to fit all bars: n * bar_width + (n-1) * gap <= inner_width
    let (bar_width, bar_gap) = if n == 0 {
        (1, 1)
    } else {
        let gaps = n.saturating_sub(1);
        // Try gap=1 first, then gap=0 if bars still don't fit
        let w_with_gap = inner_width.saturating_sub(gaps) / n;
        if w_with_gap >= 2 {
            (w_with_gap, 1)
        } else {
            (inner_width / n, 0)
        }
    };

    let bars: Vec<Bar> = data
        .bars
        .iter()
        .map(|(label, val)| {
            Bar::default()
                .value(*val)
                .label(Line::from(label.clone()))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let chart = RatatuiBarChart::default()
        .block(
            Block::default()
                .title(format!(" {} ", data.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width.max(1))
        .bar_gap(bar_gap)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White));

    frame.render_widget(chart, chunks[0]);
    frame.render_widget(
        Paragraph::new(hints_line(&[("Esc", "Home"), ("Q", "Quit")])),
        chunks[1],
    );
}

fn draw_timeline_chart(frame: &mut Frame, data: &TimelineData, area: Rect) {
    if data.points.is_empty() {
        let empty = Paragraph::new(Line::from(" No commit data available.")).block(
            Block::default()
                .title(format!(" {} ", data.title))
                .borders(Borders::ALL),
        );
        frame.render_widget(empty, area);
        return;
    }

    let chart_area = area;

    let data_points: Vec<(f64, f64)> = data
        .points
        .iter()
        .enumerate()
        .map(|(i, (_, count))| (i as f64, *count as f64))
        .collect();

    let y_max = data
        .points
        .iter()
        .map(|(_, c)| *c as f64)
        .fold(0.0f64, f64::max);
    let x_max = if data_points.len() <= 1 {
        1.0
    } else {
        (data_points.len() - 1) as f64
    };

    // Select ~6 evenly-spaced date labels for the X-axis
    let num_labels = 6.min(data.points.len());
    let x_labels: Vec<Span> = if num_labels > 1 {
        (0..num_labels)
            .map(|i| {
                let idx = i * (data.points.len() - 1) / (num_labels - 1);
                let date = &data.points[idx].0;
                Span::raw(date.clone())
            })
            .collect()
    } else {
        vec![Span::raw(data.start_date.clone())]
    };

    let dataset = Dataset::default()
        .name("Commits")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data_points);

    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(format!(" {} ", data.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .x_axis(
            Axis::default()
                .title("Date")
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, x_max])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Commits")
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, y_max.max(1.0)])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(format!("{}", (y_max / 2.0).ceil() as u32)),
                    Span::raw(format!("{}", y_max.ceil() as u32)),
                ]),
        );

    frame.render_widget(chart, chart_area);
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

fn draw_git_tools_menu(frame: &mut Frame, app: &AppState, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            " Git Tools",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if let Some(err) = &app.git_tools.workspace_error {
        lines.push(Line::from(Span::styled(
            format!(" {err}"),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
        lines.push(hints_line(&[("Esc", "Home"), ("Q", "Quit")]));
    } else {
        // Show workspace info
        if !app.git_tools.repos.is_empty() {
            let repo_count = app.git_tools.repos.len();
            let mode = if repo_count == 1 {
                "Single repository".to_string()
            } else {
                format!("{repo_count} repositories")
            };
            lines.push(Line::from(vec![
                Span::styled("   Repos:  ", Style::default().fg(Color::DarkGray)),
                Span::raw(mode),
            ]));
            if !app.git_tools.selected_repos.is_empty() {
                let names: Vec<String> = app
                    .git_tools
                    .selected_repos
                    .iter()
                    .map(|r| path_display_name(r))
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("   Active: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(names.join(", ")),
                ]));
            }
            lines.push(Line::from(""));
        }

        for (i, (label, desc, _)) in GIT_TOOLS_MENU_ITEMS.iter().enumerate() {
            let is_selected = i == app.git_tools.selected;
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

            let item_name = format!("{prefix}{}. {label}", i + 1);
            let padding = " ".repeat(34usize.saturating_sub(item_name.len()));

            lines.push(Line::from(vec![
                Span::styled(item_name, style),
                Span::raw(padding),
                Span::styled(*desc, Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(""));
        if app.git_tools.repos.len() > 1 && !app.git_tools.selected_repos.is_empty() {
            lines.push(Line::from(Span::styled(
                " Esc Home to change repositories",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
        }
        lines.push(hints_line(&[
            ("\u{2191}\u{2193}", "Navigate"),
            ("Enter", "Select"),
            ("Esc", "Home"),
            ("Q", "Quit"),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_git_tools_repo_select(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            " Git Tools \u{2014} Select Repository",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Row 0: "Select all"
    let is_cursor_on_all = app.git_tools.repo_select_idx == 0;
    let all_check = if app.git_tools.all_repos_checked() {
        "[x]"
    } else {
        "[ ]"
    };
    let (all_prefix, all_style) = if is_cursor_on_all {
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
        format!("{all_prefix}{all_check} 0. Select all"),
        all_style,
    )));

    // Rows 1..=repos.len(): individual repos
    for (i, path) in app.git_tools.repos.iter().enumerate() {
        let is_cursor = (i + 1) == app.git_tools.repo_select_idx;
        let checked = app.git_tools.repo_checked.get(i).copied().unwrap_or(false);
        let check = if checked { "[x]" } else { "[ ]" };
        let (prefix, style) = if is_cursor {
            (
                "\u{27a4} ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default())
        };

        let name = path_display_name(path);
        let full_path = path.to_string_lossy();

        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}{check} {}. {name}", i + 1), style),
            Span::styled(
                format!("  ({full_path})"),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(hints_line(&[
        ("\u{2191}\u{2193}", "Navigate"),
        ("Space", "Toggle"),
        ("Enter", "Confirm"),
        ("Esc", "Back"),
        ("Q", "Quit"),
    ]));

    app.content_height = lines.len() as u16;
    app.visible_height = area.height;

    let paragraph = Paragraph::new(lines).scroll((app.git_tools.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn draw_git_tools_input(frame: &mut Frame, app: &AppState, area: Rect) {
    let mode_label = match &app.git_tools.mode {
        Some(GitToolsMode::MergedBranches) => "Remove Merged Branches",
        Some(GitToolsMode::StaleBranches) => "Remove Stale Branches",
        None => "Git Tools",
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Git Tools \u{2014} {mode_label}"),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Show which repo is targeted
    if !app.git_tools.selected_repos.is_empty() {
        let names: Vec<String> = app
            .git_tools
            .selected_repos
            .iter()
            .map(|r| path_display_name(r))
            .collect();
        let label = if names.len() == 1 { "Repo" } else { "Repos" };
        lines.push(Line::from(vec![
            Span::styled(
                format!("   {label}:  "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(names.join(", ")),
        ]));
        lines.push(Line::from(""));
    }

    if app.is_loading {
        let frame_idx = app.spinner_frame % SPINNER_FRAMES.len();
        let spinner = SPINNER_FRAMES[frame_idx];
        lines.push(Line::from(vec![
            Span::styled(format!(" {spinner}"), Style::default().fg(Color::Cyan)),
            Span::styled(
                " Scanning branches...".to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    } else if let Some(err) = &app.git_tools.error {
        lines.push(Line::from(Span::styled(
            format!(" Error: {err}"),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
        lines.push(hints_line(&[("Esc", "Back")]));
    } else {
        let prompt = match &app.git_tools.mode {
            Some(GitToolsMode::MergedBranches) => " Enter base branch name:",
            Some(GitToolsMode::StaleBranches) => " Enter number of days (default: 90):",
            None => " Input:",
        };

        lines.push(Line::from(Span::styled(
            prompt,
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw(" > "),
            Span::raw(&app.git_tools.input),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ]));

        lines.push(Line::from(""));
        lines.push(hints_line(&[("Enter", "Confirm"), ("Esc", "Back")]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_git_tools_branch_list(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let mode_label = match &app.git_tools.mode {
        Some(GitToolsMode::MergedBranches) => "Merged Branches",
        Some(GitToolsMode::StaleBranches) => "Stale Branches",
        None => "Branches",
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" Git Tools \u{2014} {mode_label}"),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.git_tools.branches.is_empty() {
        lines.push(Line::from(Span::styled(
            " No branches to remove.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(hints_line(&[("Esc", "Back")]));
    } else {
        lines.push(Line::from(Span::styled(
            " Review the branches below, then press y/Enter to delete or n/Esc to cancel.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        let multi_repo = app.git_tools.selected_repos.len() > 1;

        // Show protected branches that exist in the selected repos
        if !app.git_tools.protected_branches.is_empty() {
            lines.push(Line::from(Span::styled(
                " Protected branches (will not be touched):",
                Style::default().fg(Color::DarkGray),
            )));
            for (repo_name, branch_name) in &app.git_tools.protected_branches {
                let label = if multi_repo {
                    format!("   [{repo_name}] {branch_name}")
                } else {
                    format!("   {branch_name}")
                };
                lines.push(Line::from(Span::styled(
                    label,
                    Style::default().fg(Color::DarkGray),
                )));
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            format!(
                " The following {} branch(es) will be deleted:",
                app.git_tools.branches.len()
            ),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
        for branch in &app.git_tools.branches {
            let branch_label = if multi_repo {
                format!("   [{}] {}", branch.repo_display_name(), branch.name)
            } else {
                format!("   {}", branch.name)
            };
            let mut parts: Vec<Span> = vec![Span::raw(branch_label)];

            let mut tags = Vec::new();
            if branch.has_local {
                tags.push("local".to_string());
            }
            if branch.has_remote {
                tags.push("remote".to_string());
            }
            if let Some(date) = &branch.last_activity {
                tags.push(date.clone());
            }
            if !tags.is_empty() {
                parts.push(Span::styled(
                    format!("  ({})", tags.join(", ")),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(parts));
        }

        lines.push(Line::from(""));
        lines.push(hints_line(&[
            ("y/Enter", "Confirm delete"),
            ("n/Esc", "Cancel"),
            ("\u{2191}\u{2193}", "Scroll"),
        ]));
    }

    app.content_height = lines.len() as u16;
    app.visible_height = area.height;

    let paragraph = Paragraph::new(lines).scroll((app.git_tools.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn draw_git_tools_progress(frame: &mut Frame, app: &mut AppState, area: Rect) {
    let total = app.git_tools.progress.len();
    let completed = app
        .git_tools
        .progress
        .iter()
        .filter(|p| p.processed)
        .count();
    let success_count = app
        .git_tools
        .progress
        .iter()
        .filter(|p| p.processed && p.local_ok.unwrap_or(true) && p.remote_ok.unwrap_or(true))
        .count();

    let mut lines = vec![
        Line::from(Span::styled(
            " Git Tools \u{2014} Deleting Branches",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // Progress counter
    if app.git_tools.done {
        lines.push(Line::from(Span::styled(
            format!(" Done. {success_count}/{total} branch(es) deleted successfully."),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!(" Removed {completed}/{total}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    }

    // "Now processing" line or spinner
    if !app.git_tools.done {
        let multi_repo = app.git_tools.selected_repos.len() > 1;
        if multi_repo && !app.git_tools.current_repo.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" Now processing: {}", app.git_tools.current_repo),
                Style::default().fg(Color::Yellow),
            )));
        } else {
            let frame_idx = app.spinner_frame % SPINNER_FRAMES.len();
            let spinner = SPINNER_FRAMES[frame_idx];
            lines.push(Line::from(vec![
                Span::styled(format!(" {spinner}"), Style::default().fg(Color::Cyan)),
                Span::styled(" Deleting...", Style::default().fg(Color::Yellow)),
            ]));
        }
    }
    lines.push(Line::from(""));

    // Full branch list
    for (i, progress) in app.git_tools.progress.iter().enumerate() {
        if progress.processed {
            let overall_success =
                progress.local_ok.unwrap_or(true) && progress.remote_ok.unwrap_or(true);
            let (icon, icon_color) = if overall_success {
                ("\u{2713}", Color::Green)
            } else {
                ("\u{2717}", Color::Red)
            };
            let detail = format_delete_detail(progress);
            lines.push(Line::from(vec![
                Span::styled(format!("   {icon} "), Style::default().fg(icon_color)),
                Span::styled(progress.name.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!("  ({detail})"),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            let is_current = i == app.git_tools.current_index && !app.git_tools.done;
            if is_current {
                let frame_idx = app.spinner_frame % SPINNER_FRAMES.len();
                let spinner = SPINNER_FRAMES[frame_idx];
                lines.push(Line::from(vec![
                    Span::styled(format!("   {spinner} "), Style::default().fg(Color::Cyan)),
                    Span::styled(progress.name.clone(), Style::default().fg(Color::Yellow)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("   \u{2022} {}", progress.name),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    // Footer
    lines.push(Line::from(""));
    if app.git_tools.done {
        lines.push(hints_line(&[
            ("Enter/Esc", "Back"),
            ("\u{2191}\u{2193}", "Scroll"),
            ("Q", "Quit"),
        ]));
    } else {
        lines.push(hints_line(&[
            ("Esc", "Cancel"),
            ("\u{2191}\u{2193}", "Scroll"),
        ]));
    }

    app.content_height = lines.len() as u16;
    app.visible_height = area.height;

    let paragraph = Paragraph::new(lines).scroll((app.git_tools.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn format_delete_detail(p: &BranchProgress) -> String {
    let mut parts = Vec::new();
    match p.local_ok {
        Some(true) => parts.push("local"),
        Some(false) => parts.push("local failed"),
        None => {}
    }
    match p.remote_ok {
        Some(true) => parts.push("remote"),
        Some(false) => parts.push("remote failed"),
        None => {}
    }
    if parts.is_empty() {
        "no action".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repolyze_core::model::WeekBucket;

    /// Walk all spans in `lines` and return `true` if any carries `Color::Cyan` as its fg.
    fn any_span_is_cyan(lines: &[Line<'_>]) -> bool {
        lines
            .iter()
            .any(|line| line.spans.iter().any(|s| s.style.fg == Some(Color::Cyan)))
    }

    #[test]
    fn text_productivity_trend_lines_renders_cyan_bars() {
        let data = ProductivityTrendData {
            reference_date: "2026-04-15".to_string(),
            window_start: "2026-04-06".to_string(),
            window_end: "2026-04-13".to_string(),
            weeks: vec![
                WeekBucket {
                    week_start: "2026-04-06".to_string(),
                    commits: 3,
                },
                WeekBucket {
                    week_start: "2026-04-13".to_string(),
                    commits: 8,
                },
            ],
        };
        let lines = text_productivity_trend_lines(&data, 80);
        assert!(!lines.is_empty());
        assert!(
            any_span_is_cyan(&lines),
            "expected at least one span styled with Color::Cyan"
        );
    }

    #[test]
    fn text_productivity_trend_lines_empty_weeks_returns_empty() {
        let lines = text_productivity_trend_lines(&ProductivityTrendData::default(), 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn text_productivity_trend_lines_all_zero_no_cyan() {
        // All-zero data falls through to the "(no data)" placeholder path which must not emit
        // any colored bars (nothing to color).
        let data = ProductivityTrendData {
            reference_date: "2026-04-15".to_string(),
            window_start: "2026-04-06".to_string(),
            window_end: "2026-04-13".to_string(),
            weeks: vec![
                WeekBucket {
                    week_start: "2026-04-06".to_string(),
                    commits: 0,
                },
                WeekBucket {
                    week_start: "2026-04-13".to_string(),
                    commits: 0,
                },
            ],
        };
        let lines = text_productivity_trend_lines(&data, 80);
        assert!(!lines.is_empty());
        assert!(!any_span_is_cyan(&lines));
    }
}
