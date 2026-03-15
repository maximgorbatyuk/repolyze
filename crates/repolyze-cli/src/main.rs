mod args;
mod run;

use std::fs;
use std::path::PathBuf;

use clap::Parser;

use args::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(dir) = &cli.directory {
        std::env::set_current_dir(dir)
            .map_err(|e| anyhow::anyhow!("cannot change to directory '{}': {e}", dir.display()))?;
    }

    match cli.command {
        Some(Commands::Tui) | None => repolyze_tui::run()?,
        Some(Commands::Analyze(args)) => {
            let repos = default_repos(args.repos);
            let format = args
                .format
                .unwrap_or_else(|| crate::args::OutputFormat::default_for_view(&args.view));
            let output = run::run_analyze(&repos, &args.view, &format)?;
            write_output(&output, args.output.as_deref())?;
        }
        Some(Commands::Compare(args)) => {
            let format: crate::args::OutputFormat = args.format.into();
            let output = run::run_analyze(&args.repos, &crate::args::AnalyzeView::All, &format)?;
            write_output(&output, args.output.as_deref())?;
        }
    }

    Ok(())
}

/// If no --repo flags were given, default to the current directory.
fn default_repos(repos: Vec<PathBuf>) -> Vec<PathBuf> {
    if repos.is_empty() {
        vec![PathBuf::from(".")]
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
