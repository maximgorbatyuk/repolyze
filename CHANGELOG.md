# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.9] - 2026-04-07

### Added

- **Multi-repo selection in Git Tools**: The repo picker now supports selecting multiple repositories at once using checkboxes. A "Select all" row toggles all repos. Press Space to toggle individual repos or the select-all row, then Enter to confirm. Branch listing and deletion operate across all selected repos simultaneously.
- **Repo prefix in multi-repo branch views**: When multiple repos are selected, branch names are prefixed with `[repo-name]` in the branch list and deletion progress screens for disambiguation.

### Fixed

- **TUI crash on empty repo selection**: Replaced `assert!` panics (which would crash the TUI and leave the terminal in raw mode) with graceful error messages when no repos are selected.
- **Single failed repo aborted entire scan**: Branch listing across multiple repos now tolerates partial failures — branches from successful repos are still shown. Errors are only reported when all repos fail and no branches are found.
- **Single-repo workspace shown as unchecked**: When a workspace contains exactly one repo, the picker now shows it as checked, consistent with it being auto-selected.
- **Stale cursor position after tool reset**: `clear_tool` now resets the repo picker cursor to the top, preventing a stale position when re-entering the picker.

### Changed

- **`delete_branch` API simplified**: `delete_branch` no longer takes a separate `repo` path argument. The repo is now carried by `BranchInfo::repo`, making the API self-contained.
- **Repo display name deduplicated**: Extracted `BranchInfo::repo_display_name()` and `path_display_name()` helpers, replacing 6 inline occurrences of the same path-basename logic.

## [0.1.8] - 2026-04-05

### Added

- **User alias settings**: Repolyze now supports a per-project settings file at `.repolyze/settings.json`. The `users` object maps a display name to a list of email addresses, allowing contributions from multiple git identities to be grouped under a single person. When aliases are configured, all tables, reports, heatmaps, and the TUI contributor picker show the configured name instead of the email and merge stats across all mapped emails.
- **Auto-create settings file**: On startup, repolyze creates `.repolyze/settings.json` with an empty JSON object (`{}`) if the file does not exist. Existing files are never overwritten.

### Changed

- **"Email" column renamed to "Author"**: Contribution and activity tables (plain-text and Markdown) now use "Author" as the column header, which works for both email addresses and configured display names.
- **Markdown Top Contributors table simplified**: The separate "Name" and "Email" columns have been merged into a single "Author" column. Without settings, the git author name is shown; with settings, the configured display name is used.

## [0.1.7] - 2026-04-03

### Added

- **TUI Markdown export**: Press `e` on the Analyze results screen to export the current report as a Markdown file. The file is written to the current directory with a timestamped name (`repolyze-report-YYYY-MM-DD-HHMMSS.md`) to avoid overwriting previous exports. The `e Export` hint appears in the footer once the report is loaded; status bar confirms the full output path or shows an error.
- **Help screen documents Analyze keybindings**: New "Analyze results" section lists `e` (Export), `j/↓` (Scroll down), and `k/↑` (Scroll up).

### Fixed

- **Compare Repositories section broken in Markdown output**: The "Compare Repositories" section in `--format md` and TUI export was rendered using the TUI plain-text table format (dash separators, no pipes), which does not display as tables in Markdown viewers. Now renders proper `| pipe |` Markdown tables with `###` sub-headings for all three comparison sub-sections (most active, least active, by weekday).

## [0.1.6] - 2026-03-22

### Fixed

- **TUI shortcuts on non-Latin keyboards**: Pressing Q, J, K, Y, N on a non-Latin keyboard layout (e.g. Russian ЙЦУКЕН) now correctly triggers quit, scroll, and confirm/cancel shortcuts. A QWERTY normalization layer translates physical key characters before matching, while text-input screens (contributor filter, branch name input) remain unaffected.

### Changed

- **Git Tools branch list hint**: Added an instructional hint above the branch list on the confirmation screen: "Review the branches below, then press y/Enter to delete or n/Esc to cancel."

## [0.1.5] - 2026-03-21

### Added

- **Windows support**: Build target `x86_64-pc-windows-msvc` with MSI installer and PowerShell install script. Release workflow now produces Windows `.zip`, `.msi`, and `.ps1` artifacts alongside macOS and Linux builds.
- **Windows CI job**: Build and test on `windows-latest` runner on every push/PR.
- **Website SEO**: Added `robots.txt`, `sitemap.xml`, `llms.txt`, and CNAME for custom domain.

### Fixed

- **TUI duplicate key events on Windows**: Added `KeyEventKind::Press` filter to prevent crossterm from firing handlers twice (Press + Release) on Windows.
- **Database path resolution on Windows**: Replaced `HOME` env var (Unix-only) with the `home` crate for cross-platform home directory lookup.

## [0.1.4] - 2026-03-18

### Added

- **Git Tools menu**: New top-level TUI screen with two branch-cleanup tools — "Remove merged branches" (deletes local+remote branches already merged into a chosen base branch) and "Remove stale branches" (deletes branches with no activity for N days). Includes repo picker for multi-repo workspaces, branch listing with local/remote indicators, confirmation before deletion, and progress screen with per-branch success/failure status. Protected branches (`main`, `master`, `dev`, `develop`, `production`, etc.) are never deleted.
- **User effort deep-dive**: New `analyze user-effort --email <email>` CLI subcommand and "User effort" TUI view. Shows per-contributor metrics: first/last commit dates, most/least active weekday with commits-per-day, average commits/files/lines per day and per commit, and top 3 file extensions by touch count. TUI includes a filterable contributor picker with type-to-search.
- **File extension tracking**: `ContributorStats` now records a `file_extensions` map (`BTreeMap<String, u64>`) counting files touched per extension. Populated during Git log parsing and merged across repositories.
- **Contributor picker screen**: TUI `UserSelect` screen lets users search/filter contributors by email or name, then select one for the user effort view. Supports keyboard navigation and scroll clamping.

### Changed

- **Renamed `UsersContribution` → `Contribution`**: CLI view enum, model type, analytics builder, table renderer, and all constants renamed from `UsersContribution`/`users_contribution` to `Contribution`/`contribution` for brevity. CLI subcommand is now `analyze contribution`.
- **Schema version bumped to 3**: Invalidates cached snapshots from v0.1.3 to pick up the new `file_extensions` field.
- **Analysis elapsed time stored**: `AppState` now tracks `analysis_elapsed` for use in TUI report headers.
- **Website redesign**: `docs/index.html` refactored with external `style.css`, new OG image (`og.svg`), and updated layout.

### Infrastructure

- `repolyze-git::branches` module: `list_merged_branches`, `list_stale_branches`, `delete_branches` functions with protected-branch guard and origin-only remote support
- `BranchInfo` and `DeleteResult` types for branch listing and deletion reporting
- `GitToolsState` and `GitToolsMode` in TUI app state with menu/input/progress screen management
- `UserEffortData` model type with `Display` impl
- `get_contributor_emails()` builder in analytics for populating the contributor picker
- `render_user_effort_table()` in report crate using key-value plain table format
- `MergedContributor` extended with `name`, `file_extensions`, `first_commit`, `last_commit` fields for cross-repo merging

## [0.1.3] - 2026-03-17

### Added

- **Activity heatmap**: GitHub-style contribution grid showing daily commit activity over the past 52 weeks. Available as a standalone TUI view, included in the Full report, and rendered in Markdown output using Unicode block characters (`·░▒▓█`). Color-coded legend shows concrete commit-count ranges computed from the data.
- **Repository comparison report**: New "Compare repositories" view (5th TUI menu item, shown only for multi-repo workspaces). Renders three tables: top 3 most active repos by commits/day, top 3 least active, and per-weekday top 3 rankings. Included in Full report as section #4 when analyzing multiple repos.
- **`commits_by_date` tracking**: New `BTreeMap<String, u32>` field on `ContributorActivityStats` counting commits per calendar date. Drives the heatmap grid and is serialized into the SQLite cache payload.
- **`date_util` module** (`repolyze-core`): Date arithmetic without chrono — `parse_ymd`, `day_of_week` (Sakamoto), `add_days` (Julian Day Number), `format_ymd`, `month_abbrev`, `today_ymd`, `to_jdn`.
- **Dynamic loading spinner**: TUI analysis runs on a background thread with animated braille spinner (`⠁⠉⠙⠸⠰⠴⠦⠇`). Event loop uses non-blocking `poll(100ms)` instead of blocking `read_event()`, keeping the UI responsive during analysis.
- **Scrollable report view**: Analyze screen supports `j/k`/arrow key scrolling. `Paragraph` renders without wrapping for exact line-count height, fixing clipped content on long reports.
- **Workspace probe on Analyze menu**: Shows current folder path and mode (single repository / multi-repository with repo count) before the view selection menu.
- **Report headers with folder and mode**: Analysis header now includes `Folder:` and `Mode:` lines between `Projects:` and `Elapsed:`.
- **Section headers and descriptions**: Each report section (Users contribution, Activity, Heatmap, Compare) includes a title and one-line description. Full report uses numbered headers (`#1`, `#2`, `#3`, `#4`).
- **Heatmap period display**: Shows `YYYY-MM-DD .. YYYY-MM-DD` range above the heatmap grid in both TUI and Markdown.

### Changed

- **Activity table column abbreviations**: Headers shortened from `Avg commits/day (best day)` to `C/D (best)`, etc. Legend block explains each abbreviation. Reduces table width from ~120 to ~65 chars.
- **"Most active week day" removed from Users contribution table**: Column dropped from model, renderer, store record, and all tests. The data remains available in the Activity table.
- **Menu renamed**: "All (full report)" renamed to "Full report".
- **Schema version bumped to 2**: Invalidates cached snapshots from v0.1.2, forcing re-analysis to populate `commits_by_date`.
- **Compare repositories table format**: Uses project-standard `render_plain_table` with dash separators and right-aligned numbers instead of ad-hoc indented lists.

### Fixed

- **TUI wrapping caused clipped heatmap rows**: Removed `Paragraph::wrap()` from Analyze screen. Fixed-width content (tables, heatmap) now clips at terminal edge instead of wrapping into garbled multi-line rows.
- **Scroll clamping was byte-based**: Previous height estimation counted UTF-8 bytes instead of display columns, causing scroll to stop before the bottom of the report.
- **Store open failure path**: `compute_analysis` returns a proper error message instead of encoding error state as a `failure_count` hack.

### Infrastructure

- `HeatmapData` type with `legend_labels()` method for computing commit-count ranges from `max_count`
- `RepoComparisonRow` type and `build_repo_comparison()` builder in `repolyze-core::analytics`
- `render_repo_comparison_table()` and `render_heatmap_section()` in `repolyze-report`
- `WorkspaceInfo` struct and `ProbeWorkspace` action in TUI app state
- `AnalysisCompletion` struct for background thread communication via `mpsc::channel`
- Constants `HEATMAP_MAX_WEEKS`, `DAYS_IN_WEEK` replace magic numbers in grid dimensions
- `to_jdn` deduplicated from analytics into `date_util` as single public function

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
