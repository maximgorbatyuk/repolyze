use std::process::Command;
use std::sync::mpsc::Sender;

use repolyze_core::error::RepolyzeError;

/// HTTP client that prefers `gh` CLI when available, falling back to `ureq`.
pub struct GitHubClient {
    mode: ClientMode,
    progress_tx: Option<Sender<String>>,
}

enum ClientMode {
    GhCli,
    DirectHttp,
}

impl GitHubClient {
    /// Detect whether `gh` CLI is authenticated and pick the best transport.
    pub fn new(progress_tx: Option<Sender<String>>) -> Self {
        let mode = if is_gh_authenticated() {
            ClientMode::GhCli
        } else {
            ClientMode::DirectHttp
        };
        Self { mode, progress_tx }
    }

    pub fn log(&self, msg: &str) {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(msg.to_string());
        }
    }

    /// GET a JSON endpoint. `endpoint` should start with `/` (e.g., `/repos/owner/repo`).
    pub fn get_json(&self, endpoint: &str) -> Result<serde_json::Value, RepolyzeError> {
        match &self.mode {
            ClientMode::GhCli => self.get_via_gh(endpoint),
            ClientMode::DirectHttp => self.get_via_http(endpoint),
        }
    }

    /// GET a paginated JSON array endpoint. Returns all pages concatenated.
    pub fn get_json_paginated(
        &self,
        endpoint: &str,
    ) -> Result<Vec<serde_json::Value>, RepolyzeError> {
        match &self.mode {
            ClientMode::GhCli => {
                // `gh api --paginate` returns all pages concatenated
                let value = self.get_via_gh_paginated(endpoint)?;
                match value {
                    serde_json::Value::Array(arr) => Ok(arr),
                    other => Ok(vec![other]),
                }
            }
            ClientMode::DirectHttp => self.get_paginated_http(endpoint),
        }
    }

    fn get_via_gh(&self, endpoint: &str) -> Result<serde_json::Value, RepolyzeError> {
        let output = Command::new("gh")
            .args(["api", endpoint])
            .output()
            .map_err(|e| RepolyzeError::GitHubApi(format!("failed to run gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RepolyzeError::GitHubApi(format!(
                "gh api {endpoint} failed: {stderr}"
            )));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| RepolyzeError::Parse(format!("invalid utf-8 in gh output: {e}")))?;

        serde_json::from_str(&stdout)
            .map_err(|e| RepolyzeError::Parse(format!("failed to parse gh response: {e}")))
    }

    fn get_via_gh_paginated(&self, endpoint: &str) -> Result<serde_json::Value, RepolyzeError> {
        let output = Command::new("gh")
            .args(["api", endpoint, "--paginate"])
            .output()
            .map_err(|e| RepolyzeError::GitHubApi(format!("failed to run gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RepolyzeError::GitHubApi(format!(
                "gh api {endpoint} failed: {stderr}"
            )));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| RepolyzeError::Parse(format!("invalid utf-8 in gh output: {e}")))?;

        // gh --paginate concatenates JSON arrays from each page.
        // Parse as a single array.
        serde_json::from_str(&stdout)
            .map_err(|e| RepolyzeError::Parse(format!("failed to parse gh response: {e}")))
    }

    fn get_via_http(&self, endpoint: &str) -> Result<serde_json::Value, RepolyzeError> {
        let url = format!("https://api.github.com{endpoint}");
        let response = ureq::get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "repolyze")
            .call()
            .map_err(|e| RepolyzeError::HttpError(format!("{e}")))?;

        self.log_rate_limit(&response);

        let body = response
            .into_body()
            .read_to_string()
            .map_err(|e| RepolyzeError::HttpError(format!("failed to read response: {e}")))?;

        serde_json::from_str(&body)
            .map_err(|e| RepolyzeError::Parse(format!("failed to parse response: {e}")))
    }

    fn get_paginated_http(&self, endpoint: &str) -> Result<Vec<serde_json::Value>, RepolyzeError> {
        let mut all_items = Vec::new();
        let mut url = format!("https://api.github.com{endpoint}");

        // Ensure per_page is set
        if !url.contains("per_page") {
            let sep = if url.contains('?') { '&' } else { '?' };
            url = format!("{url}{sep}per_page=100");
        }

        let mut page = 1;
        loop {
            self.log(&format!("Fetching page {page}..."));

            let response = ureq::get(&url)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "repolyze")
                .call()
                .map_err(|e| RepolyzeError::HttpError(format!("{e}")))?;

            self.log_rate_limit(&response);

            let next_url =
                parse_link_next(response.headers().get("link").and_then(|v| v.to_str().ok()));

            let body = response
                .into_body()
                .read_to_string()
                .map_err(|e| RepolyzeError::HttpError(format!("failed to read response: {e}")))?;

            let value: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| RepolyzeError::Parse(format!("failed to parse response: {e}")))?;

            if let serde_json::Value::Array(items) = value {
                if items.is_empty() {
                    break;
                }
                all_items.extend(items);
            } else {
                all_items.push(value);
                break;
            }

            match next_url {
                Some(next) => {
                    url = next;
                    page += 1;
                }
                None => break,
            }
        }

        Ok(all_items)
    }

    fn log_rate_limit(&self, response: &ureq::http::Response<ureq::Body>) {
        let Some(remaining) = response.headers().get("x-ratelimit-remaining") else {
            return;
        };
        let Ok(remaining_str) = remaining.to_str() else {
            return;
        };
        let Ok(n) = remaining_str.parse::<u32>() else {
            return;
        };
        if n < 10 {
            self.log(&format!(
                "Warning: GitHub API rate limit low ({n} remaining)"
            ));
        }
    }
}

/// Retry a request that may return HTTP 202 (computing statistics).
/// GitHub statistics endpoints return 202 when data is being computed.
pub fn retry_on_202<F>(mut fetch: F, max_retries: u32) -> Result<serde_json::Value, RepolyzeError>
where
    F: FnMut() -> Result<serde_json::Value, RepolyzeError>,
{
    for attempt in 0..max_retries {
        let value = fetch()?;

        // A 202 response from statistics endpoints returns an empty JSON object
        // while GitHub computes the data. We detect this by checking if the
        // expected array data is missing.
        if value.is_array()
            || (value.is_object() && value.as_object().is_some_and(|o| !o.is_empty()))
        {
            return Ok(value);
        }

        // Empty object likely means 202 — wait and retry
        if attempt < max_retries - 1 {
            let delay = std::time::Duration::from_secs(1 << attempt); // 1s, 2s, 4s
            std::thread::sleep(delay);
        }
    }

    Err(RepolyzeError::GitHubApi(
        "statistics not ready after retries (got 202)".to_string(),
    ))
}

fn is_gh_authenticated() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Parse the `Link` header to find the `rel="next"` URL.
fn parse_link_next(header: Option<&str>) -> Option<String> {
    let header = header?;
    for part in header.split(',') {
        let part = part.trim();
        if part.contains("rel=\"next\"") {
            // Extract URL from < ... >
            let start = part.find('<')? + 1;
            let end = part.find('>')?;
            return Some(part[start..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_link_next_extracts_url() {
        let header = r#"<https://api.github.com/repos/foo/bar/commits?page=2>; rel="next", <https://api.github.com/repos/foo/bar/commits?page=5>; rel="last""#;
        assert_eq!(
            parse_link_next(Some(header)),
            Some("https://api.github.com/repos/foo/bar/commits?page=2".to_string())
        );
    }

    #[test]
    fn parse_link_next_returns_none_when_no_next() {
        let header = r#"<https://api.github.com/repos/foo/bar/commits?page=1>; rel="first""#;
        assert_eq!(parse_link_next(Some(header)), None);
    }

    #[test]
    fn parse_link_next_returns_none_for_no_header() {
        assert_eq!(parse_link_next(None), None);
    }
}
