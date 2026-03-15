use std::process::Command as GitCommand;

use assert_cmd::Command;
use predicates::prelude::*;

fn create_fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    GitCommand::new("git")
        .args(["init"])
        .current_dir(root)
        .output()
        .unwrap();
    GitCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(root)
        .output()
        .unwrap();
    GitCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(root)
        .output()
        .unwrap();

    std::fs::write(root.join("README.md"), "# Test\n").unwrap();

    GitCommand::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()
        .unwrap();

    let output = GitCommand::new("git")
        .args(["commit", "-m", "initial"])
        .env("GIT_AUTHOR_DATE", "2025-01-15T10:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2025-01-15T10:00:00+00:00")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());

    dir
}

#[test]
fn analyze_outputs_json() {
    let dir = create_fixture_repo();

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "analyze",
        "--repo",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"repositories\""))
    .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn analyze_outputs_markdown() {
    let dir = create_fixture_repo();

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "analyze",
        "--repo",
        dir.path().to_str().unwrap(),
        "--format",
        "md",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Repolyze Analysis Report"));
}

#[test]
fn analyze_writes_to_file() {
    let dir = create_fixture_repo();
    let output_dir = tempfile::tempdir().unwrap();
    let output_file = output_dir.path().join("report.json");

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "analyze",
        "--repo",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
        "--output",
        output_file.to_str().unwrap(),
    ])
    .assert()
    .success();

    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"repositories\""));
}

#[test]
fn analyze_defaults_to_current_directory() {
    let dir = create_fixture_repo();

    // No --repo flag, run from the fixture repo directory
    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.current_dir(dir.path())
        .args(["analyze", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"repositories\""));
}

#[test]
fn analyze_with_directory_flag() {
    let dir = create_fixture_repo();

    // Use --directory instead of --repo
    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "--directory",
        dir.path().to_str().unwrap(),
        "analyze",
        "--format",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"repositories\""));
}

#[test]
fn analyze_with_short_directory_flag() {
    let dir = create_fixture_repo();

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "-D",
        dir.path().to_str().unwrap(),
        "analyze",
        "--format",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"repositories\""));
}

#[test]
fn directory_flag_with_invalid_path_fails() {
    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args(["--directory", "/nonexistent/path/xyz", "analyze"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot change to directory"));
}

#[test]
fn analyze_reuses_existing_database_on_second_run() {
    let repo = create_fixture_repo();

    let mut first = Command::cargo_bin("repolyze").unwrap();
    first
        .args([
            "analyze",
            "--repo",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();

    // In debug builds, the DB is next to the binary
    let bin_path = Command::cargo_bin("repolyze")
        .unwrap()
        .get_program()
        .to_owned();
    let db_path = std::path::Path::new(&bin_path)
        .parent()
        .unwrap()
        .join("repolyze-dev.db");
    assert!(db_path.exists(), "dev database should exist at {db_path:?}");

    let mut second = Command::cargo_bin("repolyze").unwrap();
    second
        .args([
            "analyze",
            "--repo",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();
}

fn create_fixture_repo_at(path: &std::path::Path) {
    std::fs::create_dir_all(path).unwrap();
    GitCommand::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    GitCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    GitCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    std::fs::write(path.join("README.md"), "# Test\n").unwrap();
    GitCommand::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    let output = GitCommand::new("git")
        .args(["commit", "-m", "initial"])
        .env("GIT_AUTHOR_DATE", "2025-01-13T10:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2025-01-13T10:00:00+00:00")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn create_workspace_with_two_repos() -> tempfile::TempDir {
    let workspace = tempfile::tempdir().unwrap();
    create_fixture_repo_at(&workspace.path().join("repo-a"));
    create_fixture_repo_at(&workspace.path().join("repo-b"));
    workspace
}

#[test]
fn analyze_users_contribution_discovers_repos_under_directory() {
    let workspace = create_workspace_with_two_repos();

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "analyze",
        "users-contribution",
        "--repo",
        workspace.path().to_str().unwrap(),
        "--format",
        "table",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Email"))
    .stdout(predicate::str::contains("Most active week day"));
}

#[test]
fn analyze_activity_outputs_ascii_table() {
    let repo = create_fixture_repo();

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "analyze",
        "activity",
        "--repo",
        repo.path().to_str().unwrap(),
        "--format",
        "table",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Avg commits/day (best day)"));
}

#[test]
fn analyze_users_contribution_defaults_to_table_format() {
    let workspace = create_workspace_with_two_repos();

    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.args([
        "analyze",
        "users-contribution",
        "--repo",
        workspace.path().to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Most active week day"));
}

#[test]
fn analyze_recomputes_when_worktree_is_dirty() {
    let repo = create_fixture_repo();

    let first = Command::cargo_bin("repolyze")
        .unwrap()
        .args([
            "analyze",
            "--repo",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        first_json["repositories"][0]["size"]["files"].as_u64(),
        Some(1)
    );

    std::fs::write(repo.path().join("NOTES.md"), "notes\n").unwrap();

    let second = Command::cargo_bin("repolyze")
        .unwrap()
        .args([
            "analyze",
            "--repo",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(
        second_json["repositories"][0]["size"]["files"].as_u64(),
        Some(2)
    );
}
