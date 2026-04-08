use std::collections::{BTreeMap, BTreeSet, HashMap};

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::{ContributionSummary, ContributorActivityStats, ContributorStats};

use crate::api_types::{CommitInfo, ContributorStat};
use crate::client::GitHubClient;

/// Fetch contribution stats by combining `/stats/contributors` (aggregate data)
/// with `/commits` (author details and timestamps).
pub fn fetch_contributions(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<ContributionSummary, RepolyzeError> {
    client.log("Fetching contributor statistics...");
    let stats = fetch_contributor_stats(client, owner, repo)?;

    client.log("Fetching commit history...");
    let commits = fetch_commits(client, owner, repo)?;

    build_contribution_summary(&stats, &commits)
}

fn fetch_contributor_stats(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<ContributorStat>, RepolyzeError> {
    let endpoint = format!("/repos/{owner}/{repo}/stats/contributors");

    let value = crate::client::retry_on_202(|| client.get_json(&endpoint), 4)?;

    let stats: Vec<ContributorStat> = serde_json::from_value(value)
        .map_err(|e| RepolyzeError::Parse(format!("failed to parse contributor stats: {e}")))?;

    Ok(stats)
}

fn fetch_commits(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<CommitInfo>, RepolyzeError> {
    let endpoint = format!("/repos/{owner}/{repo}/commits?per_page=100");

    let items = client.get_json_paginated(&endpoint)?;

    let mut commits = Vec::new();
    let mut skipped = 0u32;
    for item in items {
        match serde_json::from_value::<CommitInfo>(item) {
            Ok(commit) => commits.push(commit),
            Err(_) => skipped += 1,
        }
    }

    if skipped > 0 {
        client.log(&format!("Skipped {skipped} malformed commit entries"));
    }
    client.log(&format!("Fetched {} commits", commits.len()));
    Ok(commits)
}

fn build_contribution_summary(
    stats: &[ContributorStat],
    commits: &[CommitInfo],
) -> Result<ContributionSummary, RepolyzeError> {
    // Build a mapping from GitHub login to (name, email) using commit data
    let mut login_to_identity: HashMap<String, (String, String)> = HashMap::new();
    for commit in commits {
        if let Some(gh_author) = &commit.author {
            let login = gh_author.login.to_lowercase();
            login_to_identity.entry(login).or_insert_with(|| {
                (
                    commit.commit.author.name.clone(),
                    commit.commit.author.email.clone(),
                )
            });
        }
    }

    // Build per-email commit data from the commit list for timestamps and activity
    let mut email_commits: HashMap<String, Vec<&CommitInfo>> = HashMap::new();
    for commit in commits {
        let email = commit.commit.author.email.to_lowercase();
        email_commits.entry(email).or_default().push(commit);
    }

    let mut contributors: Vec<ContributorStats> = Vec::new();
    let mut activity_by_contributor: Vec<ContributorActivityStats> = Vec::new();
    let mut total_commits: u64 = 0;

    for stat in stats {
        let login = stat
            .author
            .as_ref()
            .map(|a| a.login.to_lowercase())
            .unwrap_or_default();

        let (name, email) = login_to_identity
            .get(&login)
            .cloned()
            .unwrap_or_else(|| (login.clone(), format!("{login}@users.noreply.github.com")));

        let email_lower = email.to_lowercase();

        // Aggregate from weekly stats
        let mut lines_added: u64 = 0;
        let mut lines_deleted: u64 = 0;
        for week in &stat.weeks {
            lines_added += week.a;
            lines_deleted += week.d;
        }

        // Get commit timestamps for this contributor
        let user_commits = email_commits.get(&email_lower);
        let mut dates: BTreeSet<String> = BTreeSet::new();
        let mut timestamps: Vec<String> = Vec::new();
        let mut weekday_commits = [0u32; 7];
        let mut hour_commits = [0u32; 24];
        let mut active_dates_by_weekday: [BTreeSet<String>; 7] =
            std::array::from_fn(|_| BTreeSet::new());
        let mut active_hour_buckets: BTreeSet<String> = BTreeSet::new();
        let mut active_hour_buckets_by_hour: [BTreeSet<String>; 24] =
            std::array::from_fn(|_| BTreeSet::new());
        let mut commits_by_date: BTreeMap<String, u32> = BTreeMap::new();

        if let Some(user_commits) = user_commits {
            for commit in user_commits {
                let ts = &commit.commit.author.date;
                timestamps.push(ts.clone());

                let date = ts.split('T').next().unwrap_or_default().to_string();
                if !date.is_empty() {
                    dates.insert(date.clone());
                    *commits_by_date.entry(date.clone()).or_insert(0) += 1;
                }

                if let Some((hour, weekday)) = parse_hour_and_weekday(ts) {
                    weekday_commits[weekday] += 1;
                    hour_commits[hour] += 1;
                    active_dates_by_weekday[weekday].insert(date.clone());
                    let hour_bucket = format!("{date}:{hour}");
                    active_hour_buckets.insert(hour_bucket.clone());
                    active_hour_buckets_by_hour[hour].insert(hour_bucket);
                }
            }
        }

        let first_commit = timestamps.iter().min().cloned().unwrap_or_default();
        let last_commit = timestamps.iter().max().cloned().unwrap_or_default();

        total_commits += stat.total;

        contributors.push(ContributorStats {
            name,
            email: email_lower.clone(),
            commits: stat.total,
            lines_added,
            lines_deleted,
            net_lines: (lines_added as i64).saturating_sub(lines_deleted as i64),
            files_touched: 0, // Not available without per-commit file data
            file_extensions: BTreeMap::new(),
            active_days: dates.len() as u64,
            first_commit,
            last_commit,
        });

        activity_by_contributor.push(ContributorActivityStats {
            email: email_lower,
            weekday_commits,
            hour_commits,
            active_dates: dates,
            active_dates_by_weekday,
            active_hour_buckets,
            active_hour_buckets_by_hour,
            commits_by_date,
        });
    }

    // Sort by commits descending
    contributors.sort_by(|a, b| b.commits.cmp(&a.commits));
    activity_by_contributor.sort_by(|a, b| {
        let a_total: u32 = a.weekday_commits.iter().sum();
        let b_total: u32 = b.weekday_commits.iter().sum();
        b_total.cmp(&a_total).then(a.email.cmp(&b.email))
    });

    Ok(ContributionSummary {
        contributors,
        activity_by_contributor,
        total_commits,
    })
}

/// Parse ISO 8601 timestamp to extract (hour, weekday).
/// Weekday: 0=Monday, 6=Sunday (matching repolyze convention).
fn parse_hour_and_weekday(timestamp: &str) -> Option<(usize, usize)> {
    // Format: "2025-01-15T10:30:00Z" or "2025-01-15T10:30:00+00:00"
    let t_pos = timestamp.find('T')?;
    let date_part = &timestamp[..t_pos];
    let time_part = &timestamp[t_pos + 1..];

    // Parse hour
    let hour: usize = time_part.get(..2)?.parse().ok()?;
    if hour >= 24 {
        return None;
    }

    // Parse date to get weekday using Tomohiko Sakamoto's algorithm
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;

    let weekday = day_of_week(y, m, d)?;
    Some((hour, weekday))
}

/// Tomohiko Sakamoto's algorithm: returns 0=Monday, 6=Sunday.
fn day_of_week(mut y: i32, m: u32, d: u32) -> Option<usize> {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    if m < 3 {
        y -= 1;
    }
    let dow = ((y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d as i32) % 7) as usize;
    // Sakamoto returns 0=Sunday; convert to 0=Monday
    Some((dow + 6) % 7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hour_and_weekday_utc() {
        // 2025-01-13 is Monday
        let result = parse_hour_and_weekday("2025-01-13T10:00:00Z");
        assert_eq!(result, Some((10, 0))); // hour=10, Monday=0
    }

    #[test]
    fn parse_hour_and_weekday_with_offset() {
        let result = parse_hour_and_weekday("2025-01-15T14:30:00+00:00");
        assert_eq!(result, Some((14, 2))); // hour=14, Wednesday=2
    }

    #[test]
    fn day_of_week_monday() {
        assert_eq!(day_of_week(2025, 1, 13), Some(0)); // Monday
    }

    #[test]
    fn day_of_week_sunday() {
        assert_eq!(day_of_week(2025, 1, 19), Some(6)); // Sunday
    }
}
