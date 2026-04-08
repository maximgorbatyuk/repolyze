use serde::Deserialize;

/// Repository metadata from `GET /repos/{owner}/{repo}`.
#[derive(Debug, Deserialize)]
pub struct RepoInfo {
    #[allow(dead_code)]
    pub full_name: String,
    /// Size in kilobytes.
    pub size: u64,
    #[allow(dead_code)]
    pub default_branch: String,
}

/// Per-contributor statistics from `GET /repos/{owner}/{repo}/stats/contributors`.
#[derive(Debug, Deserialize)]
pub struct ContributorStat {
    pub author: Option<GitHubAuthor>,
    pub total: u64,
    pub weeks: Vec<WeekStat>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubAuthor {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct WeekStat {
    /// Unix timestamp of week start.
    #[allow(dead_code)]
    pub w: u64,
    /// Additions.
    pub a: u64,
    /// Deletions.
    pub d: u64,
    /// Commits.
    #[allow(dead_code)]
    pub c: u32,
}

/// Punch card entry from `GET /repos/{owner}/{repo}/stats/punch_card`.
/// Format: [day_of_week, hour, commit_count]
/// day_of_week: 0 = Sunday, 6 = Saturday
#[derive(Debug, Deserialize)]
pub struct PunchCardEntry(pub u32, pub u32, pub u32);

/// Commit info from `GET /repos/{owner}/{repo}/commits`.
#[derive(Debug, Deserialize)]
pub struct CommitInfo {
    #[allow(dead_code)]
    pub sha: String,
    pub commit: CommitDetail,
    pub author: Option<CommitGitHubAuthor>,
}

#[derive(Debug, Deserialize)]
pub struct CommitDetail {
    pub author: CommitAuthor,
}

#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitGitHubAuthor {
    pub login: String,
}
