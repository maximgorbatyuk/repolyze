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

verify:
    cargo xtask verify

build:
    cargo build --workspace --release

release-dry-run version:
    cargo xtask release --dry-run --version {{version}}

release version:
    cargo xtask release --version {{version}}
