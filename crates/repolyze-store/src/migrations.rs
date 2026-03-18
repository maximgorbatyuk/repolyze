pub const MIGRATIONS: &[(i32, &str)] = &[(1, MIGRATION_V1)];

// Bump this to invalidate cached analysis_payload_json snapshots.
// No SQL migration needed — extension data lives inside the JSON blob.
// v2: initial schema, v3: added file_extensions to ContributorStats.
pub const SCHEMA_VERSION: i32 = 3;

const MIGRATION_V1: &str = r#"
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
  repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
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
  repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
  snapshot_id INTEGER REFERENCES analysis_snapshots(id) ON DELETE CASCADE,
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

CREATE TABLE IF NOT EXISTS contributors (
  id INTEGER PRIMARY KEY,
  canonical_email TEXT NOT NULL UNIQUE,
  display_name_last_seen TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repository_commits (
  id INTEGER PRIMARY KEY,
  repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
  contributor_id INTEGER NOT NULL REFERENCES contributors(id) ON DELETE CASCADE,
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
  commit_id INTEGER NOT NULL REFERENCES repository_commits(id) ON DELETE CASCADE,
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

CREATE TABLE IF NOT EXISTS snapshot_commits (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id) ON DELETE CASCADE,
  commit_id INTEGER NOT NULL REFERENCES repository_commits(id) ON DELETE CASCADE,
  PRIMARY KEY (snapshot_id, commit_id)
);

CREATE TABLE IF NOT EXISTS snapshot_contributor_summaries (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id) ON DELETE CASCADE,
  contributor_id INTEGER NOT NULL REFERENCES contributors(id) ON DELETE CASCADE,
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
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id) ON DELETE CASCADE,
  contributor_id INTEGER NOT NULL REFERENCES contributors(id) ON DELETE CASCADE,
  weekday INTEGER NOT NULL,
  commits_count INTEGER NOT NULL,
  active_dates_count INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id, contributor_id, weekday)
);

CREATE TABLE IF NOT EXISTS snapshot_contributor_hour_stats (
  snapshot_id INTEGER NOT NULL REFERENCES analysis_snapshots(id) ON DELETE CASCADE,
  contributor_id INTEGER NOT NULL REFERENCES contributors(id) ON DELETE CASCADE,
  hour_of_day INTEGER NOT NULL,
  commits_count INTEGER NOT NULL,
  active_hour_buckets_count INTEGER NOT NULL,
  PRIMARY KEY (snapshot_id, contributor_id, hour_of_day)
);

CREATE INDEX IF NOT EXISTS idx_snapshot_contributor_summaries_contributor
  ON snapshot_contributor_summaries (contributor_id);
"#;
