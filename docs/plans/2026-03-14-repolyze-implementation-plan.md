# Repolyze v1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust workspace that ships a TUI-first repository analysis CLI for local Git repositories, with JSON and Markdown exports, deterministic tests, release automation, and Homebrew distribution guidance.

**Architecture:** Use a Rust workspace with one distributable binary crate and multiple library crates for Git analytics, filesystem metrics, reporting, and TUI rendering. Keep analysis logic in shared library crates so both the TUI and non-interactive subcommands call the same typed services and produce the same results.

**Tech Stack:** Rust stable, `clap`, `ratatui`, `crossterm`, `serde`, `serde_json`, `ignore`, `time`, `thiserror`, `anyhow`, `assert_cmd`, `insta`, `just`, `cargo-dist`, GitHub Actions.

---

## Global Execution Rules

- Follow `@test-driven-development` for every code task.
- Use `@verification-before-completion` before claiming any milestone is done.
- Make frequent commits after each task in this plan.
- Keep the first shipped binary name as `repolyze`.
- Do not add language-aware parsing in v1.
- Keep the initial TUI template menu to a single `Help` item until the template milestone is complete.

### Task 1: Bootstrap the Rust workspace

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `.editorconfig`
- Create: `justfile`
- Create: `xtask/Cargo.toml`
- Create: `xtask/src/main.rs`
- Create: `crates/repolyze-cli/Cargo.toml`
- Create: `crates/repolyze-cli/src/main.rs`
- Test: `crates/repolyze-cli/tests/cli_help.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-cli/tests/cli_help.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_cli_help() {
    let mut cmd = Command::cargo_bin("repolyze").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("repolyze"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-cli prints_cli_help -q`
Expected: FAIL because the workspace and `repolyze` binary do not exist yet.

**Step 3: Write minimal implementation**

Create a workspace root `Cargo.toml` with members:

```toml
[workspace]
members = ["crates/repolyze-cli", "xtask"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/maximgorbatyuk/repolyze"

[workspace.dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
```

Create `crates/repolyze-cli/Cargo.toml`:

```toml
[package]
name = "repolyze-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "repolyze"
path = "src/main.rs"

[dependencies]
clap.workspace = true
anyhow.workspace = true

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

Create `crates/repolyze-cli/src/main.rs`:

```rust
use clap::Command;

fn main() {
    let _ = Command::new("repolyze")
        .about("Repository analytics for local Git repositories")
        .get_matches();
}
```

Create `justfile` targets:

```make
default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace

check:
    cargo check --workspace --all-targets

verify: fmt-check lint test

build:
    cargo build --workspace --release
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-cli prints_cli_help -q`
Expected: PASS

Run: `just verify`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore .editorconfig justfile xtask crates/repolyze-cli
git commit -m "chore: bootstrap rust workspace"
```

### Task 2: Build the initial full-screen TUI template with only `Help`

**Files:**
- Create: `crates/repolyze-tui/Cargo.toml`
- Create: `crates/repolyze-tui/src/lib.rs`
- Create: `crates/repolyze-tui/src/app.rs`
- Create: `crates/repolyze-tui/src/ui.rs`
- Modify: `Cargo.toml`
- Modify: `crates/repolyze-cli/Cargo.toml`
- Modify: `crates/repolyze-cli/src/main.rs`
- Test: `crates/repolyze-tui/src/app.rs`

**Step 1: Write the failing test**

Add to `crates/repolyze-tui/src/app.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_help_as_the_only_menu_item() {
        let app = AppState::new();
        assert_eq!(app.menu_items, vec![MenuItem::Help]);
        assert_eq!(app.selected, 0);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-tui starts_with_help_as_the_only_menu_item -q`
Expected: FAIL because `repolyze-tui`, `AppState`, and `MenuItem` do not exist yet.

**Step 3: Write minimal implementation**

Create `crates/repolyze-tui/Cargo.toml`:

```toml
[package]
name = "repolyze-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
ratatui = "0.29"
crossterm = "0.28"
```

Create `crates/repolyze-tui/src/app.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    Help,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub menu_items: Vec<MenuItem>,
    pub selected: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            menu_items: vec![MenuItem::Help],
            selected: 0,
        }
    }
}
```

Create `crates/repolyze-tui/src/lib.rs`:

```rust
pub mod app;
pub mod ui;

pub fn run() -> anyhow::Result<()> {
    Ok(())
}
```

Update `crates/repolyze-cli/src/main.rs` to accept `tui` and to default to launching the TUI when no args are given.

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-tui starts_with_help_as_the_only_menu_item -q`
Expected: PASS

Run: `cargo run -p repolyze-cli -- tui`
Expected: exits cleanly from a minimal TUI stub or placeholder event loop.

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-tui crates/repolyze-cli
git commit -m "feat: add initial help-only tui shell"
```

### Task 3: Define shared core types and service interfaces

**Files:**
- Create: `crates/repolyze-core/Cargo.toml`
- Create: `crates/repolyze-core/src/lib.rs`
- Create: `crates/repolyze-core/src/model.rs`
- Create: `crates/repolyze-core/src/service.rs`
- Create: `crates/repolyze-core/src/error.rs`
- Modify: `Cargo.toml`
- Test: `crates/repolyze-core/src/model.rs`

**Step 1: Write the failing test**

Add to `crates/repolyze-core/src/model.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_request_supports_multiple_repositories() {
        let request = AnalysisRequest {
            repositories: vec!["/tmp/a".into(), "/tmp/b".into()],
        };

        assert_eq!(request.repositories.len(), 2);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-core analysis_request_supports_multiple_repositories -q`
Expected: FAIL because the crate and types do not exist.

**Step 3: Write minimal implementation**

Define the base domain types:

```rust
pub struct AnalysisRequest {
    pub repositories: Vec<std::path::PathBuf>,
}

pub struct RepositoryTarget {
    pub root: std::path::PathBuf,
}

pub struct RepositoryAnalysis {
    pub repository: RepositoryTarget,
    pub contributions: ContributionSummary,
    pub activity: ActivitySummary,
    pub size: SizeMetrics,
}
```

Define service traits:

```rust
pub trait GitAnalyzer {
    fn analyze_git(&self, target: &RepositoryTarget) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError>;
}

pub trait MetricsAnalyzer {
    fn analyze_size(&self, target: &RepositoryTarget) -> Result<SizeMetrics, RepolyzeError>;
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-core analysis_request_supports_multiple_repositories -q`
Expected: PASS

Run: `cargo check --workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-core
git commit -m "feat: add core domain models and service traits"
```

### Task 4: Implement input resolution and repository validation

**Files:**
- Create: `crates/repolyze-core/src/input.rs`
- Modify: `crates/repolyze-core/src/lib.rs`
- Test: `crates/repolyze-core/src/input.rs`

**Step 1: Write the failing test**

Add tests for:

```rust
#[test]
fn rejects_non_git_directories() {}

#[test]
fn deduplicates_equivalent_repository_paths() {}
```

The second test should create a temporary repo and pass both the original path and its canonical path.

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-core input -q`
Expected: FAIL because no resolver exists.

**Step 3: Write minimal implementation**

Implement:

```rust
pub fn resolve_inputs(paths: &[PathBuf]) -> Result<Vec<RepositoryTarget>, RepolyzeError>
```

Behavior:

- canonicalize each path
- sort and deduplicate
- accept either repo root or paths inside the worktree by walking upward until `.git` is found
- return a typed error for unreadable or non-Git paths

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-core input -q`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/repolyze-core/src/input.rs crates/repolyze-core/src/lib.rs
git commit -m "feat: add repository input resolution"
```

### Task 5: Add deterministic Git fixture support for tests

**Files:**
- Create: `crates/repolyze-git/Cargo.toml`
- Create: `crates/repolyze-git/src/lib.rs`
- Create: `crates/repolyze-git/tests/git_fixture.rs`
- Create: `crates/repolyze-git/tests/support/mod.rs`
- Create: `tests/fixtures/repos/basic-tree/README.md`
- Create: `tests/fixtures/repos/basic-tree/src/lib.rs`
- Modify: `Cargo.toml`
- Test: `crates/repolyze-git/tests/git_fixture.rs`

**Step 1: Write the failing test**

Create a test that generates a temporary Git repository with:

- two authors
- three commits
- fixed timestamps
- one modified file

Assert that `git rev-list --count HEAD` returns `3`.

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-git git_fixture_creates_deterministic_history -q`
Expected: FAIL because the fixture helper does not exist.

**Step 3: Write minimal implementation**

Create a helper API such as:

```rust
pub struct CommitSpec {
    pub author_name: &'static str,
    pub author_email: &'static str,
    pub authored_at: &'static str,
    pub rel_path: &'static str,
    pub contents: &'static str,
}

pub fn create_fixture_repo(specs: &[CommitSpec]) -> tempfile::TempDir
```

Implementation details:

- add `repolyze-git` to the workspace members
- keep `src/lib.rs` minimal; place fixture helpers under `tests/support/mod.rs`
- run `git init`
- set local `user.name` and `user.email`
- write files
- commit with `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE`
- avoid relying on global Git config

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-git git_fixture_creates_deterministic_history -q`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-git tests/fixtures/repos
git commit -m "test: add deterministic git fixture support"
```

### Task 6: Implement Git contribution statistics

**Files:**
- Create: `crates/repolyze-git/src/backend.rs`
- Create: `crates/repolyze-git/src/parse.rs`
- Create: `crates/repolyze-git/src/contributions.rs`
- Modify: `crates/repolyze-git/src/lib.rs`
- Test: `crates/repolyze-git/src/contributions.rs`

**Step 1: Write the failing test**

Create a fixture repo with two contributors and assert:

- contributor A has 2 commits
- contributor B has 1 commit
- lines added and deleted match fixture contents
- files touched count is correct

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-git contribution_stats_are_aggregated_by_email -q`
Expected: FAIL because the backend and parser do not exist.

**Step 3: Write minimal implementation**

Implement a subprocess backend that runs:

```bash
git log --format=%H%x1f%an%x1f%ae%x1f%aI --numstat
```

Parse the output into commits and per-file churn, then aggregate by normalized email address.

Expose:

```rust
pub struct GitCliBackend;

impl repolyze_core::service::GitAnalyzer for GitCliBackend {
    fn analyze_git(&self, target: &RepositoryTarget) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError> {
        // call parser and aggregation helpers
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-git contribution_stats_are_aggregated_by_email -q`
Expected: PASS

Run: `cargo test -p repolyze-git -- --nocapture`
Expected: all Git parser tests PASS

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-git
git commit -m "feat: add git contribution statistics"
```

### Task 7: Implement activity-by-hour and activity-by-day summaries

**Files:**
- Modify: `crates/repolyze-git/src/contributions.rs`
- Create: `crates/repolyze-git/src/activity.rs`
- Modify: `crates/repolyze-git/src/lib.rs`
- Test: `crates/repolyze-git/src/activity.rs`

**Step 1: Write the failing test**

Create a fixture repo with commits at fixed times across multiple weekdays and assert:

- hour bucket counts are correct
- day-of-week counts are correct
- the heatmap matrix contains the expected cells

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-git activity_histograms_use_commit_timestamps -q`
Expected: FAIL because no activity aggregator exists.

**Step 3: Write minimal implementation**

Add:

```rust
pub struct ActivitySummary {
    pub by_hour: [u32; 24],
    pub by_weekday: [u32; 7],
    pub heatmap: [[u32; 24]; 7],
}
```

Use parsed author timestamps and bucket by local timestamp offset preserved in the commit metadata.

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-git activity_histograms_use_commit_timestamps -q`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/repolyze-git/src/activity.rs crates/repolyze-git/src/contributions.rs crates/repolyze-git/src/lib.rs
git commit -m "feat: add git activity summaries"
```

### Task 8: Implement language-agnostic repository size metrics

**Files:**
- Create: `crates/repolyze-metrics/Cargo.toml`
- Create: `crates/repolyze-metrics/src/lib.rs`
- Create: `crates/repolyze-metrics/src/walk.rs`
- Create: `crates/repolyze-metrics/src/count.rs`
- Modify: `Cargo.toml`
- Test: `crates/repolyze-metrics/src/count.rs`

**Step 1: Write the failing test**

Create tests that assert:

- ignored files are skipped
- line counts match fixture files
- binary files are excluded from line counts but still counted for bytes
- extension totals are aggregated correctly

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-metrics size_metrics_skip_ignored_files -q`
Expected: FAIL because the crate does not exist.

**Step 3: Write minimal implementation**

Implement a `.gitignore`-aware walker using `ignore::WalkBuilder`.

Expose:

```rust
pub struct SizeMetrics {
    pub files: u64,
    pub directories: u64,
    pub total_bytes: u64,
    pub total_lines: u64,
    pub non_empty_lines: u64,
    pub blank_lines: u64,
    pub by_extension: BTreeMap<String, u64>,
    pub largest_files: Vec<FileMetric>,
}
```

Implementation rules:

- count directories separately
- treat undecodable files as binary
- count lines by scanning bytes for `\n`
- exclude `.git/`

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-metrics size_metrics_skip_ignored_files -q`
Expected: PASS

Run: `cargo test -p repolyze-metrics -- --nocapture`
Expected: all metrics tests PASS

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-metrics
git commit -m "feat: add repository size metrics"
```

### Task 9: Implement aggregation for single-repo and multi-repo results

**Files:**
- Create: `crates/repolyze-core/src/aggregate.rs`
- Modify: `crates/repolyze-core/src/lib.rs`
- Test: `crates/repolyze-core/src/aggregate.rs`

**Step 1: Write the failing test**

Add tests that build two `RepositoryAnalysis` values and assert:

- total file counts are summed correctly
- top contributors are merged by normalized email
- per-repo ordering is stable

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-core aggregate -q`
Expected: FAIL because no aggregation helpers exist.

**Step 3: Write minimal implementation**

Implement:

```rust
pub fn build_comparison_report(results: Vec<RepositoryAnalysis>) -> ComparisonReport
```

The comparison report should preserve per-repository detail and add:

- total contributor counts
- total commits
- total lines changed
- sortable size comparisons

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-core aggregate -q`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/repolyze-core/src/aggregate.rs crates/repolyze-core/src/lib.rs
git commit -m "feat: add comparison report aggregation"
```

### Task 10: Implement JSON export

**Files:**
- Create: `crates/repolyze-report/Cargo.toml`
- Create: `crates/repolyze-report/src/lib.rs`
- Create: `crates/repolyze-report/src/json.rs`
- Modify: `Cargo.toml`
- Test: `crates/repolyze-report/src/json.rs`

**Step 1: Write the failing test**

Create a test that serializes a `ComparisonReport` and asserts:

- top-level keys include `repositories` and `summary`
- contributor stats are nested under each repository
- JSON is pretty-printed

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-report json_export_contains_summary_fields -q`
Expected: FAIL because the crate does not exist.

**Step 3: Write minimal implementation**

Define serializable report models or derive `Serialize` directly on domain types.

Expose:

```rust
pub fn render_json(report: &ComparisonReport) -> anyhow::Result<String>
```

Use `serde_json::to_string_pretty`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-report json_export_contains_summary_fields -q`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-report/src/json.rs crates/repolyze-report/src/lib.rs crates/repolyze-report/Cargo.toml
git commit -m "feat: add json report export"
```

### Task 11: Implement Markdown report export

**Files:**
- Create: `crates/repolyze-report/src/markdown.rs`
- Test: `crates/repolyze-report/src/markdown.rs`

**Step 1: Write the failing test**

Create an `insta` snapshot test for a two-repository comparison report that asserts the Markdown contains:

- title
- repository summary table
- contributor section
- activity section
- size section

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-report markdown_report_snapshot -q`
Expected: FAIL because no Markdown renderer exists.

**Step 3: Write minimal implementation**

Expose:

```rust
pub fn render_markdown(report: &ComparisonReport) -> String
```

Use a predictable section order:

1. Title
2. Scope
3. Repository summary
4. Top contributors
5. Activity by hour
6. Activity by weekday
7. Size comparison
8. Failures, if any

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-report markdown_report_snapshot -q`
Expected: PASS

Run: `cargo insta test -p repolyze-report`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/repolyze-report/src/markdown.rs
git commit -m "feat: add markdown report export"
```

### Task 12: Wire the `analyze` CLI command

**Files:**
- Create: `crates/repolyze-cli/src/args.rs`
- Create: `crates/repolyze-cli/src/run.rs`
- Modify: `crates/repolyze-cli/src/main.rs`
- Test: `crates/repolyze-cli/tests/analyze_cli.rs`

**Step 1: Write the failing test**

Create a CLI integration test that:

- generates a fixture repo
- runs `repolyze analyze --repo <path> --format json`
- asserts success and valid JSON output

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-cli analyze_outputs_json -q`
Expected: FAIL because `analyze` is not implemented.

**Step 3: Write minimal implementation**

Add `clap` models:

```rust
#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Tui,
    Analyze(AnalyzeArgs),
    Compare(CompareArgs),
}
```

Implement `AnalyzeArgs` with:

- `--repo <path>` repeated
- `--format json|md`
- `--output <file>` optional

If `--output` is absent, print to stdout.

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-cli analyze_outputs_json -q`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/repolyze-cli/src crates/repolyze-cli/tests/analyze_cli.rs
git commit -m "feat: add analyze cli command"
```

### Task 13: Wire the `compare` CLI command

**Files:**
- Modify: `crates/repolyze-cli/src/args.rs`
- Modify: `crates/repolyze-cli/src/run.rs`
- Test: `crates/repolyze-cli/tests/compare_cli.rs`

**Step 1: Write the failing test**

Create a CLI integration test that:

- creates two fixture repos
- runs `repolyze compare --repo <a> --repo <b> --format md`
- asserts the Markdown report contains both repo names and comparison headings

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-cli compare_outputs_markdown -q`
Expected: FAIL because `compare` is not implemented.

**Step 3: Write minimal implementation**

Use shared services to:

- resolve all repo inputs
- analyze each repo
- aggregate into one `ComparisonReport`
- render JSON or Markdown

Add partial-failure reporting in the output body when one repo fails.

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-cli compare_outputs_markdown -q`
Expected: PASS

Run: `cargo test -p repolyze-cli -- --nocapture`
Expected: all CLI integration tests PASS

**Step 5: Commit**

```bash
git add crates/repolyze-cli/src crates/repolyze-cli/tests/compare_cli.rs
git commit -m "feat: add compare cli command"
```

### Task 14: Expand the TUI from the template into analysis screens

**Files:**
- Modify: `crates/repolyze-tui/src/app.rs`
- Modify: `crates/repolyze-tui/src/ui.rs`
- Create: `crates/repolyze-tui/src/event.rs`
- Create: `crates/repolyze-tui/src/screens/help.rs`
- Create: `crates/repolyze-tui/src/screens/analyze.rs`
- Create: `crates/repolyze-tui/src/screens/compare.rs`
- Create: `crates/repolyze-tui/src/screens/errors.rs`
- Test: `crates/repolyze-tui/src/app.rs`

**Step 1: Write the failing test**

Add state-transition tests that assert:

- the template menu starts with only `Help`
- after enabling analysis screens, navigation includes `Analyze` and `Compare`
- pressing `Enter` on `Analyze` dispatches an analysis action

**Step 2: Run test to verify it fails**

Run: `cargo test -p repolyze-tui app_navigation -q`
Expected: FAIL because the expanded state machine does not exist.

**Step 3: Write minimal implementation**

Introduce:

```rust
pub enum Screen {
    Help,
    Analyze,
    Compare,
    Errors,
}

pub enum AppAction {
    StartAnalyze(Vec<PathBuf>),
    StartCompare(Vec<PathBuf>),
    ShowErrors,
}
```

Keep analysis execution outside widget code. Use a service layer injected into the TUI app model.

**Step 4: Run test to verify it passes**

Run: `cargo test -p repolyze-tui app_navigation -q`
Expected: PASS

Run: `cargo run -p repolyze-cli`
Expected: TUI launches and basic navigation works.

**Step 5: Commit**

```bash
git add crates/repolyze-tui/src
git commit -m "feat: add analysis tui screens"
```

### Task 15: Add developer verification and CI automation

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `xtask/src/verify.rs`
- Create: `xtask/src/release.rs`
- Modify: `xtask/src/main.rs`
- Modify: `justfile`
- Test: `xtask/src/verify.rs`

**Step 1: Write the failing test**

Add `xtask` argument parsing tests that assert:

- `cargo xtask verify` is accepted
- `cargo xtask release --dry-run --version 0.1.0` is accepted

**Step 2: Run test to verify it fails**

Run: `cargo test -p xtask xtask_parses_release_args -q`
Expected: FAIL because the subcommands do not exist.

**Step 3: Write minimal implementation**

Extend `justfile`:

```make
verify:
    cargo xtask verify

dist-plan:
    cargo dist plan

release-dry-run version:
    cargo xtask release --dry-run --version {{version}}

release version:
    cargo xtask release --version {{version}}
```

Implement `cargo xtask verify` to run:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo check --workspace --all-targets`

Implement `.github/workflows/ci.yml` to run those same checks on pull requests.

**Step 4: Run test to verify it passes**

Run: `cargo test -p xtask xtask_parses_release_args -q`
Expected: PASS

Run: `cargo xtask verify`
Expected: PASS

**Step 5: Commit**

```bash
git add .github/workflows justfile xtask
git commit -m "chore: add verification scripts and ci"
```

### Task 16: Add release automation with `cargo-dist`

**Files:**
- Create: `dist-workspace.toml`
- Modify: `.github/workflows/release.yml`
- Modify: `justfile`
- Test: `dist-workspace.toml`

**Step 1: Write the failing test**

Use a release smoke check rather than a Rust unit test:

Run: `cargo dist plan`
Expected: FAIL because no `dist` configuration exists.

**Step 2: Run test to verify it fails**

Run: `cargo dist plan`
Expected: FAIL with missing config or package metadata errors.

**Step 3: Write minimal implementation**

Create `dist-workspace.toml`:

```toml
[workspace]
members = ["cargo:crates/repolyze-cli"]

[dist]
cargo-dist-version = "0.31.0"
ci = ["github"]
installers = ["shell", "homebrew"]
tap = "maximgorbatyuk/homebrew-repolyze"
targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-gnu",
]
pr-run-mode = "plan"
install-path = "CARGO_HOME"
install-updater = false
```

Update release workflow so a pushed tag like `v0.1.0` triggers `cargo-dist` planning and artifact publishing.

**Step 4: Run test to verify it passes**

Run: `cargo dist plan`
Expected: PASS

Run: `cargo dist build --artifacts=local`
Expected: PASS and local artifacts created under `target/distrib/`

**Step 5: Commit**

```bash
git add dist-workspace.toml .github/workflows/release.yml justfile
git commit -m "chore: add cargo-dist release automation"
```

### Task 17: Write release and Homebrew documentation

**Files:**
- Create: `CHANGELOG.md`
- Create: `docs/release.md`
- Create: `docs/homebrew.md`
- Create: `packaging/homebrew/repolyze.rb`
- Test: `docs/release.md`

**Step 1: Write the failing test**

Use a docs smoke check:

Run: `rg "just release|cargo dist|brew install" docs/release.md docs/homebrew.md packaging/homebrew/repolyze.rb`
Expected: FAIL because the docs and formula template do not exist.

**Step 2: Run test to verify it fails**

Run: `rg "just release|cargo dist|brew install" docs/release.md docs/homebrew.md packaging/homebrew/repolyze.rb`
Expected: FAIL

**Step 3: Write minimal implementation**

Create `docs/release.md` with:

1. Pre-release checklist
2. Version bump instructions
3. `CHANGELOG.md` update rules
4. Local verification commands:
   - `just verify`
   - `just build`
   - `just release-dry-run 0.1.0`
5. Tagging commands:

```bash
git checkout main
git pull --ff-only
just verify
just release-dry-run 0.1.0
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

6. CI monitoring expectations
7. Artifact validation commands
8. Post-release smoke tests for macOS and Linux tarballs

Create `docs/homebrew.md` with:

1. Tap repo naming convention
2. Requirement that the tap repo already exists
3. How `cargo-dist` publishes the formula
4. Manual fallback flow:

```bash
brew tap maximgorbatyuk/repolyze
brew install repolyze
repolyze --help
```

5. Formula update fields:
   - version
   - URL
   - SHA256
6. Local tap testing notes:
   - `brew uninstall repolyze`
   - `brew install --formula ./repolyze.rb`

Create `packaging/homebrew/repolyze.rb` as a reference formula template for manual fallback.

**Step 4: Run test to verify it passes**

Run: `rg "just release|cargo dist|brew install" docs/release.md docs/homebrew.md packaging/homebrew/repolyze.rb`
Expected: PASS

Run: `just verify`
Expected: PASS

**Step 5: Commit**

```bash
git add CHANGELOG.md docs/release.md docs/homebrew.md packaging/homebrew/repolyze.rb
git commit -m "docs: add release and homebrew guides"
```

### Task 18: Final verification pass before first distributable release

**Files:**
- Verify only: `Cargo.toml`
- Verify only: `dist-workspace.toml`
- Verify only: `.github/workflows/ci.yml`
- Verify only: `.github/workflows/release.yml`
- Verify only: `docs/release.md`
- Verify only: `docs/homebrew.md`

**Step 1: Run full verification**

Run:

```bash
just verify
cargo dist plan
cargo dist build --artifacts=local
```

Expected: PASS for all commands.

**Step 2: Validate the binary manually**

Run:

```bash
./target/release/repolyze --help
./target/release/repolyze analyze --help
./target/release/repolyze compare --help
```

Expected: PASS with correct subcommand help text.

**Step 3: Validate TUI startup manually**

Run: `./target/release/repolyze`
Expected: TUI opens, `Help` is reachable, quit key works.

**Step 4: Prepare the release**

Run:

```bash
git status --short
```

Expected: clean working tree before tagging.

**Step 5: Commit if any release-fixups were required**

```bash
git add .
git commit -m "chore: finalize v0.1.0 release prep"
```

Only do this step if verification required code or doc fixes.

## Deliverables Checklist

- Rust workspace with one distributable binary
- Full-screen TUI template first, then expanded analysis screens
- Shared Git and metrics analysis engine
- JSON export
- Markdown export
- `just` developer commands
- `xtask` verification and release helpers
- CI workflow
- `cargo-dist` release workflow
- Release guide
- Homebrew guide

## Release Operator Checklist

Use this exact flow for the first distributable release:

1. Update `CHANGELOG.md`.
2. Ensure the version in workspace manifests is correct.
3. Run `just verify`.
4. Run `just build`.
5. Run `just release-dry-run 0.1.0`.
6. Run `cargo dist plan`.
7. Create and push tag `v0.1.0`.
8. Wait for GitHub Actions release workflow to finish.
9. Verify GitHub Release artifacts for macOS and Linux.
10. Verify the Homebrew formula publish step or update the tap manually.
11. Install from Homebrew on macOS and from tarball on Linux, then run `repolyze --help`.
