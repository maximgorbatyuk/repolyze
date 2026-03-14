use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "repolyze",
    about = "Repository analytics for local Git repositories",
    version
)]
pub struct Cli {
    /// Working directory (defaults to current directory)
    #[arg(long = "directory", short = 'D', global = true)]
    pub directory: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch the interactive TUI
    Tui,
    /// Analyze one or more repositories
    Analyze(AnalyzeArgs),
    /// Compare multiple repositories
    Compare(CompareArgs),
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    /// Repository path(s) to analyze (defaults to current directory)
    #[arg(long = "repo")]
    pub repos: Vec<PathBuf>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,

    /// Output file (stdout if omitted)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct CompareArgs {
    /// Repository path(s) to compare
    #[arg(long = "repo", required = true)]
    pub repos: Vec<PathBuf>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,

    /// Output file (stdout if omitted)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Md,
}
