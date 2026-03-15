use repolyze_core::model::{UserActivityRow, UsersContributionRow};

pub fn render_users_contribution_table(rows: &[UsersContributionRow]) -> String {
    if rows.is_empty() {
        return "No contributor data available.".to_string();
    }

    let headers = &[
        "Email",
        "Commits",
        "Lines Modified",
        "Lines per commit",
        "Files Touched",
        "Most active week day",
    ];
    let right_align = &[false, true, true, true, true, false];

    let data: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.email.clone(),
                r.commits.to_string(),
                r.lines_modified.to_string(),
                format!("{:.2}", r.lines_per_commit),
                r.files_touched.to_string(),
                r.most_active_week_day.clone(),
            ]
        })
        .collect();

    let total_commits: u64 = rows.iter().map(|r| r.commits).sum();
    let total_lines: u64 = rows.iter().map(|r| r.lines_modified).sum();
    let total_files: u64 = rows.iter().map(|r| r.files_touched).sum();
    let total_lpc = if total_commits > 0 {
        total_lines as f64 / total_commits as f64
    } else {
        0.0
    };
    let totals = vec![
        "Total".to_string(),
        total_commits.to_string(),
        total_lines.to_string(),
        format!("{:.2}", total_lpc),
        total_files.to_string(),
        String::new(),
    ];

    render_plain_table(headers, &data, right_align, Some(&totals))
}

pub fn render_user_activity_table(rows: &[UserActivityRow]) -> String {
    if rows.is_empty() {
        return "No activity data available.".to_string();
    }

    let headers = &[
        "Email",
        "Most active week day",
        "Avg commits/day (best day)",
        "Avg commits/day",
        "Avg commits/hour (best hour)",
        "Avg commits/hour",
    ];
    let right_align = &[false, false, true, true, true, true];

    let data: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.email.clone(),
                r.most_active_week_day.clone(),
                format!("{:.2}", r.average_commits_per_day_in_most_active_day),
                format!("{:.2}", r.average_commits_per_day),
                format!("{:.2}", r.average_commits_per_hour_in_most_active_hour),
                format!("{:.2}", r.average_commits_per_hour),
            ]
        })
        .collect();

    render_plain_table(headers, &data, right_align, None)
}

fn render_plain_table(
    headers: &[&str],
    data: &[Vec<String>],
    right_align: &[bool],
    totals: Option<&[String]>,
) -> String {
    let col_count = headers.len();

    // Compute column widths from headers, data, and optional totals
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in data {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            widths[i] = widths[i].max(cell.len());
        }
    }
    if let Some(t) = totals {
        for (i, cell) in t.iter().enumerate().take(col_count) {
            widths[i] = widths[i].max(cell.len());
        }
    }

    let mut out = String::new();

    // Header row — left-aligned always
    out.push_str(&format_row_plain(
        headers.iter().map(|h| h.to_string()),
        &widths,
        &vec![false; col_count],
    ));

    // Separator
    out.push_str(&separator_line(&widths));

    // Data rows
    for row in data {
        out.push_str(&format_row_plain(row.iter().cloned(), &widths, right_align));
    }

    // Totals
    if let Some(t) = totals {
        out.push_str(&separator_line(&widths));
        out.push_str(&format_row_plain(t.iter().cloned(), &widths, right_align));
    }

    out
}

fn separator_line(widths: &[usize]) -> String {
    let mut out = String::new();
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*w));
    }
    out.push('\n');
    out
}

fn format_row_plain(
    cells: impl Iterator<Item = String>,
    widths: &[usize],
    right_align: &[bool],
) -> String {
    let cells_vec: Vec<String> = cells.collect();
    let mut out = String::new();
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let cell = cells_vec.get(i).map(|s| s.as_str()).unwrap_or("");
        let is_right = right_align.get(i).copied().unwrap_or(false);
        if is_right {
            out.push_str(&format!("{:>width$}", cell, width = width));
        } else {
            out.push_str(&format!("{:<width$}", cell, width = width));
        }
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_users_contribution_table_uses_rf8_headers() {
        let rows = vec![UsersContributionRow {
            email: "alice@example.com".to_string(),
            commits: 5,
            lines_modified: 42,
            lines_per_commit: 8.4,
            files_touched: 4,
            most_active_week_day: "Monday".to_string(),
        }];

        let table = render_users_contribution_table(&rows);

        assert!(table.contains("Email"));
        assert!(table.contains("Commits"));
        assert!(table.contains("Lines Modified"));
        assert!(table.contains("Lines per commit"));
        assert!(table.contains("Files Touched"));
        assert!(table.contains("Most active week day"));
        assert!(table.contains("alice@example.com"));
        assert!(table.contains("Total"));
    }

    #[test]
    fn render_users_contribution_table_right_aligns_numbers() {
        let rows = vec![
            UsersContributionRow {
                email: "alice@example.com".to_string(),
                commits: 100,
                lines_modified: 5000,
                lines_per_commit: 50.0,
                files_touched: 20,
                most_active_week_day: "Monday".to_string(),
            },
            UsersContributionRow {
                email: "bob@example.com".to_string(),
                commits: 5,
                lines_modified: 42,
                lines_per_commit: 8.4,
                files_touched: 4,
                most_active_week_day: "Friday".to_string(),
            },
        ];

        let table = render_users_contribution_table(&rows);
        // Total row should be present
        assert!(table.contains("Total"));
        assert!(table.contains("105")); // 100 + 5
        assert!(table.contains("5042")); // 5000 + 42
        // Separators use dashes, not pipes
        assert!(!table.contains('|'));
        assert!(table.contains("---"));
    }

    #[test]
    fn render_user_activity_table_uses_rf9_headers() {
        let rows = vec![UserActivityRow {
            email: "alice@example.com".to_string(),
            most_active_week_day: "Monday".to_string(),
            average_commits_per_day_in_most_active_day: 2.0,
            average_commits_per_day: 1.5,
            average_commits_per_hour_in_most_active_hour: 2.0,
            average_commits_per_hour: 1.0,
        }];

        let table = render_user_activity_table(&rows);

        assert!(table.contains("Avg commits/day (best day)"));
        assert!(table.contains("Avg commits/hour (best hour)"));
        assert!(table.contains("alice@example.com"));
        assert!(!table.contains('|'));
    }

    #[test]
    fn render_empty_table_returns_helpful_message() {
        assert_eq!(
            render_users_contribution_table(&[]),
            "No contributor data available."
        );
        assert_eq!(
            render_user_activity_table(&[]),
            "No activity data available."
        );
    }
}
