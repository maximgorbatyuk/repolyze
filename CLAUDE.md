# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Repolyze is a Rust CLI/TUI tool for analyzing Git repositories — both local and remote GitHub repos. It ships a single binary (`repolyze`) that defaults to a full-screen TUI and also exposes `analyze` and `compare` subcommands for scripting. GitHub repositories are analyzed via the GitHub API without cloning.

## Architecture

Rust workspace with one binary crate and multiple library crates:

- **repolyze-cli** — Binary entrypoint, `clap` command parsing, launches TUI or runs subcommands
- **repolyze-tui** — TUI app state, event loop, rendering (thin presentation layer, no domain logic). Includes Git Tools screens (branch cleanup with repo picker and progress tracking) and contributor picker for user effort view
- **repolyze-core** — Shared domain types (`AnalysisRequest`, `RepositoryTarget`, `RepositoryAnalysis`, `HeatmapData`, `UserEffortData`), service traits (`GitAnalyzer`, `MetricsAnalyzer`), input validation, error types, aggregation, analytics builders (contribution rows, activity rows, heatmap, repo comparison, user effort, `get_contributor_emails`), `date_util` module (date arithmetic without chrono)
- **repolyze-git** — Git subprocess backend, commit history parsing, contribution stats, activity summaries, branch management (`branches` module: list merged/stale branches, delete with protected-branch guard)
- **repolyze-github** — GitHub API backend for remote repository analysis. Dual HTTP transport: prefers `gh` CLI (5000 req/hr) when authenticated, falls back to `ureq` direct HTTP (60 req/hr unauthenticated). Uses `/stats/contributors`, `/stats/punch_card`, and `/languages` endpoints for efficient data fetching
- **repolyze-metrics** — `.gitignore`-aware repo walking, file/line/byte counting, extension breakdowns
- **repolyze-report** — JSON, Markdown, and plain-text table report rendering. Table renderer (`table.rs`) provides `render_plain_table` and specialized functions for contribution, activity, heatmap, and repo comparison output
- **repolyze-store** — SQLite cache (`rusqlite`), database bootstrap, migrations, snapshot read/write queries
- **xtask** — Developer automation (verification, release helpers)

Domain logic lives in library crates so both TUI and CLI call the same services.

## Build Commands

```bash
cargo run                                # run repolyze (dev build, launches TUI)
cargo run -- analyze --format json       # run a subcommand during development
cargo run --manifest-path xtask/Cargo.toml -- verify  # fmt-check + clippy + test + check (primary workflow)
cargo build --workspace                  # build all crates
cargo build --workspace --release        # release build
cargo test --workspace                   # run all tests
cargo test -p repolyze-git              # test a single crate
cargo test -p repolyze-core input       # run tests matching "input" in one crate
cargo fmt --all --check                  # format check
cargo clippy --workspace --all-targets --all-features -- -D warnings  # lint
```

A `justfile` exists with the same targets (`just verify`, `just test`, etc.) but `just` may not be installed. The `cargo xtask` alias is configured in `.cargo/config.toml`.

## CLI Usage

```bash
repolyze                                 # launch TUI (default)
repolyze tui                             # launch TUI (explicit)
repolyze analyze                         # analyze current directory, JSON to stdout
repolyze analyze -D /path/to/repo        # analyze a specific directory
repolyze analyze --repo ./a --repo ./b   # analyze specific repos
repolyze analyze --repo https://github.com/owner/repo  # analyze a GitHub repo (no clone)
repolyze analyze --format md --output report.md  # Markdown to file
repolyze analyze contribution --format table                  # contribution table to stdout
repolyze analyze activity --format table                      # activity table to stdout
repolyze analyze user-effort --email user@example.com         # per-user deep-dive to stdout
repolyze compare --repo ./a --repo ./b   # compare multiple repos
```

Global flag `-D` / `--directory` sets the working directory before any subcommand runs. For `analyze`, `--repo` is optional and defaults to `.` (current directory). For `compare`, `--repo` is required (2+ repos).

## Release Workflow

```bash
cargo xtask release --dry-run --version 0.1.0  # dry-run (runs verify + checks clean tree)
cargo xtask release --version 0.1.0            # prepare release
cargo dist plan                                # verify cargo-dist config
```

Release artifacts are built by GitHub Actions on version tags via `cargo-dist`. Targets: macOS (aarch64, x86_64), Linux (x86_64), and Windows (x86_64). Installers: shell script (macOS/Linux), PowerShell script (Windows), Homebrew formula (`maximgorbatyuk/homebrew-tap`), and MSI (Windows).

## Database

- Dev builds (`cargo run`): `target/debug/repolyze-dev.db`
- Release builds (installed binary): `~/.repolyze/repolyze.db`
- Detection uses `cfg!(debug_assertions)` — no env var needed. Path resolution uses the `home` crate for cross-platform home directory lookup
- Tests always use `tempfile::tempdir()`, never the real DB

## Design Constraints

- No language-aware parsing (classes, functions) in v1
- GitHub remote analysis via API (no cloning). Prefers `gh` CLI when available; falls back to unauthenticated HTTP. Some fields (files_touched, file_extensions, line counts) are unavailable for remote repos
- Batch analysis tolerates partial failures (one bad repo doesn't abort the run)
- TUI is a thin presentation layer — widgets never compute Git or filesystem metrics directly
- TUI analysis runs on a background thread via `mpsc::channel`; the event loop uses `poll(100ms)` for non-blocking input
- Git analysis uses subprocess calls (`git log`, `git rev-list`), not libgit2
- Tests use deterministic fixture repos with fixed timestamps and known authors, not the project's own Git history
- When adding/changing fields in serialized model types (e.g. `ContributorActivityStats`), bump `SCHEMA_VERSION` in `repolyze-store/src/migrations.rs` to invalidate stale cached snapshots

## Testing

- Tests use `tempfile::tempdir()` + `git init` to create fixture repos with controlled authors, timestamps, and content
- Fixture helper: `crates/repolyze-git/tests/support/mod.rs` (`CommitSpec` + `create_fixture_repo`)
- CLI integration tests use `assert_cmd` and `predicates` crates
- Never test against the project's own Git history — always use fixtures

## Conventions

- Follow test-driven development: write failing test → implement → verify green
- Commit after each task using conventional commits (`feat:`, `fix:`, `chore:`, `test:`, `docs:`)
- CI runs on push to `main`/`dev` and PRs to `main`: fmt check, clippy, build, test
- Release via `cargo-dist` with GitHub Actions; macOS, Linux, and Windows
- CI runs Windows build + test alongside Linux checks

## Table Output Format

All plain-text tables (CLI and TUI) must follow this format. Renderer: `crates/repolyze-report/src/table.rs`.

```
Period:    2024-03-01 .. 2025-03-15
Projects:  2 repositories
Folder:    /Users/dev/projects
Mode:      Multi-repository
Elapsed:   1.234s

Column A           Column B  Column C
-----------------  --------  --------
left-aligned text        42     12.50
another row               7      3.00
-----------------  --------  --------
Total                    49     15.50
```

Rules:
- No `|` borders — columns separated by two spaces
- Header row is always left-aligned
- Numeric columns are right-aligned
- Dash separator after header and before totals row
- Totals row only on contribution table
- Summary header (period, project count, folder, mode, elapsed) precedes every table output

## Rust 2024 Edition Gotchas

- Pattern `|(_, &c)| c` is rejected — use `|(_, c)| *c` instead
- Use `std::io::Error::other()` not `Error::new(ErrorKind::Other, ...)`
- Use `std::slice::from_ref(&x)` not `&[x.clone()]`
- Data record constructors with 8+ args need `#[allow(clippy::too_many_arguments)]`
- `rusqlite::Connection` methods take `&self` — store wrapper methods should too
- crossterm on Windows fires both `KeyEventKind::Press` and `KeyEventKind::Release` — always filter `key.kind == KeyEventKind::Press` in the event loop
