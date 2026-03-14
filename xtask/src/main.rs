use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Developer automation for repolyze")]
struct Cli {
    #[command(subcommand)]
    command: XtaskCommand,
}

#[derive(Subcommand)]
enum XtaskCommand {
    /// Run all verification checks (fmt, clippy, test, check)
    Verify,
    /// Prepare a release
    Release(ReleaseArgs),
}

#[derive(clap::Args)]
struct ReleaseArgs {
    /// Version to release (e.g. 0.1.0)
    #[arg(long)]
    version: String,

    /// Dry-run mode: validate without creating tags
    #[arg(long)]
    dry_run: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        XtaskCommand::Verify => run_verify(),
        XtaskCommand::Release(args) => run_release(&args),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_verify() -> Result<(), String> {
    let steps: &[(&str, &[&str])] = &[
        ("Format check", &["cargo", "fmt", "--all", "--check"]),
        (
            "Clippy",
            &[
                "cargo",
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("Test", &["cargo", "test", "--workspace"]),
        ("Check", &["cargo", "check", "--workspace", "--all-targets"]),
    ];

    for (name, cmd) in steps {
        eprintln!("── {name} ──");
        let status = Command::new(cmd[0])
            .args(&cmd[1..])
            .status()
            .map_err(|e| format!("failed to run {}: {e}", cmd[0]))?;

        if !status.success() {
            return Err(format!("{name} failed"));
        }
    }

    eprintln!("── All checks passed ──");
    Ok(())
}

fn run_release(args: &ReleaseArgs) -> Result<(), String> {
    eprintln!("Preparing release v{}...", args.version);

    // Always run verification first
    run_verify()?;

    if args.dry_run {
        eprintln!("Dry-run mode: skipping tag creation");
        eprintln!("Would create tag: v{}", args.version);
        return Ok(());
    }

    // Check for clean working tree
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("git status failed: {e}"))?;

    let status_text = String::from_utf8_lossy(&output.stdout);
    if !status_text.trim().is_empty() {
        return Err("working tree is not clean — commit or stash changes first".to_string());
    }

    eprintln!("Working tree is clean.");
    eprintln!(
        "Ready to tag v{}. Run:\n  git tag v{}\n  git push origin main\n  git push origin v{}",
        args.version, args.version, args.version
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xtask_parses_verify() {
        let cli = Cli::try_parse_from(["xtask", "verify"]);
        assert!(cli.is_ok());
        assert!(matches!(cli.unwrap().command, XtaskCommand::Verify));
    }

    #[test]
    fn xtask_parses_release_args() {
        let cli = Cli::try_parse_from(["xtask", "release", "--dry-run", "--version", "0.1.0"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            XtaskCommand::Release(args) => {
                assert_eq!(args.version, "0.1.0");
                assert!(args.dry_run);
            }
            _ => panic!("expected Release command"),
        }
    }

    #[test]
    fn xtask_release_requires_version() {
        let cli = Cli::try_parse_from(["xtask", "release"]);
        assert!(cli.is_err());
    }
}
