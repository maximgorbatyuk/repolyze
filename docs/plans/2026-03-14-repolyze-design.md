# Repolyze v1 Design

## Goal

Build a Rust CLI application named `repolyze` for analyzing one or more already-cloned local Git repositories. The primary UX is a full-screen TUI, but the same capabilities must also be exposed as CLI commands for automation and scripting.

## Scope

### In scope for v1

- Local filesystem analysis only
- One or more Git repositories as input
- Contribution statistics derived from Git history
- Most active hours and days derived from Git commit timestamps
- Language-agnostic repository size metrics
- Cross-repository comparison
- JSON export
- Markdown report export
- macOS and Linux release artifacts
- Homebrew distribution for macOS

### Explicitly out of scope for v1

- Remote GitHub/GitLab repository analysis
- Language-aware parsing for classes, functions, or logic blocks
- Windows release artifacts
- Hosted backend or database
- Plugin system
- Alias mapping UI for multiple author emails

## Product Shape

The shipped binary is `repolyze`.

- `repolyze` with no arguments launches the TUI
- `repolyze tui` explicitly launches the TUI
- `repolyze analyze ...` performs non-interactive analysis
- `repolyze compare ...` performs non-interactive multi-repository comparison
- `repolyze help` prints CLI help

The initial project template should open into a full-screen TUI that contains only a single menu item: `Help`. That template is the first milestone, not the final state of the product. Later milestones can add `Analyze`, `Compare`, `Exports`, and `Errors` screens once the analysis engine exists.

## Architecture

The project should be a Rust workspace with one distributable binary and several focused library crates. Domain logic must live outside the TUI and CLI so both entrypoints call the same services and return the same result types.

### Recommended workspace layout

- `crates/repolyze-core`
- `crates/repolyze-git`
- `crates/repolyze-metrics`
- `crates/repolyze-report`
- `crates/repolyze-tui`
- `crates/repolyze-cli`
- `xtask`
- `tests/fixtures/`
- `docs/plans/`
- `.github/workflows/`

### Crate responsibilities

#### `repolyze-core`

- Shared domain types
- App service interfaces
- Input validation and path resolution
- Error types
- Configuration loading

#### `repolyze-git`

- Git subprocess backend
- Parsing commit history and `--numstat` output
- Contribution stats
- Activity-by-hour and activity-by-day summaries

#### `repolyze-metrics`

- `.gitignore`-aware repository walking
- File and directory counts
- Byte counts
- Total, non-empty, and blank line counts
- Extension breakdowns
- Largest-file summaries

#### `repolyze-report`

- JSON rendering
- Markdown report rendering
- Cross-repository comparison formatting

#### `repolyze-tui`

- App state
- Event loop
- Rendering
- Input handling
- Screen transitions

#### `repolyze-cli`

- `clap` command parsing
- Binary entrypoint
- TUI launch
- `analyze` and `compare` subcommands

#### `xtask`

- Developer automation
- Verification orchestration
- Fixture generation helpers if needed
- Release guardrails

## Data Flow

### Analysis pipeline

1. `InputResolver`
   - Accepts one or more local paths
   - Canonicalizes and deduplicates paths
   - Verifies Git repository validity
   - Returns typed `RepositoryTarget` values
2. `GitAnalyzer`
   - Runs `git` subprocesses through a trait-backed backend
   - Produces contribution and activity summaries
3. `RepoMetricsAnalyzer`
   - Walks each repository with `.gitignore` awareness
   - Produces language-agnostic size metrics
4. `Aggregator`
   - Builds per-repository and cross-repository summaries
5. `ReportRenderer`
   - Produces terminal tables, JSON exports, and Markdown reports

### Core domain types

- `AnalysisRequest`
- `RepositoryTarget`
- `RepositoryAnalysis`
- `ContributionSummary`
- `ContributorStats`
- `ActivitySummary`
- `SizeMetrics`
- `ComparisonReport`
- `ExportBundle`
- `PartialFailure`

## TUI Design

### Initial template

- Full-screen terminal app
- Left-side menu
- Main content panel
- Bottom status bar
- Menu contains only `Help`

### Keybindings

- `q` quits
- Up/Down and `j`/`k` move selection
- `Enter` activates selection
- `?` opens help

### Future screens after the template milestone

- `Home`
- `Help`
- `Analyze`
- `Compare`
- `Exports`
- `Errors`

The TUI must remain a thin presentation layer. Widgets do not compute Git or filesystem metrics directly.

## CLI Design

### Initial commands

- `repolyze`
- `repolyze tui`
- `repolyze help`
- `repolyze analyze --repo <path>... --format json|md --output <file>`
- `repolyze compare --repo <path>... --format json|md --output <file>`

### Behavior rules

- No-arg startup opens the TUI
- JSON output is stable and machine-friendly
- Markdown output is readable without the TUI
- Multi-repository runs tolerate partial failures by default

## Metrics Model

### Git-derived contributor metrics

- Commit count
- Lines added
- Lines deleted
- Net lines changed
- Files touched
- Active days
- First contribution date
- Last contribution date

### Time-activity metrics

- Commits by hour of day
- Commits by day of week
- Heatmap-ready day/hour matrix

### Language-agnostic repository metrics

- File count
- Directory count
- Total bytes
- Total lines
- Non-empty lines
- Blank lines
- Extension breakdown
- Largest files
- Average file size

## Error Handling

Batch analysis should not abort if one repository is unreadable or malformed. Results should include successful analyses plus a list of per-repository failures. A stricter fail-fast mode can be added later for CI-style workflows.

## Testing Strategy

### Unit tests

- Git output parsing
- Timestamp bucketing
- Metrics counters
- Report rendering helpers
- TUI state transitions
- CLI argument parsing

### Integration tests

- Single-repository analysis
- Multi-repository comparison
- JSON output snapshots
- Markdown output snapshots
- Partial-failure behavior

### Fixtures

Use tiny deterministic fixture repositories created for tests. Prefer generating commit history in temporary repositories from controlled inputs rather than relying on the current repository history.

## Tooling

### Recommended dependencies

- `clap`
- `ratatui`
- `crossterm`
- `serde`
- `serde_json`
- `ignore`
- `time`
- `thiserror`
- `anyhow`
- `assert_cmd`
- `insta`

### Developer workflow tooling

- `just` for top-level commands
- `cargo xtask` for non-trivial automation
- `cargo fmt`
- `cargo clippy`
- `cargo test`
- `cargo dist`

## Release Strategy

### Release artifacts

- `aarch64-apple-darwin` tarball
- `x86_64-apple-darwin` tarball
- `x86_64-unknown-linux-gnu` tarball
- Checksums
- JSON manifest from `cargo-dist`
- Homebrew formula publication to a dedicated tap

### Release system

- GitHub Releases as the source of truth
- GitHub Actions for CI and tagged releases
- `cargo-dist` for release planning, packaging, and Homebrew publishing

## Homebrew Strategy

Use a dedicated tap repository such as `maximgorbatyuk/homebrew-repolyze`. Homebrew distribution is only for macOS; Linux users should install from release tarballs in v1.

The tap workflow should document:

- Required tap repository
- Formula naming
- URL and SHA256 updates
- Local install and reinstall checks
- Post-release validation

## Risks And Deferred Work

### Risks

- Parsing large Git histories can be slow without caching
- Email-based contributor aggregation can split one person across identities
- Counting lines in mixed encodings and large binaries needs careful rules

### Deferred work

- Alias mapping config for contributor identity merge
- Cached analysis runs
- Remote repository inputs
- Language-aware parsing
- Rich TUI charts
- Windows artifacts
