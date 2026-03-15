# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-03-15

### Added

- SQLite analytics cache (`repolyze-store` crate) — repeat analysis on unchanged repos is instant
- Contributor analytics views: `analyze users-contribution` and `analyze activity` with `--format table`
- `analyze all` (default) shows both contribution and activity tables in TUI
- Per-contributor activity facts: weekday/hour commit counts, active date buckets
- Cross-repository contributor merging by lowercased email with date deduplication
- Plain-text table output with right-aligned numbers, dash separators, and totals row
- Analysis header showing period, project count, and elapsed time
- Recursive directory discovery — pass a parent folder and nested repos are found automatically
- TUI Analyze submenu: All / Users contribution / Most active days and hours
- TUI Metadata screen showing database path, file size, and table row counts
- Ctrl+C graceful quit from any TUI screen
- Worktree dirty detection — dirty repos bypass cache for fresh analysis
- Scan run history tracking (hit/miss/bypass/success/failure)
- Dev/release database path separation (`target/debug/repolyze-dev.db` vs `~/.repolyze/repolyze.db`)
- `scripts/release.py` for automated version bump, merge, and tag workflow
- Project website with feature cards, usage examples, and live output demo

### Changed

- TUI Analyze screen shows results directly instead of path entry form
- Errors menu item replaced with Metadata screen
- Help screen updated to reflect current screens and keybindings
- Compare screen key hints show Ctrl+C instead of misleading Q for quit
- Static screens (Help, Metadata) no longer respond to arrow keys

### Fixed

- Saturating arithmetic for `net_lines`, `lines_modified`, and weekday/hour commit merging
- `most_active_index` returns N/A for contributors with zero activity instead of "Monday"
- `load_snapshot` distinguishes `QueryReturnedNoRows` from real database errors
- Transaction rollback on COMMIT failure in `upsert_commit` and `save_snapshot`
- Store errors logged instead of silently discarded
- Stale analysis state cleared when navigating home
- Status message reset between analysis runs
- Empty `snapshot_ids` slice guarded against invalid SQL
- View/format validation runs before expensive analysis in CLI
- `HeadMetadata.branch_name` uses `Option<String>` instead of empty-string sentinel

### Infrastructure

- WAL mode enabled for concurrent read-write safety
- Migrations restructured as ordered list for future V2 support
- `ON DELETE CASCADE` on all foreign keys
- `open_default_store` eliminates duplicated store-opening code across CLI and TUI
- `StoreError::Io` variant added for path-related failures

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
