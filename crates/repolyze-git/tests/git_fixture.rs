mod support;

use std::process::Command;

use support::{CommitSpec, create_fixture_repo};

#[test]
fn git_fixture_creates_deterministic_history() {
    let specs = &[
        CommitSpec {
            author_name: "Alice",
            author_email: "alice@example.com",
            authored_at: "2025-01-15T10:00:00+00:00",
            message: "initial commit",
            rel_path: "README.md",
            contents: "# Test Repo\n",
        },
        CommitSpec {
            author_name: "Bob",
            author_email: "bob@example.com",
            authored_at: "2025-01-16T14:30:00+00:00",
            message: "add source file",
            rel_path: "src/lib.rs",
            contents: "pub fn hello() {}\n",
        },
        CommitSpec {
            author_name: "Alice",
            author_email: "alice@example.com",
            authored_at: "2025-01-17T09:15:00+00:00",
            message: "update readme",
            rel_path: "README.md",
            contents: "# Test Repo\n\nUpdated.\n",
        },
    ];

    let dir = create_fixture_repo(specs);

    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let count: u32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap();

    assert_eq!(count, 3, "expected 3 commits in fixture repo");
}
