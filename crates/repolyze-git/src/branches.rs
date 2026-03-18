use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use repolyze_core::date_util;
use repolyze_core::error::RepolyzeError;

use crate::backend::run_git;

const PROTECTED_BRANCHES: &[&str] = &[
    "main",
    "master",
    "dev",
    "develop",
    "development",
    "sandbox",
    "prod",
    "production",
    "demo",
];

/// NOTE: Only the "origin" remote is supported. Branches tracked under other
/// remotes (e.g. "upstream") are not detected or deleted.
const REMOTE_NAME: &str = "origin";

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub has_local: bool,
    pub has_remote: bool,
    pub last_activity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub branch: String,
    pub local_ok: Option<bool>,
    pub remote_ok: Option<bool>,
    pub error: Option<String>,
}

fn is_protected(name: &str) -> bool {
    PROTECTED_BRANCHES.contains(&name)
}

fn current_branch(repo: &Path) -> Result<String, RepolyzeError> {
    Ok(run_git(repo, &["branch", "--show-current"])?
        .trim()
        .to_string())
}

pub fn list_merged_branches(
    repo: &Path,
    base_branch: &str,
) -> Result<Vec<BranchInfo>, RepolyzeError> {
    let current = current_branch(repo)?;

    // Local branches merged into base
    let local_output = run_git(repo, &["branch", "--merged", base_branch])?;
    let local_branches: Vec<String> = local_output
        .lines()
        .map(|l| l.trim().trim_start_matches("* ").to_string())
        .filter(|name| {
            !name.is_empty() && !is_protected(name) && *name != base_branch && *name != current
        })
        .collect();

    // Remote branches merged into base (origin only)
    let remote_prefix = format!("{REMOTE_NAME}/");
    let remote_output =
        run_git(repo, &["branch", "-r", "--merged", base_branch]).unwrap_or_default();
    let remote_branches: Vec<String> = remote_output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|name| !name.contains("->")) // skip HEAD -> origin/main
        .filter_map(|name| name.strip_prefix(&remote_prefix).map(|s| s.to_string()))
        .filter(|name| !is_protected(name) && *name != base_branch && *name != current)
        .collect();

    // Merge local and remote info
    let mut branches: Vec<BranchInfo> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for name in &local_branches {
        seen.insert(name.clone());
        let has_remote = remote_branches.contains(name);
        branches.push(BranchInfo {
            name: name.clone(),
            has_local: true,
            has_remote,
            last_activity: None,
        });
    }

    for name in &remote_branches {
        if !seen.contains(name) {
            branches.push(BranchInfo {
                name: name.clone(),
                has_local: false,
                has_remote: true,
                last_activity: None,
            });
        }
    }

    branches.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(branches)
}

pub fn list_stale_branches(repo: &Path, days: u64) -> Result<Vec<BranchInfo>, RepolyzeError> {
    let current = current_branch(repo)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RepolyzeError::Parse(format!("system time error: {e}")))?
        .as_secs();
    let threshold = now.saturating_sub(days.saturating_mul(86400));

    // Local branches with last commit timestamp
    let local_output = run_git(
        repo,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short) %(committerdate:unix)",
            "refs/heads/",
        ],
    )?;

    let mut branches: Vec<BranchInfo> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in local_output.lines() {
        let parts: Vec<&str> = line.rsplitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }
        let timestamp: u64 = match parts[0].parse() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let name = parts[1].to_string();

        if is_protected(&name) || name == current {
            continue;
        }

        if timestamp < threshold {
            seen.insert(name.clone());
            let date = date_util::format_unix_timestamp(timestamp);
            branches.push(BranchInfo {
                name: name.clone(),
                has_local: true,
                has_remote: false,
                last_activity: Some(date),
            });
        }
    }

    // Check remote branches too (origin only)
    let remote_ref_prefix = format!("refs/remotes/{REMOTE_NAME}/");
    let remote_output = run_git(
        repo,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short) %(committerdate:unix)",
            &remote_ref_prefix,
        ],
    )
    .unwrap_or_default();

    let strip_prefix = format!("{REMOTE_NAME}/");
    for line in remote_output.lines() {
        let parts: Vec<&str> = line.rsplitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }
        let timestamp: u64 = match parts[0].parse() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let full_ref = parts[1].to_string();
        let name = match full_ref.strip_prefix(&strip_prefix) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if is_protected(&name) || name == current || name.contains("HEAD") {
            continue;
        }

        if timestamp < threshold {
            if let Some(existing) = branches.iter_mut().find(|b| b.name == name) {
                existing.has_remote = true;
            } else if !seen.contains(&name) {
                seen.insert(name.clone());
                let date = date_util::format_unix_timestamp(timestamp);
                branches.push(BranchInfo {
                    name: name.clone(),
                    has_local: false,
                    has_remote: true,
                    last_activity: Some(date),
                });
            }
        }
    }

    branches.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(branches)
}

/// Delete a branch locally and/or from the "origin" remote.
pub fn delete_branch(repo: &Path, branch: &BranchInfo, force: bool) -> DeleteResult {
    let mut result = DeleteResult {
        branch: branch.name.clone(),
        local_ok: None,
        remote_ok: None,
        error: None,
    };

    if branch.has_local {
        let flag = if force { "-D" } else { "-d" };
        match run_git(repo, &["branch", flag, &branch.name]) {
            Ok(_) => result.local_ok = Some(true),
            Err(e) => {
                result.local_ok = Some(false);
                result.error = Some(format!("local: {e}"));
            }
        }
    }

    if branch.has_remote {
        match run_git(repo, &["push", REMOTE_NAME, "--delete", &branch.name]) {
            Ok(_) => result.remote_ok = Some(true),
            Err(e) => {
                let msg = format!("remote: {e}");
                if let Some(existing) = &result.error {
                    result.error = Some(format!("{existing}; {msg}"));
                } else {
                    result.error = Some(msg);
                }
                result.remote_ok = Some(false);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_branches_are_detected() {
        assert!(is_protected("main"));
        assert!(is_protected("master"));
        assert!(is_protected("dev"));
        assert!(is_protected("develop"));
        assert!(is_protected("development"));
        assert!(is_protected("sandbox"));
        assert!(is_protected("prod"));
        assert!(is_protected("production"));
        assert!(is_protected("demo"));
        assert!(!is_protected("feature/foo"));
        assert!(!is_protected("bugfix/bar"));
    }

    #[test]
    fn list_merged_branches_in_fixture_repo() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Init repo with main branch
        run_cmd(root, &["git", "init", "-b", "main"]);
        run_cmd(root, &["git", "config", "user.name", "Test"]);
        run_cmd(root, &["git", "config", "user.email", "test@test.com"]);
        std::fs::write(root.join("README.md"), "# Test\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "initial"]);

        // Create and merge a feature branch
        run_cmd(root, &["git", "checkout", "-b", "feature/merged"]);
        std::fs::write(root.join("feature.txt"), "feature\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "feature"]);
        run_cmd(root, &["git", "checkout", "main"]);
        run_cmd(root, &["git", "merge", "feature/merged"]);

        // Create an unmerged branch
        run_cmd(root, &["git", "checkout", "-b", "feature/unmerged"]);
        std::fs::write(root.join("unmerged.txt"), "unmerged\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "unmerged"]);
        run_cmd(root, &["git", "checkout", "main"]);

        let branches = list_merged_branches(root, "main").unwrap();
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
        assert!(
            names.contains(&"feature/merged"),
            "should list merged branch"
        );
        assert!(
            !names.contains(&"feature/unmerged"),
            "should not list unmerged branch"
        );
        assert!(!names.contains(&"main"), "should not list base branch");
    }

    #[test]
    fn list_stale_branches_finds_old_branches() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        run_cmd(root, &["git", "init", "-b", "main"]);
        run_cmd(root, &["git", "config", "user.name", "Test"]);
        run_cmd(root, &["git", "config", "user.email", "test@test.com"]);
        std::fs::write(root.join("README.md"), "# Test\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd_env(
            root,
            &["git", "commit", "-m", "initial"],
            &[
                ("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+00:00"),
                ("GIT_COMMITTER_DATE", "2020-01-01T00:00:00+00:00"),
            ],
        );

        // Create a stale branch with old timestamp
        run_cmd(root, &["git", "checkout", "-b", "feature/stale"]);
        std::fs::write(root.join("stale.txt"), "stale\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd_env(
            root,
            &["git", "commit", "-m", "stale work"],
            &[
                ("GIT_AUTHOR_DATE", "2020-06-01T00:00:00+00:00"),
                ("GIT_COMMITTER_DATE", "2020-06-01T00:00:00+00:00"),
            ],
        );
        run_cmd(root, &["git", "checkout", "main"]);

        let branches = list_stale_branches(root, 90).unwrap();
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"feature/stale"), "should list stale branch");
        assert!(!names.contains(&"main"), "should not list protected branch");
    }

    #[test]
    fn delete_local_branch_in_fixture_repo() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        run_cmd(root, &["git", "init", "-b", "main"]);
        run_cmd(root, &["git", "config", "user.name", "Test"]);
        run_cmd(root, &["git", "config", "user.email", "test@test.com"]);
        std::fs::write(root.join("README.md"), "# Test\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "initial"]);

        // Create and merge a branch so -d works
        run_cmd(root, &["git", "checkout", "-b", "feature/to-delete"]);
        std::fs::write(root.join("f.txt"), "content\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "feature"]);
        run_cmd(root, &["git", "checkout", "main"]);
        run_cmd(root, &["git", "merge", "feature/to-delete"]);

        let branch = BranchInfo {
            name: "feature/to-delete".to_string(),
            has_local: true,
            has_remote: false,
            last_activity: None,
        };
        let result = delete_branch(root, &branch, false);
        assert_eq!(result.local_ok, Some(true));
        assert_eq!(result.remote_ok, None); // no remote to delete
        assert!(result.error.is_none());

        // Verify branch is gone
        let output = run_git(root, &["branch"]).unwrap();
        assert!(!output.contains("feature/to-delete"));
    }

    #[test]
    fn force_delete_unmerged_branch() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        run_cmd(root, &["git", "init", "-b", "main"]);
        run_cmd(root, &["git", "config", "user.name", "Test"]);
        run_cmd(root, &["git", "config", "user.email", "test@test.com"]);
        std::fs::write(root.join("README.md"), "# Test\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "initial"]);

        // Create an unmerged branch
        run_cmd(root, &["git", "checkout", "-b", "feature/unmerged"]);
        std::fs::write(root.join("f.txt"), "content\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "wip"]);
        run_cmd(root, &["git", "checkout", "main"]);

        // Non-force delete should fail
        let branch = BranchInfo {
            name: "feature/unmerged".to_string(),
            has_local: true,
            has_remote: false,
            last_activity: None,
        };
        let result = delete_branch(root, &branch, false);
        assert_eq!(result.local_ok, Some(false));
        assert!(result.error.is_some());

        // Force delete should succeed
        let result = delete_branch(root, &branch, true);
        assert_eq!(result.local_ok, Some(true));
        assert!(result.error.is_none());
    }

    #[test]
    fn delete_remote_only_branch_reports_failure_without_remote() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        run_cmd(root, &["git", "init", "-b", "main"]);
        run_cmd(root, &["git", "config", "user.name", "Test"]);
        run_cmd(root, &["git", "config", "user.email", "test@test.com"]);
        std::fs::write(root.join("README.md"), "# Test\n").unwrap();
        run_cmd(root, &["git", "add", "."]);
        run_cmd(root, &["git", "commit", "-m", "initial"]);

        // No remote configured, so remote delete should fail gracefully
        let branch = BranchInfo {
            name: "feature/remote-only".to_string(),
            has_local: false,
            has_remote: true,
            last_activity: None,
        };
        let result = delete_branch(root, &branch, false);
        assert_eq!(result.local_ok, None); // no local to delete
        assert_eq!(result.remote_ok, Some(false));
        assert!(result.error.is_some());
    }

    fn run_cmd(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new(args[0])
            .args(&args[1..])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "command failed: {} — {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_cmd_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) {
        let mut cmd = std::process::Command::new(args[0]);
        cmd.args(&args[1..]).current_dir(dir);
        for (key, val) in env {
            cmd.env(key, val);
        }
        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "command failed: {} — {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
