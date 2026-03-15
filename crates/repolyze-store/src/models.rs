#[derive(Debug, Clone)]
pub struct ContributorRecord {
    pub canonical_email: String,
    pub display_name_last_seen: String,
}

impl ContributorRecord {
    pub fn new(email: &str, name: &str) -> Self {
        Self {
            canonical_email: email.to_lowercase(),
            display_name_last_seen: name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitRecord {
    pub repository_id: i64,
    pub contributor_id: i64,
    pub commit_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub committed_at: String,
    pub commit_date: String,
    pub commit_hour: i64,
    pub commit_weekday: i64,
    pub files_changed_count: i64,
    pub lines_added: i64,
    pub lines_deleted: i64,
    pub lines_modified: i64,
}

impl CommitRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository_id: i64,
        contributor_id: i64,
        commit_hash: &str,
        author_name: &str,
        author_email: &str,
        committed_at: &str,
        commit_hour: i64,
        commit_weekday: i64,
        files_changed_count: i64,
        lines_added: i64,
        lines_deleted: i64,
        lines_modified: i64,
    ) -> Self {
        let commit_date = committed_at
            .split('T')
            .next()
            .unwrap_or_default()
            .to_string();
        Self {
            repository_id,
            contributor_id,
            commit_hash: commit_hash.to_string(),
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            committed_at: committed_at.to_string(),
            commit_date,
            commit_hour,
            commit_weekday,
            files_changed_count,
            lines_added,
            lines_deleted,
            lines_modified,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitFileChangeRecord {
    pub file_path: String,
    pub additions: i64,
    pub deletions: i64,
    pub lines_modified: i64,
}

impl CommitFileChangeRecord {
    pub fn new(file_path: &str, additions: i64, deletions: i64, lines_modified: i64) -> Self {
        Self {
            file_path: file_path.to_string(),
            additions,
            deletions,
            lines_modified,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UsersContributionRowRecord {
    pub email: String,
    pub commits: i64,
    pub lines_modified: i64,
    pub lines_per_commit: f64,
    pub files_touched: i64,
    pub most_active_week_day: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRowCount {
    pub table_name: String,
    pub record_count: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseMetadata {
    pub tables: Vec<TableRowCount>,
    pub total_rows: i64,
}

#[derive(Debug, Clone)]
pub struct UserActivityRowRecord {
    pub email: String,
    pub most_active_week_day: String,
    pub average_commits_per_day_in_most_active_day: f64,
    pub average_commits_per_day: f64,
    pub average_commits_per_hour_in_most_active_hour: f64,
    pub average_commits_per_hour: f64,
}
