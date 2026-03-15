use repolyze_core::model::{UserActivityRow, UsersContributionRow};

pub fn render_users_contribution_table(rows: &[UsersContributionRow]) -> String {
    if rows.is_empty() {
        return "No contributor data available.".to_string();
    }

    let headers = [
        "Email",
        "Commits",
        "Lines Modified",
        "Lines per commit",
        "Files Touched",
        "Most active week day",
    ];

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

    render_ascii_table(&headers, &data)
}

pub fn render_user_activity_table(rows: &[UserActivityRow]) -> String {
    if rows.is_empty() {
        return "No activity data available.".to_string();
    }

    let headers = [
        "Email",
        "Most active week day",
        "Average commits per day, in the most active day",
        "Average commits per day",
        "Average commits per hour, in the most active hour",
        "Average commits per hour",
    ];

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

    render_ascii_table(&headers, &data)
}

fn render_ascii_table(headers: &[&str], data: &[Vec<String>]) -> String {
    let col_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in data {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            widths[i] = widths[i].max(cell.len());
        }
    }

    let mut out = String::new();

    // Separator line
    let separator: String = widths
        .iter()
        .map(|w| format!("+-{}-", "-".repeat(*w)))
        .collect::<Vec<_>>()
        .join("")
        + "+\n";

    // Header
    out.push_str(&separator);
    out.push_str(&format_row(headers.iter().map(|h| h.to_string()), &widths));
    out.push_str(&separator);

    // Data rows
    for row in data {
        out.push_str(&format_row(row.iter().cloned(), &widths));
    }
    out.push_str(&separator);

    out
}

fn format_row(cells: impl Iterator<Item = String>, widths: &[usize]) -> String {
    let mut out = String::new();
    for (cell, width) in cells.zip(widths.iter()) {
        out.push_str(&format!("| {:<width$} ", cell, width = width));
    }
    out.push_str("|\n");
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

        assert!(table.contains("Average commits per day, in the most active day"));
        assert!(table.contains("Average commits per hour, in the most active hour"));
        assert!(table.contains("alice@example.com"));
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
