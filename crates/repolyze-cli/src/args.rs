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
    /// Analysis view
    #[arg(value_enum, default_value = "all")]
    pub view: AnalyzeView,

    /// Repository path(s) to analyze (defaults to current directory)
    #[arg(long = "repo")]
    pub repos: Vec<PathBuf>,

    /// Output format
    #[arg(long)]
    pub format: Option<OutputFormat>,

    /// Output file (stdout if omitted)
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Contributor email (required for user-effort view)
    #[arg(long)]
    pub email: Option<String>,
}

#[derive(clap::Args)]
pub struct CompareArgs {
    /// Repository path(s) to compare
    #[arg(long = "repo", required = true)]
    pub repos: Vec<PathBuf>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: CompareOutputFormat,

    /// Output file (stdout if omitted)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum AnalyzeView {
    /// Full analysis (JSON or Markdown)
    All,
    /// Per-contributor commit and line statistics (RF-8)
    Contribution,
    /// Most active days and hours per contributor (RF-9)
    Activity,
    /// Detailed per-contributor deep-dive
    UserEffort,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Md,
    Table,
}

impl OutputFormat {
    pub fn default_for_view(view: &AnalyzeView) -> Self {
        match view {
            AnalyzeView::All => Self::Json,
            AnalyzeView::Contribution | AnalyzeView::Activity | AnalyzeView::UserEffort => {
                Self::Table
            }
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CompareOutputFormat {
    Json,
    Md,
}

impl From<CompareOutputFormat> for OutputFormat {
    fn from(value: CompareOutputFormat) -> Self {
        match value {
            CompareOutputFormat::Json => Self::Json,
            CompareOutputFormat::Md => Self::Md,
        }
    }
}
