# Homebrew Installation Guide

## Tap repository

Repolyze uses a dedicated Homebrew tap at `maximgorbatyuk/homebrew-repolyze`.

This tap repository must exist on GitHub before the first release. `cargo-dist` publishes the formula automatically during the release workflow.

## Installation

```bash
brew tap maximgorbatyuk/repolyze
brew install repolyze
repolyze --help
```

## Updating

```bash
brew update
brew upgrade repolyze
```

## Uninstalling

```bash
brew uninstall repolyze
brew untap maximgorbatyuk/repolyze
```

## How it works

During a tagged release, `cargo-dist` generates a Homebrew formula (`repolyze-cli.rb`) that:

- Points to the release tarball URL for the current version
- Includes the SHA256 checksum for verification
- Installs the `repolyze` binary into the Homebrew prefix

The formula is pushed to the `maximgorbatyuk/homebrew-repolyze` tap repository automatically.

## Formula fields updated per release

- `version` — the SemVer version
- `url` — tarball download URL for the tagged release
- `sha256` — checksum of the tarball

## Manual fallback

If the automated publish fails, you can update the tap manually:

1. Download the release tarball and compute its SHA256:

```bash
curl -LO https://github.com/maximgorbatyuk/repolyze/releases/download/vX.Y.Z/repolyze-cli-aarch64-apple-darwin.tar.xz
shasum -a 256 repolyze-cli-aarch64-apple-darwin.tar.xz
```

2. Update the formula in the tap repository with the new URL and SHA256.

3. Test locally:

```bash
brew uninstall repolyze
brew install --formula ./repolyze.rb
repolyze --help
```

## Reference formula

A template formula is available at `packaging/homebrew/repolyze.rb` for manual fallback scenarios.

## Limitations

- Homebrew distribution is macOS only in v1
- Linux users should install from release tarballs or the shell installer
