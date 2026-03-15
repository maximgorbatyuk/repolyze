# Analysis Rules

## Caching Behavior

Analysis results are cached in a local SQLite database keyed by repository path + `HEAD` commit hash. Whether data is recalculated depends on the repository state:

| Scenario | Recalculates? | Reason |
|----------|--------------|--------|
| Same repo, same HEAD, clean worktree | No | Cache hit — returns stored `RepositoryAnalysis` JSON |
| Same repo, same HEAD, dirty worktree | Yes | Cache bypass — uncommitted changes may affect file metrics |
| Same repo, new commit (different HEAD) | Yes | Cache miss — new HEAD hash has no cached entry |
| RF-8/RF-9 table rows from cached data | Yes | Row builders always run in memory from `ContributionSummary` |

## What Gets Cached

The SQLite store (`~/.repolyze/repolyze.db` in release, `target/debug/repolyze-dev.db` in dev) persists:

- **Repository metadata** — canonical path, display name, first/last seen timestamps
- **Analysis snapshots** — full serialized `RepositoryAnalysis` JSON, keyed by `(repository_id, history_scope, head_commit_hash)`
- **Scan run history** — trigger source, cache status (hit/miss/bypass), success/failure, timestamps

## What Is NOT Cached

- RF-8 (Users contribution) and RF-9 (Most active days and hours) table rows — always computed from the in-memory `ContributionSummary`
- ASCII table rendering — always generated fresh
- Cross-repository contributor merging — always recomputed since it depends on which repositories are included in the current run

## Cache Key

The cache key for a repository snapshot is:

1. **Canonical repository path** — absolute, resolved path to the repo root
2. **History scope** — currently always `"head"` (commits reachable from HEAD)
3. **HEAD commit hash** — the current `git rev-parse HEAD` value

A cache hit requires all three to match an existing snapshot, plus the worktree must be clean (`git status --porcelain` returns empty output).

## Worktree Dirty Detection

Before checking the cache, repolyze runs `git status --porcelain --untracked-files=all`. If the output is non-empty, the `cacheable` flag is set to `false` and the cache is bypassed entirely — both for reading and writing. This ensures that file metrics (file count, line count, byte count) reflect the current state of the working directory, including uncommitted and untracked files.
