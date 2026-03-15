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
