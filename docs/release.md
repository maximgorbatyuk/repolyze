# Release Guide

## Pre-release checklist

- [ ] All tests pass: `cargo xtask verify`
- [ ] `CHANGELOG.md` is updated with the new version
- [ ] Version in `Cargo.toml` workspace matches the release tag
- [ ] Working tree is clean (`git status`)

## Version bump

Update the version in the workspace root `Cargo.toml`:

```toml
[workspace.package]
version = "X.Y.Z"
```

All workspace crates inherit this version.

## Local verification

```bash
cargo xtask verify
cargo build --workspace --release
cargo xtask release --dry-run --version X.Y.Z
cargo dist plan
```

## Creating a release

```bash
git checkout main
git pull --ff-only
cargo xtask verify
cargo xtask release --dry-run --version X.Y.Z
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

## CI release pipeline

Pushing a version tag triggers the `release.yml` workflow, which:

1. Runs `dist plan` to determine build matrix
2. Builds platform-specific artifacts (macOS aarch64, macOS x86_64, Linux x86_64)
3. Builds global artifacts (shell installer, Homebrew formula, checksums)
4. Creates a GitHub Release with all artifacts
5. Publishes the Homebrew formula to the tap repository

## Artifact validation

After the release workflow completes:

```bash
# Check the GitHub Release page
gh release view vX.Y.Z

# Download and test macOS binary
curl -LO https://github.com/maximgorbatyuk/repolyze/releases/download/vX.Y.Z/repolyze-cli-aarch64-apple-darwin.tar.xz
tar xf repolyze-cli-aarch64-apple-darwin.tar.xz
./repolyze --help
./repolyze --version

# Test with a local repo
./repolyze analyze --repo . --format json
```

## Post-release smoke tests

```bash
# Install via shell installer
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/maximgorbatyuk/repolyze/releases/download/vX.Y.Z/repolyze-cli-installer.sh | sh

# Verify
repolyze --help
repolyze --version
repolyze analyze --repo /path/to/repo --format json
```

For Homebrew installation, see [docs/homebrew.md](homebrew.md).
