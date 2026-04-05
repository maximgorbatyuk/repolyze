# Repolyze

`repolyze` is a Rust-based repository analysis tool for one or more already-cloned local Git repositories.
It is designed as a TUI-first application with optional CLI commands for users who want scriptable analysis without the interactive interface.

## Features

### Contribution Statistics

Per-contributor breakdown across one or many repositories:

- Total commits, lines added, lines deleted, net lines changed
- Average lines modified per commit
- Distinct files touched and file extension breakdown
- Active days count
- Sorted by total commits (most active contributor first), with a totals row

### Activity Patterns

Shows when each contributor is most active:

- Most active weekday and hour of day per contributor
- Average commits per active day (overall and on the best weekday)
- Average commits per active hour (overall and on the best hour)

### Activity Heatmap

GitHub-style 52-week commit grid rendered in the terminal:

- One cell per day, rows for weekdays (Mon–Sun), columns for weeks
- ASCII intensity levels: `·` (no commits), `░` `▒` `▓` `█` (increasing density)
- Month labels along the top for orientation

### User Effort Deep-Dive

Detailed profile for a single contributor (selected by email):

- Date range of activity (first and latest commit)
- Most and least active weekday with average commits per day
- Average commits per day, files per commit, files per day
- Average lines modified per commit and per day
- Top 3 file extensions by touch count

### Repository Comparison

Side-by-side analysis when two or more repositories are provided:

- Per-repo commit count, active days, and commits per active day
- Most active and least active repositories ranked by commits per day
- Per-weekday rankings showing which repo is most active on each day of the week

### Repository Size Metrics

Language-agnostic codebase size snapshot (respects `.gitignore`):

- File, directory, and byte counts
- Total, non-empty, and blank line counts
- Breakdown by file extension
- Top 10 largest files with path, size, and line count

### Git Tools

Interactive branch cleanup from the TUI:

- **Remove merged branches** — lists branches already merged into a target branch, confirms before deleting
- **Remove stale branches** — lists branches with no activity for a configurable number of days, confirms before deleting
- Protected-branch guard prevents accidental deletion of main/dev branches

### Output Formats

- **JSON** — full structured report for scripting and CI/CD pipelines
- **Markdown** — human-readable report with tables, heatmap, and summary sections
- **Plain-text table** — compact ASCII tables for terminal output (contribution, activity, user effort, comparison views)

### TUI and CLI

- **TUI** — full-screen interactive interface with menu navigation, background analysis with animated spinner, and scrollable results
- **CLI** — scriptable subcommands (`analyze`, `compare`) with format and output file options

## Who is Repolyze for?

- **Team Leads** — Track team member contributions and effort across repositories. See who's active, how workload is distributed, and where the team's energy goes.
- **Tech Leads** — Get actionable insights across company repositories. Compare codebases side by side, spot activity trends, and make data-driven decisions about your engineering portfolio.
- **Individual Contributors** — Understand your own contribution patterns. Track your output over time, see where your effort goes, and benchmark your activity across projects.

## Installation

```bash
brew tap maximgorbatyuk/tap
brew install repolyze

# Check installation
repolyze -V
```

## Project Docs

- Development guide: [`docs/development.md`](docs/development.md)
- Features and known issues: [`FEATURES.md`](FEATURES.md)
- Privacy policy: [`PRIVACY-POLICY.md`](PRIVACY-POLICY.md)
- Release guide: [`docs/release.md`](docs/release.md)
- Homebrew guide: [`docs/homebrew.md`](docs/homebrew.md)

## Tech Stack

- Rust
- `clap`
- `ratatui`
- `crossterm`
- `serde` / `serde_json`
- `ignore`
- `cargo-dist`
- GitHub Actions
- Homebrew for macOS distribution
