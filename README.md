# Repolyze

`repolyze` is a Rust-based repository analysis tool for one or more already-cloned local Git repositories.
It is designed as a TUI-first application with optional CLI commands for users who want scriptable analysis without the interactive interface.

The initial product scope focuses on:

- user contribution statistics from Git history
- most active hours and days based on commit activity
- language-agnostic repository size comparison
- JSON and Markdown report output

## Installation

```bash
brew tap maximgorbatyuk/tap
brew install repolyze

# Check installation
repolyze -V
```

## Project Docs

- Development guide: [`docs/development.md`](docs/development.md)
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
