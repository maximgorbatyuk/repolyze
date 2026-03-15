use repolyze_core::model::ActivitySummary;

use crate::parse::ParsedCommit;

/// Build activity histograms from parsed commit timestamps.
///
/// Timestamps are expected in ISO 8601 format with offset (e.g. 2025-01-15T10:00:00+00:00).
/// We parse the local time (hour, weekday) to bucket commits.
pub fn build_activity_summary(commits: &[ParsedCommit]) -> ActivitySummary {
    let mut summary = ActivitySummary::default();

    for commit in commits {
        if let Some((hour, weekday)) = parse_hour_and_weekday(&commit.timestamp) {
            summary.by_hour[hour] += 1;
            summary.by_weekday[weekday] += 1;
            summary.heatmap[weekday][hour] += 1;
        }
    }

    summary
}

/// Parse hour (0-23) and weekday (0=Monday..6=Sunday) from an ISO 8601 timestamp.
pub(crate) fn parse_hour_and_weekday(timestamp: &str) -> Option<(usize, usize)> {
    // Format: 2025-01-15T10:00:00+00:00
    // We need the date part for weekday and time part for hour
    let t_pos = timestamp.find('T')?;
    let date_part = &timestamp[..t_pos];
    let time_part = &timestamp[t_pos + 1..];

    // Parse hour from time
    let hour: usize = time_part.get(..2)?.parse().ok()?;
    if hour >= 24 {
        return None;
    }

    // Parse date for weekday calculation
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }
    let year: i32 = date_parts[0].parse().ok()?;
    let month: u32 = date_parts[1].parse().ok()?;
    let day: u32 = date_parts[2].parse().ok()?;

    let weekday = day_of_week(year, month, day)?;

    Some((hour, weekday))
}

/// Zeller-like day-of-week calculation.
/// Returns 0=Monday, 1=Tuesday, ..., 6=Sunday.
fn day_of_week(year: i32, month: u32, day: u32) -> Option<usize> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Tomohiko Sakamoto's algorithm
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let dow = (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day as i32) % 7;
    // Sakamoto returns 0=Sunday, 1=Monday, ..., 6=Saturday
    // We want 0=Monday, ..., 6=Sunday
    let monday_based = ((dow + 6) % 7) as usize;
    Some(monday_based)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::ParsedCommit;

    fn make_commit(timestamp: &str) -> ParsedCommit {
        ParsedCommit {
            hash: "abc".to_string(),
            author_name: "Test".to_string(),
            author_email: "test@test.com".to_string(),
            timestamp: timestamp.to_string(),
            file_changes: Vec::new(),
        }
    }

    #[test]
    fn activity_histograms_use_commit_timestamps() {
        // 2025-01-15 is a Wednesday, 2025-01-16 is a Thursday
        let commits = vec![
            make_commit("2025-01-15T10:00:00+00:00"), // Wed, hour 10
            make_commit("2025-01-15T10:30:00+00:00"), // Wed, hour 10
            make_commit("2025-01-16T14:00:00+00:00"), // Thu, hour 14
        ];

        let summary = build_activity_summary(&commits);

        // Hour buckets
        assert_eq!(summary.by_hour[10], 2);
        assert_eq!(summary.by_hour[14], 1);
        assert_eq!(summary.by_hour.iter().sum::<u32>(), 3);

        // Day buckets (0=Monday, 2=Wednesday, 3=Thursday)
        assert_eq!(summary.by_weekday[2], 2); // Wednesday
        assert_eq!(summary.by_weekday[3], 1); // Thursday
        assert_eq!(summary.by_weekday.iter().sum::<u32>(), 3);

        // Heatmap
        assert_eq!(summary.heatmap[2][10], 2); // Wed hour 10
        assert_eq!(summary.heatmap[3][14], 1); // Thu hour 14
    }

    #[test]
    fn day_of_week_known_dates() {
        // 2025-01-15 is Wednesday (2)
        assert_eq!(day_of_week(2025, 1, 15), Some(2));
        // 2025-01-16 is Thursday (3)
        assert_eq!(day_of_week(2025, 1, 16), Some(3));
        // 2025-01-13 is Monday (0)
        assert_eq!(day_of_week(2025, 1, 13), Some(0));
        // 2025-01-19 is Sunday (6)
        assert_eq!(day_of_week(2025, 1, 19), Some(6));
    }

    #[test]
    fn handles_empty_commits() {
        let summary = build_activity_summary(&[]);
        assert_eq!(summary.by_hour.iter().sum::<u32>(), 0);
        assert_eq!(summary.by_weekday.iter().sum::<u32>(), 0);
    }
}
