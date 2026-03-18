# Repolyze

`repolyze` is a Rust-based repository analysis tool for one or more already-cloned local Git repositories.
It is designed as a TUI-first application with optional CLI commands for users who want scriptable analysis without the interactive interface.

The initial product scope focuses on:

- user contribution statistics from Git history
- most active hours and days based on commit activity
- GitHub-style activity heatmap showing daily commits over the past year
- multi-repository comparison with per-repo and per-weekday rankings
- language-agnostic repository size comparison
- scrollable full-screen TUI with background analysis and animated spinner
- JSON and Markdown report output

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
