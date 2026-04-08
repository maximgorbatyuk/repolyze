use repolyze_core::error::RepolyzeError;
use repolyze_core::model::ActivitySummary;

use crate::api_types::PunchCardEntry;
use crate::client::GitHubClient;

/// Fetch activity data from `/stats/punch_card` endpoint.
pub fn fetch_activity(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<ActivitySummary, RepolyzeError> {
    client.log("Fetching activity punch card...");
    let endpoint = format!("/repos/{owner}/{repo}/stats/punch_card");

    let value = crate::client::retry_on_202(|| client.get_json(&endpoint), 4)?;

    let entries: Vec<PunchCardEntry> = serde_json::from_value(value)
        .map_err(|e| RepolyzeError::Parse(format!("failed to parse punch card: {e}")))?;

    Ok(build_activity_summary(&entries))
}

fn build_activity_summary(entries: &[PunchCardEntry]) -> ActivitySummary {
    let mut by_hour = [0u32; 24];
    let mut by_weekday = [0u32; 7];
    let mut heatmap = [[0u32; 24]; 7];

    for entry in entries {
        let gh_day = entry.0 as usize; // GitHub: 0=Sunday
        let hour = entry.1 as usize;
        let commits = entry.2;

        if hour >= 24 || gh_day >= 7 {
            continue;
        }

        // Remap: GitHub 0=Sunday -> our 0=Monday
        let weekday = (gh_day + 6) % 7;

        by_hour[hour] += commits;
        by_weekday[weekday] += commits;
        heatmap[weekday][hour] += commits;
    }

    ActivitySummary {
        by_hour,
        by_weekday,
        heatmap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_activity_summary_remaps_weekdays() {
        let entries = vec![
            PunchCardEntry(0, 10, 5), // Sunday hour 10 -> weekday 6
            PunchCardEntry(1, 14, 3), // Monday hour 14 -> weekday 0
            PunchCardEntry(6, 9, 2),  // Saturday hour 9 -> weekday 5
        ];

        let summary = build_activity_summary(&entries);

        assert_eq!(summary.by_weekday[6], 5); // Sunday
        assert_eq!(summary.by_weekday[0], 3); // Monday
        assert_eq!(summary.by_weekday[5], 2); // Saturday

        assert_eq!(summary.by_hour[10], 5);
        assert_eq!(summary.by_hour[14], 3);
        assert_eq!(summary.by_hour[9], 2);

        assert_eq!(summary.heatmap[6][10], 5); // Sunday, hour 10
        assert_eq!(summary.heatmap[0][14], 3); // Monday, hour 14
    }
}
