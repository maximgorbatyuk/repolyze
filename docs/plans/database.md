# SQLite Analytics Cache Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an application-owned SQLite database that stores historical repository analysis snapshots, raw Git facts, and precomputed contributor stats needed to serve RF-8 and RF-9 efficiently.

**Architecture:** Introduce a new `repolyze-store` crate that owns database bootstrap, migrations, and cache read/write queries. Store one immutable `analysis_snapshot` per repository and `HEAD` commit hash, persist raw commit and file-change facts for traceability, and persist snapshot-scoped contributor summary/day/hour aggregates so RF-8 and RF-9 can read from SQLite without reparsing Git history.

**Tech Stack:** Rust 2024 workspace, SQLite, `rusqlite` with bundled SQLite, `serde_json`, existing `repolyze-core`, `repolyze-git`, CLI and TUI crates

---

## Scope And Defaults

- This plan assumes the database keeps **historical runs**, not only the latest snapshot.
- Default database path: `~/.repolyze/repolyze.db`.
- Cache key for the first version: `canonical repository path + history scope + HEAD commit hash`.
- RF-8 and RF-9 use `HEAD`-reachable history only, matching current behavior.
- The first version stores enough data for RF-8 and RF-9 and also keeps a JSON copy of `RepositoryAnalysis` so existing analyze/compare flows can reuse cached results without reconstructing every field from SQL.

## Recommended Schema

Use `PRAGMA user_version` for migration versioning and keep `app_settings` for app-owned settings only.

```sql
CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE repositories (
  id INTEGER PRIMARY KEY,
  canonical_path TEXT NOT NULL UNIQUE,
  display_name TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE analysis_snapshots (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
  history_scope TEXT NOT NULL,
  head_commit_hash TEXT NOT NULL,
  branch_name TEXT,
  analysis_period_start_at TEXT,
  analysis_period_end_at TEXT,
  commits_count INTEGER NOT NULL,
  contributors_count INTEGER NOT NULL,
  analysis_payload_json TEXT NOT NULL,
  snapshot_created_at TEXT NOT NULL,
  repolyze_version TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  is_complete INTEGER NOT NULL DEFAULT 1,
  UNIQUE (repository_id, history_scope, head_commit_hash)
);

CREATE TABLE scan_runs (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
  snapshot_id INTEGER REFERENCES analysis_snapshots(id),
  trigger_source TEXT NOT NULL,
  cache_status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  status TEXT NOT NULL,
  failure_reason TEXT
);

CREATE TABLE contributors (
  id INTEGER PRIMARY KEY,
  canonical_email TEXT NOT NULL UNIQUE,
  display_name_last_seen TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);

CREATE TABLE repository_commits (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  commit_hash TEXT NOT NULL,
  author_name TEXT NOT NULL,
  author_email TEXT NOT NULL,
  committed_at TEXT NOT NULL,
  commit_date TEXT NOT NULL,
  commit_hour INTEGER NOT NULL,
  commit_weekday INTEGER NOT NULL,
  files_changed_count INTEGER NOT NULL,
  lines_added INTEGER NOT NULL,
  lines_deleted INTEGER NOT NULL,
  lines_modified INTEGER NOT NULL,
  UNIQUE (repository_id, commit_hash)
);

CREATE TABLE snapshot_commits (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  commit_id INTEGER NOT NULL REFERENCES repository_commits(id),
  PRIMARY KEY (snapshot_id, commit_id)
);

CREATE TABLE commit_file_changes (
  id INTEGER PRIMARY KEY,
  commit_id INTEGER NOT NULL REFERENCES repository_commits(id),
  file_path TEXT NOT NULL,
  additions INTEGER NOT NULL,
  deletions INTEGER NOT NULL,
  lines_modified INTEGER NOT NULL
);

CREATE TABLE snapshot_contributor_summaries (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  commits_count INTEGER NOT NULL,
  lines_added INTEGER NOT NULL,
  lines_deleted INTEGER NOT NULL,
  lines_modified INTEGER NOT NULL,
  files_touched_count INTEGER NOT NULL,
  active_days_count INTEGER NOT NULL,
  first_commit_at TEXT NOT NULL,
  last_commit_at TEXT NOT NULL,
  most_active_weekday INTEGER,
  most_active_hour INTEGER,
  PRIMARY KEY (snapshot_id, contributor_id)
);

CREATE TABLE snapshot_contributor_weekday_stats (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  weekday INTEGER NOT NULL,
  commits_count INTEGER NOT NULL,
  active_dates_count INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id, contributor_id, weekday)
);

CREATE TABLE snapshot_contributor_hour_stats (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  hour_of_day INTEGER NOT NULL,
  commits_count INTEGER NOT NULL,
  active_hour_buckets_count INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id, contributor_id, hour_of_day)
);

CREATE INDEX idx_analysis_snapshots_repo_created
  ON analysis_snapshots (repository_id, snapshot_created_at DESC);

CREATE INDEX idx_scan_runs_repo_started
  ON scan_runs (repository_id, started_at DESC);

CREATE INDEX idx_repository_commits_repo_datetime
  ON repository_commits (repository_id, committed_at DESC);

CREATE INDEX idx_repository_commits_contributor_datetime
  ON repository_commits (contributor_id, committed_at DESC);

CREATE INDEX idx_commit_file_changes_commit
  ON commit_file_changes (commit_id);

CREATE INDEX idx_snapshot_contributor_summaries_contributor
  ON snapshot_contributor_summaries (contributor_id);
```

## Data Flow

1. Resolve repository input to a canonical repository root.
2. Open SQLite and ensure migrations have run.
3. Ask Git for lightweight repository metadata: `HEAD` hash and current branch name.
4. Start a `scan_runs` row with `cache_status = 'miss'`.
5. If a complete `analysis_snapshots` row already exists for the same cache key, return the cached `analysis_payload_json`, mark the `scan_runs` row as a cache hit, and skip live analysis.
6. On a cache miss, run normal Git and metrics analysis.
7. Persist contributors, commits, file changes, snapshot links, and contributor day/hour aggregates in one transaction.
8. Save the serialized `RepositoryAnalysis` JSON on `analysis_snapshots` so existing CLI/TUI report generation can hydrate the current model directly.

## Notes For RF-8 And RF-9

- `repository_commits` stores raw author/timestamp/change facts.
- `commit_file_changes` is required because RF-8 `Files Touched` cannot be reconstructed correctly from a single per-commit integer once we need distinct file paths.
- `snapshot_contributor_summaries` supports RF-8 directly.
- `snapshot_contributor_weekday_stats` and `snapshot_contributor_hour_stats` support RF-9 directly.
- Cross-repository RF-8/RF-9 merging should still group contributors by lowercased email, which matches the current codebase behavior.

### Task 1: Scaffold the SQLite store crate and bootstrap path handling

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/repolyze-store/Cargo.toml`
- Create: `crates/repolyze-store/src/lib.rs`
- Create: `crates/repolyze-store/src/error.rs`
- Create: `crates/repolyze-store/src/path.rs`
- Test: `crates/repolyze-store/tests/path_bootstrap.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-store/tests/path_bootstrap.rs`.

```rust
use repolyze_store::path::database_path_from_home;

#[test]
fn database_path_defaults_to_repolyze_db_in_home_directory() {
    let path = database_path_from_home("/tmp/test-home");
    assert_eq!(path, std::path::PathBuf::from("/tmp/test-home/.repolyze/repolyze.db"));
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-store database_path_defaults_to_repolyze_db_in_home_directory -- --exact`

Expected: FAIL because the crate and helper do not exist yet.

**Step 3: Write the minimal implementation**

Update `Cargo.toml` workspace members and add the new crate.

```toml
[workspace]
members = [
  "crates/repolyze-cli",
  "crates/repolyze-core",
  "crates/repolyze-git",
  "crates/repolyze-metrics",
  "crates/repolyze-report",
  "crates/repolyze-store",
  "crates/repolyze-tui",
  "xtask",
]
```

Create `crates/repolyze-store/Cargo.toml`.

```toml
[package]
name = "repolyze-store"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
repolyze-core = { path = "../repolyze-core" }
rusqlite = { version = "0.32", features = ["bundled"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

Create `crates/repolyze-store/src/lib.rs`.

```rust
pub mod error;
pub mod path;
```

Create `crates/repolyze-store/src/path.rs`.

```rust
use std::path::PathBuf;

pub fn database_path_from_home(home: &str) -> PathBuf {
    PathBuf::from(home).join(".repolyze").join("repolyze.db")
}
```

Create `crates/repolyze-store/src/error.rs`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
```

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-store path_bootstrap`

Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml crates/repolyze-store/Cargo.toml crates/repolyze-store/src/lib.rs crates/repolyze-store/src/error.rs crates/repolyze-store/src/path.rs crates/repolyze-store/tests/path_bootstrap.rs
git commit -m "feat: add sqlite store crate scaffold"
```

### Task 2: Add migrations for metadata tables and database bootstrap

**Files:**
- Modify: `crates/repolyze-store/src/lib.rs`
- Create: `crates/repolyze-store/src/migrations.rs`
- Create: `crates/repolyze-store/src/sqlite.rs`
- Test: `crates/repolyze-store/tests/migrations.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-store/tests/migrations.rs`.

```rust
use repolyze_store::sqlite::SqliteStore;

#[test]
fn sqlite_store_bootstrap_creates_metadata_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");

    let store = SqliteStore::open(&db_path).unwrap();
    let table_names = store.table_names().unwrap();

    assert!(table_names.contains(&"app_settings".to_string()));
    assert!(table_names.contains(&"repositories".to_string()));
    assert!(table_names.contains(&"analysis_snapshots".to_string()));
    assert!(table_names.contains(&"scan_runs".to_string()));
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-store sqlite_store_bootstrap_creates_metadata_tables -- --exact`

Expected: FAIL because `SqliteStore` and migrations do not exist yet.

**Step 3: Write the minimal implementation**

Create `crates/repolyze-store/src/migrations.rs` with explicit SQL.

```rust
pub const SCHEMA_VERSION: i32 = 1;

pub const MIGRATION_V1: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repositories (
  id INTEGER PRIMARY KEY,
  canonical_path TEXT NOT NULL UNIQUE,
  display_name TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS analysis_snapshots (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
  history_scope TEXT NOT NULL,
  head_commit_hash TEXT NOT NULL,
  branch_name TEXT,
  analysis_period_start_at TEXT,
  analysis_period_end_at TEXT,
  commits_count INTEGER NOT NULL,
  contributors_count INTEGER NOT NULL,
  analysis_payload_json TEXT NOT NULL,
  snapshot_created_at TEXT NOT NULL,
  repolyze_version TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  is_complete INTEGER NOT NULL DEFAULT 1,
  UNIQUE (repository_id, history_scope, head_commit_hash)
);

CREATE TABLE IF NOT EXISTS scan_runs (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
  snapshot_id INTEGER REFERENCES analysis_snapshots(id),
  trigger_source TEXT NOT NULL,
  cache_status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  status TEXT NOT NULL,
  failure_reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_analysis_snapshots_repo_created
  ON analysis_snapshots (repository_id, snapshot_created_at DESC);

CREATE INDEX IF NOT EXISTS idx_scan_runs_repo_started
  ON scan_runs (repository_id, started_at DESC);
"#;
```

Create `crates/repolyze-store/src/sqlite.rs`.

```rust
use std::path::Path;

use rusqlite::Connection;

use crate::error::StoreError;
use crate::migrations::{MIGRATION_V1, SCHEMA_VERSION};

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(MIGRATION_V1)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Self { conn })
    }

    pub fn table_names(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
```

Export the new modules from `crates/repolyze-store/src/lib.rs`.

```rust
pub mod error;
pub mod migrations;
pub mod path;
pub mod sqlite;
```

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-store migrations`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-store/src/lib.rs crates/repolyze-store/src/migrations.rs crates/repolyze-store/src/sqlite.rs crates/repolyze-store/tests/migrations.rs
git commit -m "feat: add sqlite metadata migrations"
```

### Task 3: Add raw repository, contributor, commit, and file-change tables

**Files:**
- Modify: `crates/repolyze-store/src/migrations.rs`
- Modify: `crates/repolyze-store/src/sqlite.rs`
- Create: `crates/repolyze-store/src/models.rs`
- Test: `crates/repolyze-store/tests/raw_git_facts.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-store/tests/raw_git_facts.rs`.

```rust
use repolyze_store::models::{CommitFileChangeRecord, CommitRecord, ContributorRecord};
use repolyze_store::sqlite::SqliteStore;

#[test]
fn raw_commit_writer_dedupes_commit_hash_per_repository() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let mut store = SqliteStore::open(&db_path).unwrap();

    let repository_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    let contributor_id = store.upsert_contributor(&ContributorRecord::new("alice@example.com", "Alice")).unwrap();

    let commit = CommitRecord::new(repository_id, contributor_id, "abc123", "Alice", "alice@example.com", "2025-01-15T10:00:00+00:00", 10, 2, 2, 12, 4, 16);
    let file_change = CommitFileChangeRecord::new("src/lib.rs", 12, 4, 16);

    let first_id = store.upsert_commit(&commit, &[file_change.clone()]).unwrap();
    let second_id = store.upsert_commit(&commit, &[file_change]).unwrap();

    assert_eq!(first_id, second_id);
    assert_eq!(store.commit_count(repository_id).unwrap(), 1);
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-store raw_commit_writer_dedupes_commit_hash_per_repository -- --exact`

Expected: FAIL because the raw Git fact tables and write helpers do not exist yet.

**Step 3: Write the minimal implementation**

Extend `crates/repolyze-store/src/migrations.rs` with the raw-fact tables and indexes.

```rust
CREATE TABLE IF NOT EXISTS contributors (
  id INTEGER PRIMARY KEY,
  canonical_email TEXT NOT NULL UNIQUE,
  display_name_last_seen TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repository_commits (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  commit_hash TEXT NOT NULL,
  author_name TEXT NOT NULL,
  author_email TEXT NOT NULL,
  committed_at TEXT NOT NULL,
  commit_date TEXT NOT NULL,
  commit_hour INTEGER NOT NULL,
  commit_weekday INTEGER NOT NULL,
  files_changed_count INTEGER NOT NULL,
  lines_added INTEGER NOT NULL,
  lines_deleted INTEGER NOT NULL,
  lines_modified INTEGER NOT NULL,
  UNIQUE (repository_id, commit_hash)
);

CREATE TABLE IF NOT EXISTS commit_file_changes (
  id INTEGER PRIMARY KEY,
  commit_id INTEGER NOT NULL REFERENCES repository_commits(id),
  file_path TEXT NOT NULL,
  additions INTEGER NOT NULL,
  deletions INTEGER NOT NULL,
  lines_modified INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_repository_commits_repo_datetime
  ON repository_commits (repository_id, committed_at DESC);

CREATE INDEX IF NOT EXISTS idx_repository_commits_contributor_datetime
  ON repository_commits (contributor_id, committed_at DESC);

CREATE INDEX IF NOT EXISTS idx_commit_file_changes_commit
  ON commit_file_changes (commit_id);
```

Create `crates/repolyze-store/src/models.rs`.

```rust
#[derive(Debug, Clone)]
pub struct ContributorRecord {
    pub canonical_email: String,
    pub display_name_last_seen: String,
}

impl ContributorRecord {
    pub fn new(email: &str, name: &str) -> Self {
        Self {
            canonical_email: email.to_lowercase(),
            display_name_last_seen: name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitRecord {
    pub repository_id: i64,
    pub contributor_id: i64,
    pub commit_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub committed_at: String,
    pub commit_date: String,
    pub commit_hour: i64,
    pub commit_weekday: i64,
    pub files_changed_count: i64,
    pub lines_added: i64,
    pub lines_deleted: i64,
    pub lines_modified: i64,
}

impl CommitRecord {
    pub fn new(repository_id: i64, contributor_id: i64, commit_hash: &str, author_name: &str, author_email: &str, committed_at: &str, commit_hour: i64, commit_weekday: i64, files_changed_count: i64, lines_added: i64, lines_deleted: i64, lines_modified: i64) -> Self {
        let commit_date = committed_at.split('T').next().unwrap_or_default().to_string();
        Self {
            repository_id,
            contributor_id,
            commit_hash: commit_hash.to_string(),
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            committed_at: committed_at.to_string(),
            commit_date,
            commit_hour,
            commit_weekday,
            files_changed_count,
            lines_added,
            lines_deleted,
            lines_modified,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitFileChangeRecord {
    pub file_path: String,
    pub additions: i64,
    pub deletions: i64,
    pub lines_modified: i64,
}

impl CommitFileChangeRecord {
    pub fn new(file_path: &str, additions: i64, deletions: i64, lines_modified: i64) -> Self {
        Self {
            file_path: file_path.to_string(),
            additions,
            deletions,
            lines_modified,
        }
    }
}
```

Add `upsert_repository`, `upsert_contributor`, `upsert_commit`, and `commit_count` methods in `crates/repolyze-store/src/sqlite.rs`.

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-store raw_git_facts`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-store/src/migrations.rs crates/repolyze-store/src/models.rs crates/repolyze-store/src/sqlite.rs crates/repolyze-store/tests/raw_git_facts.rs
git commit -m "feat: store raw repository commit facts"
```

### Task 4: Add snapshot tables and RF-8/RF-9 aggregate tables

**Files:**
- Modify: `crates/repolyze-store/src/migrations.rs`
- Modify: `crates/repolyze-store/src/models.rs`
- Modify: `crates/repolyze-store/src/sqlite.rs`
- Test: `crates/repolyze-store/tests/snapshot_aggregates.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-store/tests/snapshot_aggregates.rs`.

```rust
use repolyze_store::sqlite::SqliteStore;

#[test]
fn snapshot_writer_persists_summary_weekday_and_hour_stats() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let mut store = SqliteStore::open(&db_path).unwrap();

    let repository_id = store.upsert_repository("/tmp/repo-a", "repo-a").unwrap();
    let snapshot_id = store.insert_snapshot_header(repository_id, "head", "abc123", Some("main"), Some("2025-01-01T00:00:00+00:00"), Some("2025-01-15T10:00:00+00:00"), 3, 1, "{}", "0.1.1").unwrap();
    let contributor_id = store.upsert_contributor_by_email("alice@example.com", "Alice").unwrap();

    store.upsert_snapshot_contributor_summary(snapshot_id, contributor_id, 3, 12, 4, 16, 2, 2, "2025-01-01T09:00:00+00:00", "2025-01-15T10:00:00+00:00", Some(2), Some(10)).unwrap();
    store.upsert_snapshot_contributor_weekday_stat(snapshot_id, contributor_id, 2, 2, 1).unwrap();
    store.upsert_snapshot_contributor_hour_stat(snapshot_id, contributor_id, 10, 2, 1).unwrap();

    let summary_rows = store.snapshot_summary_row_count(snapshot_id).unwrap();
    let weekday_rows = store.snapshot_weekday_row_count(snapshot_id).unwrap();
    let hour_rows = store.snapshot_hour_row_count(snapshot_id).unwrap();

    assert_eq!(summary_rows, 1);
    assert_eq!(weekday_rows, 1);
    assert_eq!(hour_rows, 1);
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-store snapshot_writer_persists_summary_weekday_and_hour_stats -- --exact`

Expected: FAIL because snapshot aggregate tables and write helpers do not exist yet.

**Step 3: Write the minimal implementation**

Extend `crates/repolyze-store/src/migrations.rs`.

```rust
CREATE TABLE IF NOT EXISTS snapshot_commits (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  commit_id INTEGER NOT NULL REFERENCES repository_commits(id),
  PRIMARY KEY (snapshot_id, commit_id)
);

CREATE TABLE IF NOT EXISTS snapshot_contributor_summaries (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  commits_count INTEGER NOT NULL,
  lines_added INTEGER NOT NULL,
  lines_deleted INTEGER NOT NULL,
  lines_modified INTEGER NOT NULL,
  files_touched_count INTEGER NOT NULL,
  active_days_count INTEGER NOT NULL,
  first_commit_at TEXT NOT NULL,
  last_commit_at TEXT NOT NULL,
  most_active_weekday INTEGER,
  most_active_hour INTEGER,
  PRIMARY KEY (snapshot_id, contributor_id)
);

CREATE TABLE IF NOT EXISTS snapshot_contributor_weekday_stats (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  weekday INTEGER NOT NULL,
  commits_count INTEGER NOT NULL,
  active_dates_count INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id, contributor_id, weekday)
);

CREATE TABLE IF NOT EXISTS snapshot_contributor_hour_stats (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id),
  contributor_id INTEGER NOT NULL REFERENCES contributors(id),
  hour_of_day INTEGER NOT NULL,
  commits_count INTEGER NOT NULL,
  active_hour_buckets_count INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id, contributor_id, hour_of_day)
);
```

Add the corresponding write helpers to `crates/repolyze-store/src/sqlite.rs`.

```rust
pub fn insert_snapshot_header(&mut self, repository_id: i64, history_scope: &str, head_commit_hash: &str, branch_name: Option<&str>, analysis_period_start_at: Option<&str>, analysis_period_end_at: Option<&str>, commits_count: i64, contributors_count: i64, analysis_payload_json: &str, repolyze_version: &str) -> Result<i64, StoreError> { /* insert statement */ }

pub fn upsert_snapshot_contributor_summary(&mut self, snapshot_id: i64, contributor_id: i64, commits_count: i64, lines_added: i64, lines_deleted: i64, lines_modified: i64, files_touched_count: i64, active_days_count: i64, first_commit_at: &str, last_commit_at: &str, most_active_weekday: Option<i64>, most_active_hour: Option<i64>) -> Result<(), StoreError> { /* insert or replace */ }
```

Keep the snapshot-scoped stats immutable after insert; if the same snapshot key already exists, load it instead of mutating it.

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-store snapshot_aggregates`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-store/src/migrations.rs crates/repolyze-store/src/models.rs crates/repolyze-store/src/sqlite.rs crates/repolyze-store/tests/snapshot_aggregates.rs
git commit -m "feat: persist snapshot aggregates for rf-8 and rf-9"
```

### Task 5: Add Git repository metadata helpers needed for cache keys

**Files:**
- Modify: `crates/repolyze-git/src/lib.rs`
- Modify: `crates/repolyze-git/src/backend.rs`
- Create: `crates/repolyze-git/src/repository.rs`
- Test: `crates/repolyze-git/tests/git_fixture.rs`

**Step 1: Write the failing test**

Extend `crates/repolyze-git/tests/git_fixture.rs`.

```rust
#[test]
fn current_head_metadata_returns_head_hash_and_branch_name() {
    let repo = create_fixture_repo(&[CommitSpec {
        author_name: "Alice",
        author_email: "alice@example.com",
        authored_at: "2025-01-15T10:00:00+00:00",
        message: "initial",
        rel_path: "README.md",
        contents: "# Test\n",
    }]);

    let metadata = repolyze_git::repository::current_head_metadata(repo.path()).unwrap();

    assert!(!metadata.head_commit_hash.is_empty());
    assert!(!metadata.branch_name.is_empty());
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-git current_head_metadata_returns_head_hash_and_branch_name -- --exact`

Expected: FAIL because the helper does not exist yet.

**Step 3: Write the minimal implementation**

Create `crates/repolyze-git/src/repository.rs`.

```rust
use std::path::Path;

use repolyze_core::error::RepolyzeError;

use crate::backend::run_git;

#[derive(Debug, Clone)]
pub struct HeadMetadata {
    pub head_commit_hash: String,
    pub branch_name: String,
}

pub fn current_head_metadata(repo: &Path) -> Result<HeadMetadata, RepolyzeError> {
    let head_commit_hash = run_git(repo, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let branch_name = run_git(repo, &["branch", "--show-current"])?
        .trim()
        .to_string();

    Ok(HeadMetadata {
        head_commit_hash,
        branch_name,
    })
}
```

Export the new module from `crates/repolyze-git/src/lib.rs`.

```rust
pub mod activity;
pub mod backend;
pub mod contributions;
pub mod parse;
pub mod repository;
```

Update `crates/repolyze-git/src/backend.rs` so the concrete backend can expose this metadata to the shared service once the `GitAnalyzer` trait grows a cache-metadata method in Task 6.

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-git git_fixture`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-git/src/lib.rs crates/repolyze-git/src/backend.rs crates/repolyze-git/src/repository.rs crates/repolyze-git/tests/git_fixture.rs
git commit -m "feat: expose repository metadata for cache keys"
```

### Task 6: Add a cache-aware repository analysis service

**Files:**
- Modify: `crates/repolyze-core/src/error.rs`
- Modify: `crates/repolyze-core/src/service.rs`
- Modify: `crates/repolyze-git/src/backend.rs`
- Modify: `crates/repolyze-store/src/lib.rs`
- Modify: `crates/repolyze-store/src/sqlite.rs`
- Test: `crates/repolyze-core/src/service.rs`

**Step 1: Write the failing test**

Add a cache-hit test to `crates/repolyze-core/src/service.rs` using fake analyzers and a fake store.

```rust
#[test]
fn analyze_target_uses_cached_snapshot_when_key_matches() {
    let target = RepositoryTarget { root: "/tmp/repo-a".into() };
    let cached = make_repository_analysis("/tmp/repo-a");
    let git = PanicGitAnalyzer;
    let metrics = PanicMetricsAnalyzer;
    let store = FakeAnalysisStore::with_hit(cached.clone(), "abc123");

    let result = analyze_targets_with_store(&[target], &git, &metrics, &store);

    assert_eq!(result.repositories.len(), 1);
    assert_eq!(result.repositories[0].repository.root, cached.repository.root);
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-core analyze_target_uses_cached_snapshot_when_key_matches -- --exact`

Expected: FAIL because there is no store-aware orchestration path yet.

**Step 3: Write the minimal implementation**

Extend `crates/repolyze-core/src/service.rs`.

```rust
#[derive(Debug, Clone)]
pub struct RepositoryCacheMetadata {
    pub repository_root: std::path::PathBuf,
    pub history_scope: String,
    pub head_commit_hash: String,
    pub branch_name: Option<String>,
}

pub trait AnalysisStore {
    fn load_snapshot(&self, key: &RepositoryCacheMetadata) -> Result<Option<RepositoryAnalysis>, RepolyzeError>;
    fn save_snapshot(&self, key: &RepositoryCacheMetadata, analysis: &RepositoryAnalysis) -> Result<(), RepolyzeError>;
    fn record_scan_failure(&self, repository_root: &std::path::Path, reason: &str) -> Result<(), RepolyzeError>;
}

pub trait GitAnalyzer {
    fn cache_metadata(&self, target: &RepositoryTarget) -> Result<RepositoryCacheMetadata, RepolyzeError>;
    fn analyze_git(
        &self,
        target: &RepositoryTarget,
    ) -> Result<(ContributionSummary, ActivitySummary), RepolyzeError>;
}
```

Add `analyze_targets_with_store` that:

- calls `git.cache_metadata(target)` before expensive work
- tries `load_snapshot`
- falls back to normal analysis on a miss
- saves the snapshot on success
- records failures without aborting the batch

Add DB error handling to `crates/repolyze-core/src/error.rs`.

```rust
#[error("store error: {0}")]
Store(String),
```

Export `SqliteStore` and implement the trait in `crates/repolyze-store/src/sqlite.rs`.

Update `crates/repolyze-git/src/backend.rs` so `GitCliBackend` implements `cache_metadata` using `repository::current_head_metadata(&target.root)`.

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-core service`

Expected: PASS for existing service tests and the new cache-hit coverage.

**Step 5: Commit**

```bash
git add crates/repolyze-core/src/error.rs crates/repolyze-core/src/service.rs crates/repolyze-git/src/backend.rs crates/repolyze-store/src/lib.rs crates/repolyze-store/src/sqlite.rs
git commit -m "feat: add cache-aware analysis orchestration"
```

### Task 7: Persist full snapshots and RF-8/RF-9 facts from live analysis

**Files:**
- Modify: `crates/repolyze-git/src/contributions.rs`
- Modify: `crates/repolyze-git/src/activity.rs`
- Modify: `crates/repolyze-store/src/models.rs`
- Modify: `crates/repolyze-store/src/sqlite.rs`
- Test: `crates/repolyze-store/tests/cache_roundtrip.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-store/tests/cache_roundtrip.rs`.

```rust
#[test]
fn cache_roundtrip_restores_repository_analysis_and_snapshot_stats() {
    let repo = create_fixture_repo(&[CommitSpec {
        author_name: "Alice",
        author_email: "alice@example.com",
        authored_at: "2025-01-15T10:00:00+00:00",
        message: "initial",
        rel_path: "README.md",
        contents: "# Test\n",
    }]);

    let target = RepositoryTarget { root: repo.path().to_path_buf() };
    let git = GitCliBackend;
    let metrics = FilesystemMetricsBackend;
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let store = SqliteStore::open(&db_path).unwrap();

    let report = analyze_targets_with_store(&[target.clone()], &git, &metrics, &store);
    let cached = store.latest_snapshot_for_repository(&target.root).unwrap().unwrap();

    assert_eq!(report.repositories.len(), 1);
    assert_eq!(cached.analysis.repository.root, target.root);
    assert!(cached.contributor_summary_rows.len() >= 1);
    assert!(cached.weekday_rows.len() >= 1);
    assert!(cached.hour_rows.len() >= 1);
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-store cache_roundtrip_restores_repository_analysis_and_snapshot_stats -- --exact`

Expected: FAIL because the store does not yet persist and hydrate the full snapshot package.

**Step 3: Write the minimal implementation**

Extend the store models so one write transaction can carry everything needed for RF-8/RF-9.

```rust
pub struct SnapshotWriteRequest {
    pub canonical_repository_path: std::path::PathBuf,
    pub display_name: String,
    pub history_scope: String,
    pub head_commit_hash: String,
    pub branch_name: Option<String>,
    pub analysis_period_start_at: Option<String>,
    pub analysis_period_end_at: Option<String>,
    pub analysis_payload_json: String,
    pub contributor_summaries: Vec<SnapshotContributorSummaryRecord>,
    pub weekday_stats: Vec<SnapshotContributorWeekdayRecord>,
    pub hour_stats: Vec<SnapshotContributorHourRecord>,
    pub commits: Vec<CommitWriteBundle>,
}
```

Update `crates/repolyze-git/src/contributions.rs` and `crates/repolyze-git/src/activity.rs` so the live analysis path can produce the contributor weekday/hour facts required by `SnapshotWriteRequest`. Reuse the RF-8/RF-9 contributor fact model from `docs/plans/RF-8-9.md` rather than creating a second incompatible shape.

Add `save_snapshot_package` and `latest_snapshot_for_repository` to `crates/repolyze-store/src/sqlite.rs`. The write should happen in a single transaction.

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-store cache_roundtrip`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-git/src/contributions.rs crates/repolyze-git/src/activity.rs crates/repolyze-store/src/models.rs crates/repolyze-store/src/sqlite.rs crates/repolyze-store/tests/cache_roundtrip.rs
git commit -m "feat: persist full analysis snapshots and contributor facts"
```

### Task 8: Wire the SQLite store into CLI and TUI entry paths

**Files:**
- Modify: `crates/repolyze-cli/Cargo.toml`
- Modify: `crates/repolyze-cli/src/run.rs`
- Modify: `crates/repolyze-tui/Cargo.toml`
- Modify: `crates/repolyze-tui/src/lib.rs`
- Test: `crates/repolyze-cli/tests/analyze_cli.rs`
- Test: `crates/repolyze-tui/src/lib.rs`

**Step 1: Write the failing tests**

Add a CLI integration test in `crates/repolyze-cli/tests/analyze_cli.rs` that runs analysis twice and expects the second run to succeed with an existing database file.

```rust
#[test]
fn analyze_reuses_existing_database_on_second_run() {
    let repo = create_fixture_repo();
    let home = tempfile::tempdir().unwrap();

    let mut first = Command::cargo_bin("repolyze").unwrap();
    first.env("HOME", home.path())
        .args(["analyze", "--repo", repo.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success();

    let db_path = home.path().join(".repolyze/repolyze.db");
    assert!(db_path.exists());

    let mut second = Command::cargo_bin("repolyze").unwrap();
    second.env("HOME", home.path())
        .args(["analyze", "--repo", repo.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success();
}
```

Add a TUI test in `crates/repolyze-tui/src/lib.rs` that ensures `execute_pending_action` still returns a report when the store is present.

**Step 2: Run the tests to verify they fail**

Run: `cargo test -p repolyze-cli analyze_reuses_existing_database_on_second_run -- --exact`

Expected: FAIL because CLI does not instantiate the store yet.

**Step 3: Write the minimal implementation**

Add the store dependency to the CLI and TUI crates.

```toml
[dependencies]
repolyze-store = { path = "../repolyze-store" }
```

Update `crates/repolyze-cli/src/run.rs` and `crates/repolyze-tui/src/lib.rs` to open the DB once per command/session and call `analyze_targets_with_store` instead of `analyze_targets`.

Sketch for `crates/repolyze-cli/src/run.rs`:

```rust
let db_path = repolyze_store::path::database_path_from_home(
    &std::env::var("HOME").expect("HOME must be set")
);
if let Some(parent) = db_path.parent() {
    std::fs::create_dir_all(parent)?;
}
let store = repolyze_store::sqlite::SqliteStore::open(&db_path)?;
let mut report = analyze_targets_with_store(&targets, &git, &metrics, &store);
```

Keep the cache layer transparent to the caller; CLI and TUI should not need to know whether a result was live or cached beyond optional status messaging.

**Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test -p repolyze-cli analyze_cli
cargo test -p repolyze-tui
```

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-cli/Cargo.toml crates/repolyze-cli/src/run.rs crates/repolyze-cli/tests/analyze_cli.rs crates/repolyze-tui/Cargo.toml crates/repolyze-tui/src/lib.rs
git commit -m "feat: wire sqlite cache into cli and tui"
```

### Task 9: Add store-level RF-8 and RF-9 read queries

**Files:**
- Modify: `crates/repolyze-store/src/models.rs`
- Modify: `crates/repolyze-store/src/sqlite.rs`
- Test: `crates/repolyze-store/tests/rf8_rf9_queries.rs`

**Step 1: Write the failing test**

Create `crates/repolyze-store/tests/rf8_rf9_queries.rs`.

```rust
#[test]
fn rf8_and_rf9_queries_return_snapshot_scoped_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("repolyze.db");
    let mut store = SqliteStore::open(&db_path).unwrap();

    let fixture = seed_snapshot_with_one_contributor(&mut store);

    let rf8_rows = store.users_contribution_rows_for_snapshots(&[fixture.snapshot_id]).unwrap();
    let rf9_rows = store.user_activity_rows_for_snapshots(&[fixture.snapshot_id]).unwrap();

    assert_eq!(rf8_rows.len(), 1);
    assert_eq!(rf9_rows.len(), 1);
    assert_eq!(rf8_rows[0].email, "alice@example.com");
    assert_eq!(rf9_rows[0].email, "alice@example.com");
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p repolyze-store rf8_and_rf9_queries_return_snapshot_scoped_rows -- --exact`

Expected: FAIL because the read queries and row models do not exist yet.

**Step 3: Write the minimal implementation**

Add typed row models to `crates/repolyze-store/src/models.rs` that match the RF-8 and RF-9 output contracts from `docs/plans/RF-8-9.md`.

```rust
pub struct UsersContributionRowRecord {
    pub email: String,
    pub commits: i64,
    pub lines_modified: i64,
    pub lines_per_commit: f64,
    pub files_touched: i64,
    pub most_active_week_day: String,
}

pub struct UserActivityRowRecord {
    pub email: String,
    pub most_active_week_day: String,
    pub average_commits_per_day_in_most_active_day: f64,
    pub average_commits_per_day: f64,
    pub average_commits_per_hour_in_most_active_hour: f64,
    pub average_commits_per_hour: f64,
}
```

Implement SQL queries in `crates/repolyze-store/src/sqlite.rs` that:

- join contributor summaries with contributor emails
- join weekday/hour stats to compute averages
- filter by a list of snapshot ids
- return rows sorted by commits descending then email ascending

Do not move all RF-8/RF-9 presentation logic into SQL; use SQL to fetch the facts and do the final small calculations in Rust if it keeps the query readable.

**Step 4: Run the test to verify it passes**

Run: `cargo test -p repolyze-store rf8_rf9_queries`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/repolyze-store/src/models.rs crates/repolyze-store/src/sqlite.rs crates/repolyze-store/tests/rf8_rf9_queries.rs
git commit -m "feat: add rf-8 and rf-9 snapshot queries"
```

### Task 10: Run full verification and document follow-up work

**Files:**
- Modify: any files touched above if verification reveals issues
- Test: workspace

**Step 1: Run targeted crate tests**

Run:

```bash
cargo test -p repolyze-store
cargo test -p repolyze-git
cargo test -p repolyze-core
cargo test -p repolyze-cli analyze_cli
cargo test -p repolyze-tui
```

Expected: PASS.

**Step 2: Run the full test suite**

Run: `cargo test --workspace`

Expected: PASS.

**Step 3: Run the full verification workflow**

Run: `cargo xtask verify`

Expected: PASS for format check, clippy, tests, and build.

**Step 4: Request code review**

Use `@requesting-code-review` after verification succeeds.

**Step 5: Commit any final cleanups**

```bash
git add Cargo.toml crates/repolyze-store/Cargo.toml crates/repolyze-store/src/lib.rs crates/repolyze-store/src/error.rs crates/repolyze-store/src/path.rs crates/repolyze-store/src/migrations.rs crates/repolyze-store/src/models.rs crates/repolyze-store/src/sqlite.rs crates/repolyze-store/tests/path_bootstrap.rs crates/repolyze-store/tests/migrations.rs crates/repolyze-store/tests/raw_git_facts.rs crates/repolyze-store/tests/snapshot_aggregates.rs crates/repolyze-store/tests/cache_roundtrip.rs crates/repolyze-store/tests/rf8_rf9_queries.rs crates/repolyze-git/src/lib.rs crates/repolyze-git/src/backend.rs crates/repolyze-git/src/repository.rs crates/repolyze-git/src/contributions.rs crates/repolyze-git/src/activity.rs crates/repolyze-git/tests/git_fixture.rs crates/repolyze-core/src/error.rs crates/repolyze-core/src/service.rs crates/repolyze-cli/Cargo.toml crates/repolyze-cli/src/run.rs crates/repolyze-cli/tests/analyze_cli.rs crates/repolyze-tui/Cargo.toml crates/repolyze-tui/src/lib.rs
git commit -m "feat: add sqlite-backed analytics cache"
```

## Expected User-Facing Outcome

- Running `repolyze analyze` or `repolyze compare` creates `~/.repolyze/repolyze.db` automatically.
- Re-analyzing a repository at the same `HEAD` reuses the cached snapshot instead of rerunning live Git analysis.
- The database keeps historical scan metadata and immutable snapshots.
- RF-8 and RF-9 can later read contributor rows from SQLite without reparsing Git history.
