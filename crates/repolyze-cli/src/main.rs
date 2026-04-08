mod args;
mod run;

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use repolyze_core::settings::Settings;

use args::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(dir) = &cli.directory {
        std::env::set_current_dir(dir)
            .map_err(|e| anyhow::anyhow!("cannot change to directory '{}': {e}", dir.display()))?;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let settings = Settings::ensure_and_load(&cwd);

    match cli.command {
        Some(Commands::Tui(args)) => {
            let initial = initial_repos(args.repos, cli.repos);
            repolyze_tui::run(initial, &settings)?;
        }
        None => {
            let initial = initial_repos(vec![], cli.repos);
            repolyze_tui::run(initial, &settings)?;
        }
        Some(Commands::Analyze(args)) => {
            let repos = default_repos(args.repos);
            let format = args
                .format
                .unwrap_or_else(|| crate::args::OutputFormat::default_for_view(&args.view));
            let output = run::run_analyze(
                &repos,
                &args.view,
                &format,
                args.email.as_deref(),
                &settings,
            )?;
            write_output(&output, args.output.as_deref())?;
        }
        Some(Commands::Compare(args)) => {
            let format: crate::args::OutputFormat = args.format.into();
            let repos = default_repos(args.repos);
            let output = run::run_analyze(
                &repos,
                &crate::args::AnalyzeView::All,
                &format,
                None,
                &settings,
            )?;
            write_output(&output, args.output.as_deref())?;
        }
    }

    Ok(())
}

/// Merge subcommand-level and global `--repo` flags into an optional initial set for the TUI.
fn initial_repos(subcommand_repos: Vec<String>, global_repos: Vec<String>) -> Option<Vec<String>> {
    let repos = if subcommand_repos.is_empty() {
        global_repos
    } else {
        subcommand_repos
    };
    if repos.is_empty() { None } else { Some(repos) }
}

/// If no --repo flags were given, default to the current directory.
fn default_repos(repos: Vec<String>) -> Vec<String> {
    if repos.is_empty() {
        vec![".".to_string()]
    } else {
        repos
    }
}

fn write_output(content: &str, path: Option<&std::path::Path>) -> anyhow::Result<()> {
    match path {
        Some(p) => {
            fs::write(p, content)?;
            eprintln!("Report written to {}", p.display());
        }
        None => print!("{content}"),
    }
    Ok(())
}
