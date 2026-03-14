use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_cli_help() {
    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("repolyze"));
}
