mod args;
mod run;

use std::fs;

use clap::Parser;

use args::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) | None => repolyze_tui::run()?,
        Some(Commands::Analyze(args)) => {
            let output = run::run_analyze(&args.repos, &args.format)?;
            write_output(&output, args.output.as_deref())?;
        }
        Some(Commands::Compare(args)) => {
            let output = run::run_analyze(&args.repos, &args.format)?;
            write_output(&output, args.output.as_deref())?;
        }
    }

    Ok(())
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
