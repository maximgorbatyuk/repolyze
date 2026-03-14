# repolyze

`repolyze` is a Rust-based repository analysis tool for one or more already-cloned local Git repositories.
It is designed as a TUI-first application with optional CLI commands for users who want scriptable analysis without the interactive interface.

The initial product scope focuses on:

- user contribution statistics from Git history
- most active hours and days based on commit activity
- language-agnostic repository size comparison
- JSON and Markdown report output

## Planning Docs

- Design: [`docs/plans/2026-03-14-repolyze-design.md`](docs/plans/2026-03-14-repolyze-design.md)
- Implementation plan: [`docs/plans/2026-03-14-repolyze-implementation-plan.md`](docs/plans/2026-03-14-repolyze-implementation-plan.md)

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
