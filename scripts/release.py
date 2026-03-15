#!/usr/bin/env python3
"""
Repolyze release script.

Release flow:
  1. Parse and validate version argument (X.Y.Z, all non-negative integers)
  2. Check prerequisites: gh CLI installed and authenticated
  3. Check clean git working tree (no uncommitted changes)
  4. Run cargo xtask verify (fmt, clippy, test, build)
  5. Switch to 'dev' branch, pull latest
  6. Update version in workspace Cargo.toml
  7. Run cargo check to verify Cargo.lock updates
  8. Commit version bump, push dev
  9. Switch to 'main' branch, pull latest
  10. Merge dev into main, push main
  11. Create and push tag vX.Y.Z
  12. Print success summary

Usage:
  ./scripts/release.py 0.2.0
"""

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = REPO_ROOT / "Cargo.toml"


def run(cmd: list[str], *, cwd: Path = REPO_ROOT, check: bool = True) -> subprocess.CompletedProcess:
    """Run a command, print it, and abort on failure."""
    print(f"\n  $ {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if result.stdout.strip():
        print(result.stdout.strip())
    if result.stderr.strip():
        print(result.stderr.strip())
    if check and result.returncode != 0:
        print(f"\nERROR: command failed with exit code {result.returncode}")
        sys.exit(1)
    return result


def validate_version(version: str) -> None:
    """Validate that version matches X.Y.Z where each part is a non-negative integer."""
    pattern = r"^\d+\.\d+\.\d+$"
    if not re.match(pattern, version):
        print(f"ERROR: invalid version '{version}'. Expected format: X.Y.Z (e.g. 0.2.0)")
        sys.exit(1)

    parts = version.split(".")
    for part in parts:
        n = int(part)
        if n < 0:
            print(f"ERROR: version parts must be non-negative, got '{part}'")
            sys.exit(1)

    print(f"  Version: {version}")


def check_gh_cli() -> None:
    """Check that the GitHub CLI is installed and authenticated."""
    result = subprocess.run(["which", "gh"], capture_output=True, text=True)
    if result.returncode != 0:
        print("ERROR: 'gh' CLI is not installed.")
        print("  Install it: brew install gh")
        print("  Then authenticate: gh auth login")
        sys.exit(1)

    result = subprocess.run(["gh", "auth", "status"], capture_output=True, text=True)
    if result.returncode != 0:
        print("ERROR: 'gh' CLI is not authenticated.")
        print("  Run: gh auth login")
        sys.exit(1)

    print("  gh CLI: installed and authenticated")


def check_clean_worktree() -> None:
    """Abort if there are uncommitted changes."""
    result = run(["git", "status", "--porcelain"], check=False)
    if result.stdout.strip():
        print("ERROR: working tree is not clean. Commit or stash changes first.")
        print(result.stdout.strip())
        sys.exit(1)
    print("  Working tree: clean")


def run_verify() -> None:
    """Run the full verification workflow: fmt, clippy, test, build."""
    print("\n--- Running cargo xtask verify ---")
    run(["cargo", "run", "--manifest-path", "xtask/Cargo.toml", "--", "verify"])


def switch_branch(branch: str) -> None:
    """Switch to a branch and pull latest from remote."""
    print(f"\n--- Switching to '{branch}' branch ---")
    run(["git", "checkout", branch])
    run(["git", "pull", "--ff-only", "origin", branch])


def update_version(version: str) -> None:
    """Update the workspace version in Cargo.toml."""
    print(f"\n--- Updating version to {version} ---")

    content = CARGO_TOML.read_text()
    updated = re.sub(
        r'^(version\s*=\s*)"[^"]+"',
        f'\\1"{version}"',
        content,
        count=1,
        flags=re.MULTILINE,
    )

    if updated == content:
        print("ERROR: failed to find version field in Cargo.toml")
        sys.exit(1)

    CARGO_TOML.write_text(updated)
    print(f"  Updated Cargo.toml: version = \"{version}\"")

    # Regenerate Cargo.lock with new version
    run(["cargo", "check", "--workspace"])


def commit_and_push_version(version: str) -> None:
    """Commit the version bump and push to dev."""
    print(f"\n--- Committing version bump ---")
    run(["git", "add", "Cargo.toml", "Cargo.lock"])
    run(["git", "commit", "-m", f"chore: bump version to {version}"])
    run(["git", "push", "origin", "dev"])


def merge_to_main() -> None:
    """Switch to main, merge dev, and push."""
    print("\n--- Merging dev into main ---")
    switch_branch("main")
    run(["git", "merge", "dev", "--no-edit"])
    run(["git", "push", "origin", "main"])


def create_and_push_tag(version: str) -> None:
    """Create a version tag and push it to trigger the release workflow."""
    tag = f"v{version}"
    print(f"\n--- Creating tag {tag} ---")
    run(["git", "tag", tag])
    run(["git", "push", "origin", tag])


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: ./scripts/release.py <version>")
        print("Example: ./scripts/release.py 0.2.0")
        sys.exit(1)

    version = sys.argv[1]

    print("=== Repolyze Release ===\n")

    # Step 1: Validate version
    print("Step 1: Validate version")
    validate_version(version)

    # Step 2: Check prerequisites
    print("\nStep 2: Check prerequisites")
    check_gh_cli()

    # Step 3: Check clean working tree
    print("\nStep 3: Check clean working tree")
    check_clean_worktree()

    # Step 4: Run verification
    print("\nStep 4: Run verification")
    run_verify()

    # Step 5: Switch to dev branch
    print("\nStep 5: Switch to dev branch")
    switch_branch("dev")

    # Step 6: Update version
    print("\nStep 6: Update version")
    update_version(version)

    # Step 7: Commit and push dev
    print("\nStep 7: Commit and push version bump")
    commit_and_push_version(version)

    # Step 8: Merge dev into main
    print("\nStep 8: Merge dev into main")
    merge_to_main()

    # Step 9: Create and push tag
    print("\nStep 9: Create and push tag")
    create_and_push_tag(version)

    # Step 10: Done
    tag = f"v{version}"
    print(f"\n=== Release {tag} complete ===")
    print(f"  Tag: {tag}")
    print(f"  GitHub Actions will build release artifacts via cargo-dist.")
    print(f"  Monitor: gh run list --workflow release.yml")


if __name__ == "__main__":
    main()
