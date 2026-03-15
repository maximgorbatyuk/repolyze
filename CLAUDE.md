# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Repolyze is a Rust CLI/TUI tool for analyzing local Git repositories. It ships a single binary (`repolyze`) that defaults to a full-screen TUI and also exposes `analyze` and `compare` subcommands for scripting.

## Architecture

Rust workspace with one binary crate and multiple library crates:

- **repolyze-cli** — Binary entrypoint, `clap` command parsing, launches TUI or runs subcommands
- **repolyze-tui** — TUI app state, event loop, rendering (thin presentation layer, no domain logic)
- **repolyze-core** — Shared domain types (`AnalysisRequest`, `RepositoryTarget`, `RepositoryAnalysis`), service traits (`GitAnalyzer`, `MetricsAnalyzer`), input validation, error types, aggregation
- **repolyze-git** — Git subprocess backend, commit history parsing, contribution stats, activity summaries
- **repolyze-metrics** — `.gitignore`-aware repo walking, file/line/byte counting, extension breakdowns
- **repolyze-report** — JSON and Markdown report rendering (aggregates contributors/activity across repos)
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

A `justfile` exists with the same targets (`just verify`, `just test`, etc.) but `just` may not be installed. The `cargo xtask` alias is not configured — use `cargo run --manifest-path xtask/Cargo.toml -- verify` directly.

## CLI Usage

```bash
repolyze                                 # launch TUI (default)
repolyze tui                             # launch TUI (explicit)
repolyze analyze                         # analyze current directory, JSON to stdout
repolyze analyze -D /path/to/repo        # analyze a specific directory
repolyze analyze --repo ./a --repo ./b   # analyze specific repos
repolyze analyze --format md --output report.md  # Markdown to file
repolyze compare --repo ./a --repo ./b   # compare multiple repos
```

Global flag `-D` / `--directory` sets the working directory before any subcommand runs. For `analyze`, `--repo` is optional and defaults to `.` (current directory). For `compare`, `--repo` is required (2+ repos).

## Release Workflow

```bash
cargo xtask release --dry-run --version 0.1.0  # dry-run (runs verify + checks clean tree)
cargo xtask release --version 0.1.0            # prepare release
cargo dist plan                                # verify cargo-dist config
```

Release artifacts are built by GitHub Actions on version tags via `cargo-dist`. Targets: macOS (aarch64, x86_64) and Linux (x86_64). Homebrew formula publishes to `maximgorbatyuk/homebrew-tap`.

## Database

- Dev builds (`cargo run`): `target/debug/repolyze-dev.db`
- Release builds (installed binary): `~/.repolyze/repolyze.db`
- Detection uses `cfg!(debug_assertions)` — no env var needed
- Tests always use `tempfile::tempdir()`, never the real DB

## Design Constraints

- No language-aware parsing (classes, functions) in v1
- Local filesystem analysis only — no remote GitHub/GitLab
- Batch analysis tolerates partial failures (one bad repo doesn't abort the run)
- TUI is a thin presentation layer — widgets never compute Git or filesystem metrics directly
- Git analysis uses subprocess calls (`git log`, `git rev-list`), not libgit2
- Tests use deterministic fixture repos with fixed timestamps and known authors, not the project's own Git history

## Testing

- Tests use `tempfile::tempdir()` + `git init` to create fixture repos with controlled authors, timestamps, and content
- Fixture helper: `crates/repolyze-git/tests/support/mod.rs` (`CommitSpec` + `create_fixture_repo`)
- CLI integration tests use `assert_cmd` and `predicates` crates
- Never test against the project's own Git history — always use fixtures

## Conventions

- Follow test-driven development: write failing test → implement → verify green
- Commit after each task using conventional commits (`feat:`, `fix:`, `chore:`, `test:`, `docs:`)
- CI runs on push to `main`/`dev` and PRs to `main`: fmt check, clippy, build, test
- Release via `cargo-dist` with GitHub Actions; macOS + Linux only (no Windows in v1)

## Table Output Format

All plain-text tables (CLI and TUI) must follow this format. Renderer: `crates/repolyze-report/src/table.rs`.

```
Period:    2024-03-01 .. 2025-03-15
Projects:  2 repositories
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
- Totals row only on RF-8 (Users contribution) table
- Summary header (period, project count, elapsed) precedes every table output

## Rust 2024 Edition Gotchas

- Pattern `|(_, &c)| c` is rejected — use `|(_, c)| *c` instead
- Use `std::io::Error::other()` not `Error::new(ErrorKind::Other, ...)`
- Use `std::slice::from_ref(&x)` not `&[x.clone()]`
- Data record constructors with 8+ args need `#[allow(clippy::too_many_arguments)]`
- `rusqlite::Connection` methods take `&self` — store wrapper methods should too
