use std::collections::BTreeMap;

use repolyze_core::error::RepolyzeError;
use repolyze_core::model::SizeMetrics;

use crate::api_types::RepoInfo;
use crate::client::GitHubClient;

/// Fetch size metrics from GitHub API.
/// Uses `/repos/{owner}/{repo}` for overall size and `/languages` for breakdown.
pub fn fetch_size(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<(SizeMetrics, RepoInfo), RepolyzeError> {
    client.log("Fetching repository metadata...");
    let repo_value = client.get_json(&format!("/repos/{owner}/{repo}"))?;
    let repo_info: RepoInfo = serde_json::from_value(repo_value)
        .map_err(|e| RepolyzeError::Parse(format!("failed to parse repo info: {e}")))?;

    client.log("Fetching language breakdown...");
    let lang_value = client.get_json(&format!("/repos/{owner}/{repo}/languages"))?;
    let languages: BTreeMap<String, u64> = serde_json::from_value(lang_value)
        .map_err(|e| RepolyzeError::Parse(format!("failed to parse languages: {e}")))?;

    let total_bytes = repo_info.size * 1024; // API returns KB

    let by_extension: BTreeMap<String, u64> = languages
        .into_iter()
        .map(|(lang, bytes)| (lang.to_lowercase(), bytes))
        .collect();

    let metrics = SizeMetrics {
        files: 0,       // Not available from API
        directories: 0, // Not available from API
        total_bytes,
        total_lines: 0,     // Not available from API
        non_empty_lines: 0, // Not available from API
        blank_lines: 0,     // Not available from API
        by_extension,
        largest_files: vec![],
        average_file_size: 0.0,
    };

    Ok((metrics, repo_info))
}
