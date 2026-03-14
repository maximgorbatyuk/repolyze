# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-14

### Added

- Full-screen TUI with Help, Analyze, Compare, and Errors screens
- `repolyze analyze` CLI command with `--repo`, `--format`, and `--output` flags
- `repolyze compare` CLI command for multi-repository comparison
- Git contribution statistics (commits, lines added/deleted, files touched, active days)
- Activity-by-hour and activity-by-day summaries with heatmap matrix
- Language-agnostic repository size metrics (.gitignore-aware)
- JSON report export
- Markdown report export
- Input resolution with path canonicalization and deduplication
- Partial failure tolerance for batch analysis
- `cargo xtask verify` for fmt + clippy + test + check
- `cargo xtask release` with `--dry-run` and `--version` flags
- CI workflow for PRs to main
- Release workflow via cargo-dist (macOS aarch64/x86_64, Linux x86_64)
- Shell installer and Homebrew formula generation
