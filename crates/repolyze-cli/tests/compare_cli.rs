use std::process::Command as GitCommand;

use assert_cmd::Command;
use predicates::prelude::*;

fn create_fixture_repo(name: &str) -> tempfile::TempDir {
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

    std::fs::write(root.join("README.md"), format!("# {name}\n")).unwrap();

    GitCommand::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()
        .unwrap();

    let output = GitCommand::new("git")
        .args(["commit", "-m", &format!("init {name}")])
        .env("GIT_AUTHOR_DATE", "2025-01-15T10:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2025-01-15T10:00:00+00:00")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());

    dir
}

fn isolated_db() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn repolyze_cmd(db_dir: &tempfile::TempDir) -> Command {
    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.env(
        "REPOLYZE_DB_PATH",
        db_dir.path().join("test.db").to_str().unwrap(),
    );
    cmd
}

#[test]
fn compare_outputs_markdown() {
    let repo_a = create_fixture_repo("repo-a");
    let repo_b = create_fixture_repo("repo-b");
    let db = isolated_db();

    repolyze_cmd(&db)
        .args([
            "compare",
            "--repo",
            repo_a.path().to_str().unwrap(),
            "--repo",
            repo_b.path().to_str().unwrap(),
            "--format",
            "md",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Repolyze Analysis Report"))
        .stdout(predicate::str::contains("**2** repositories"));
}

#[test]
fn compare_outputs_json_with_multiple_repos() {
    let repo_a = create_fixture_repo("repo-a");
    let repo_b = create_fixture_repo("repo-b");
    let db = isolated_db();

    repolyze_cmd(&db)
        .args([
            "compare",
            "--repo",
            repo_a.path().to_str().unwrap(),
            "--repo",
            repo_b.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"repositories\""));

    let output = repolyze_cmd(&db)
        .args([
            "compare",
            "--repo",
            repo_a.path().to_str().unwrap(),
            "--repo",
            repo_b.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(json["repositories"].as_array().unwrap().len(), 2);
}

#[test]
fn compare_reports_invalid_repo_as_failure() {
    let repo = create_fixture_repo("repo-a");
    let db = isolated_db();

    let output = repolyze_cmd(&db)
        .args([
            "compare",
            "--repo",
            repo.path().to_str().unwrap(),
            "--repo",
            "/nonexistent/path",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(json["repositories"].as_array().unwrap().len(), 1);
    assert_eq!(json["failures"].as_array().unwrap().len(), 1);
    assert!(
        json["failures"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("path does not exist")
    );
}

#[test]
fn compare_help_does_not_advertise_table_format() {
    let output = Command::cargo_bin("repolyze")
        .unwrap()
        .args(["compare", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("json"));
    assert!(stdout.contains("md"));
    assert!(!stdout.contains("table"));
}
