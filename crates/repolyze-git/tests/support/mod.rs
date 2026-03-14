use std::path::Path;
use std::process::Command;

pub struct CommitSpec {
    pub author_name: &'static str,
    pub author_email: &'static str,
    pub authored_at: &'static str,
    pub message: &'static str,
    pub rel_path: &'static str,
    pub contents: &'static str,
}

pub fn create_fixture_repo(specs: &[CommitSpec]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();

    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.name", "Default"]);
    run_git(repo, &["config", "user.email", "default@test.com"]);

    for spec in specs {
        let file_path = repo.join(spec.rel_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, spec.contents).unwrap();

        run_git(repo, &["add", spec.rel_path]);

        let output = Command::new("git")
            .args(["commit", "-m", spec.message])
            .env("GIT_AUTHOR_NAME", spec.author_name)
            .env("GIT_AUTHOR_EMAIL", spec.author_email)
            .env("GIT_AUTHOR_DATE", spec.authored_at)
            .env("GIT_COMMITTER_NAME", spec.author_name)
            .env("GIT_COMMITTER_EMAIL", spec.author_email)
            .env("GIT_COMMITTER_DATE", spec.authored_at)
            .current_dir(repo)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    dir
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}
