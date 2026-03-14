# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Repolyze is a Rust CLI/TUI tool for analyzing local Git repositories. It ships a single binary (`repolyze`) that defaults to a full-screen TUI and also exposes `analyze` and `compare` subcommands for scripting.

## Architecture

Rust workspace with one binary crate and multiple library crates:

- **repolyze-cli** — Binary entrypoint, `clap` command parsing, launches TUI or runs subcommands
- **repolyze-tui** — TUI app state, event loop, rendering (thin presentation layer, no domain logic)
- **repolyze-core** — Shared domain types (`AnalysisRequest`, `RepositoryTarget`, `RepositoryAnalysis`), service traits (`GitAnalyzer`, `MetricsAnalyzer`), input validation, error types
- **repolyze-git** — Git subprocess backend, commit history parsing, contribution stats, activity summaries
- **repolyze-metrics** — `.gitignore`-aware repo walking, file/line/byte counting, extension breakdowns
- **repolyze-report** — JSON and Markdown report rendering
- **xtask** — Developer automation (verification, release helpers)

Domain logic lives in library crates so both TUI and CLI call the same services.

## Build Commands

```bash
cargo build --workspace                  # build all crates
cargo build --workspace --release        # release build
cargo test --workspace                   # run all tests
cargo test -p repolyze-git              # test a single crate
cargo test -p repolyze-core input       # run tests matching "input" in one crate
cargo fmt --all --check                  # format check
cargo clippy --workspace --all-targets --all-features -- -D warnings  # lint
```

When a `justfile` is added, use `just verify` to run fmt+clippy+test in one command.

## Design Constraints

- No language-aware parsing (classes, functions) in v1
- Local filesystem analysis only — no remote GitHub/GitLab
- Batch analysis tolerates partial failures (one bad repo doesn't abort the run)
- TUI is a thin presentation layer — widgets never compute Git or filesystem metrics directly
- Git analysis uses subprocess calls (`git log`, `git rev-list`), not libgit2
- Tests use deterministic fixture repos with fixed timestamps and known authors, not the project's own Git history

## Conventions

- Follow test-driven development: write failing test → implement → verify green
- Commit after each task using conventional commits (`feat:`, `fix:`, `chore:`, `test:`, `docs:`)
- CI runs on PRs to `main`: fmt check, clippy, build, test
- Release via `cargo-dist` with GitHub Actions; macOS + Linux only (no Windows in v1)
