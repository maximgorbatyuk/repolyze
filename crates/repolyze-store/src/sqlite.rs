use std::path::Path;

use rusqlite::{Connection, Error as SqliteError, params};
use serde_json::from_str;

use crate::error::StoreError;
use crate::migrations::SCHEMA_VERSION;
use crate::models::{
    CommitFileChangeRecord, CommitRecord, ContributionRowRecord, ContributorRecord,
    UserActivityRowRecord,
};

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;

        // FK enforcement is per-connection, set unconditionally before migrations
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        // Version-aware migration: apply each migration newer than the current version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        for &(version, sql) in crate::migrations::MIGRATIONS {
            if current_version < version {
                conn.execute_batch(sql)?;
            }
        }
        let target = crate::migrations::MIGRATIONS
            .last()
            .map(|(v, _)| *v)
            .unwrap_or(0);
        conn.pragma_update(None, "user_version", target)?;

        Ok(Self { conn })
    }

    pub fn open_default() -> Result<Self, StoreError> {
        let db_path = crate::path::resolve_database_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::open(&db_path)
    }

    pub fn table_names(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn database_metadata(&self) -> Result<crate::models::DatabaseMetadata, StoreError> {
        let tables = self.table_names()?;
        let mut counts: Vec<(String, i64)> = Vec::new();
        let mut total: i64 = 0;

        for t in &tables {
            if t.starts_with("sqlite_") {
                continue;
            }
            let count: i64 =
                self.conn
                    .query_row(&format!("SELECT COUNT(*) FROM \"{t}\""), [], |row| {
                        row.get(0)
                    })?;
            total += count;
            counts.push((t.clone(), count));
        }

        let mut rows: Vec<crate::models::TableRowCount> = counts
            .into_iter()
            .map(|(name, count)| {
                let percentage = if total > 0 {
                    (count as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                crate::models::TableRowCount {
                    table_name: name,
                    record_count: count,
                    percentage,
                }
            })
            .collect();

        rows.sort_by(|a, b| {
            b.record_count
                .cmp(&a.record_count)
                .then(a.table_name.cmp(&b.table_name))
        });

        Ok(crate::models::DatabaseMetadata {
            tables: rows,
            total_rows: total,
        })
    }

    pub fn upsert_repository(
        &self,
        canonical_path: &str,
        display_name: &str,
    ) -> Result<i64, StoreError> {
        let now = now_unix_secs();
        self.conn.execute(
            "INSERT INTO repositories (canonical_path, display_name, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(canonical_path) DO UPDATE SET
               display_name = excluded.display_name,
               last_seen_at = excluded.last_seen_at",
            params![canonical_path, display_name, now, now],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM repositories WHERE canonical_path = ?1",
            params![canonical_path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn upsert_contributor(&self, record: &ContributorRecord) -> Result<i64, StoreError> {
        let now = now_unix_secs();
        self.conn.execute(
            "INSERT INTO contributors (canonical_email, display_name_last_seen, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(canonical_email) DO UPDATE SET
               display_name_last_seen = excluded.display_name_last_seen,
               last_seen_at = excluded.last_seen_at",
            params![record.canonical_email, record.display_name_last_seen, now, now],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM contributors WHERE canonical_email = ?1",
            params![record.canonical_email],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn upsert_commit(
        &self,
        commit: &CommitRecord,
        file_changes: &[CommitFileChangeRecord],
    ) -> Result<i64, StoreError> {
        // Check if commit already exists
        let existing: Option<i64> = match self.conn.query_row(
            "SELECT id FROM repository_commits WHERE repository_id = ?1 AND commit_hash = ?2",
            params![commit.repository_id, commit.commit_hash],
            |row| row.get(0),
        ) {
            Ok(val) => Some(val),
            Err(SqliteError::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };

        if let Some(id) = existing {
            return Ok(id);
        }

        // Wrap commit + file changes in a transaction
        self.conn.execute_batch("BEGIN")?;

        let result = (|| -> Result<i64, StoreError> {
            self.conn.execute(
                "INSERT INTO repository_commits (repository_id, contributor_id, commit_hash, author_name, author_email, committed_at, commit_date, commit_hour, commit_weekday, files_changed_count, lines_added, lines_deleted, lines_modified)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    commit.repository_id,
                    commit.contributor_id,
                    commit.commit_hash,
                    commit.author_name,
                    commit.author_email,
                    commit.committed_at,
                    commit.commit_date,
                    commit.commit_hour,
                    commit.commit_weekday,
                    commit.files_changed_count,
                    commit.lines_added,
                    commit.lines_deleted,
                    commit.lines_modified,
                ],
            )?;
            let commit_id = self.conn.last_insert_rowid();

            for fc in file_changes {
                self.conn.execute(
                    "INSERT INTO commit_file_changes (commit_id, file_path, additions, deletions, lines_modified)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![commit_id, fc.file_path, fc.additions, fc.deletions, fc.lines_modified],
                )?;
            }

            Ok(commit_id)
        })();

        match &result {
            Ok(_) => {
                if let Err(e) = self.conn.execute_batch("COMMIT") {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    return Err(e.into());
                }
            }
            Err(_) => {
                let _ = self.conn.execute_batch("ROLLBACK");
            }
        }

        result
    }

    pub fn commit_count(&self, repository_id: i64) -> Result<i64, StoreError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM repository_commits WHERE repository_id = ?1",
            params![repository_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_snapshot_header(
        &self,
        repository_id: i64,
        history_scope: &str,
        head_commit_hash: &str,
        branch_name: Option<&str>,
        analysis_period_start_at: Option<&str>,
        analysis_period_end_at: Option<&str>,
        commits_count: i64,
        contributors_count: i64,
        analysis_payload_json: &str,
        repolyze_version: &str,
    ) -> Result<i64, StoreError> {
        let now = now_unix_secs();
        self.conn.execute(
            "INSERT INTO analysis_snapshots (repository_id, history_scope, head_commit_hash, branch_name, analysis_period_start_at, analysis_period_end_at, commits_count, contributors_count, analysis_payload_json, snapshot_created_at, repolyze_version, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(repository_id, history_scope, head_commit_hash) DO UPDATE SET
               branch_name = excluded.branch_name,
               analysis_period_start_at = excluded.analysis_period_start_at,
               analysis_period_end_at = excluded.analysis_period_end_at,
               commits_count = excluded.commits_count,
               contributors_count = excluded.contributors_count,
               analysis_payload_json = excluded.analysis_payload_json,
               snapshot_created_at = excluded.snapshot_created_at,
               repolyze_version = excluded.repolyze_version,
               schema_version = excluded.schema_version,
               is_complete = 1",
            params![
                repository_id,
                history_scope,
                head_commit_hash,
                branch_name,
                analysis_period_start_at,
                analysis_period_end_at,
                commits_count,
                contributors_count,
                analysis_payload_json,
                now,
                repolyze_version,
                crate::migrations::SCHEMA_VERSION,
            ],
        )?;
        let snapshot_id = self.conn.query_row(
            "SELECT id FROM analysis_snapshots WHERE repository_id = ?1 AND history_scope = ?2 AND head_commit_hash = ?3",
            params![repository_id, history_scope, head_commit_hash],
            |row| row.get(0),
        )?;
        Ok(snapshot_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_snapshot_contributor_summary(
        &self,
        snapshot_id: i64,
        contributor_id: i64,
        commits_count: i64,
        lines_added: i64,
        lines_deleted: i64,
        lines_modified: i64,
        files_touched_count: i64,
        active_days_count: i64,
        first_commit_at: &str,
        last_commit_at: &str,
        most_active_weekday: Option<i64>,
        most_active_hour: Option<i64>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshot_contributor_summaries (snapshot_id, contributor_id, commits_count, lines_added, lines_deleted, lines_modified, files_touched_count, active_days_count, first_commit_at, last_commit_at, most_active_weekday, most_active_hour)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                snapshot_id,
                contributor_id,
                commits_count,
                lines_added,
                lines_deleted,
                lines_modified,
                files_touched_count,
                active_days_count,
                first_commit_at,
                last_commit_at,
                most_active_weekday,
                most_active_hour,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_snapshot_contributor_weekday_stat(
        &self,
        snapshot_id: i64,
        contributor_id: i64,
        weekday: i64,
        commits_count: i64,
        active_dates_count: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshot_contributor_weekday_stats (snapshot_id, contributor_id, weekday, commits_count, active_dates_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![snapshot_id, contributor_id, weekday, commits_count, active_dates_count],
        )?;
        Ok(())
    }

    pub fn upsert_snapshot_contributor_hour_stat(
        &self,
        snapshot_id: i64,
        contributor_id: i64,
        hour_of_day: i64,
        commits_count: i64,
        active_hour_buckets_count: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshot_contributor_hour_stats (snapshot_id, contributor_id, hour_of_day, commits_count, active_hour_buckets_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![snapshot_id, contributor_id, hour_of_day, commits_count, active_hour_buckets_count],
        )?;
        Ok(())
    }

    pub fn snapshot_summary_row_count(&self, snapshot_id: i64) -> Result<i64, StoreError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM snapshot_contributor_summaries WHERE snapshot_id = ?1",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn snapshot_weekday_row_count(&self, snapshot_id: i64) -> Result<i64, StoreError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM snapshot_contributor_weekday_stats WHERE snapshot_id = ?1",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn snapshot_hour_row_count(&self, snapshot_id: i64) -> Result<i64, StoreError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM snapshot_contributor_hour_stats WHERE snapshot_id = ?1",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn contribution_rows_for_snapshots(
        &self,
        snapshot_ids: &[i64],
    ) -> Result<Vec<ContributionRowRecord>, StoreError> {
        let analyses = self.load_analyses_for_snapshot_ids(snapshot_ids)?;
        let no_settings = repolyze_core::settings::Settings::default();
        Ok(
            repolyze_core::analytics::build_contribution_rows(&analyses, &no_settings)
                .into_iter()
                .map(|row| ContributionRowRecord {
                    email: row.identifier,
                    commits: row.commits as i64,
                    lines_modified: row.lines_modified as i64,
                    lines_per_commit: row.lines_per_commit,
                    files_touched: row.files_touched as i64,
                })
                .collect(),
        )
    }

    pub fn user_activity_rows_for_snapshots(
        &self,
        snapshot_ids: &[i64],
    ) -> Result<Vec<UserActivityRowRecord>, StoreError> {
        let analyses = self.load_analyses_for_snapshot_ids(snapshot_ids)?;
        let no_settings = repolyze_core::settings::Settings::default();
        Ok(
            repolyze_core::analytics::build_user_activity_rows(&analyses, &no_settings)
                .into_iter()
                .map(|row| UserActivityRowRecord {
                    email: row.identifier,
                    most_active_week_day: row.most_active_week_day,
                    average_commits_per_day_in_most_active_day: row
                        .average_commits_per_day_in_most_active_day,
                    average_commits_per_day: row.average_commits_per_day,
                    average_commits_per_hour_in_most_active_hour: row
                        .average_commits_per_hour_in_most_active_hour,
                    average_commits_per_hour: row.average_commits_per_hour,
                })
                .collect(),
        )
    }

    fn load_analyses_for_snapshot_ids(
        &self,
        snapshot_ids: &[i64],
    ) -> Result<Vec<repolyze_core::model::RepositoryAnalysis>, StoreError> {
        if snapshot_ids.is_empty() {
            return Ok(Vec::new());
        }

        let in_clause = parameterized_in_clause(snapshot_ids.len());
        let sql = format!(
            "SELECT analysis_payload_json FROM analysis_snapshots WHERE id IN ({in_clause}) ORDER BY id"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params = rusqlite::params_from_iter(snapshot_ids.iter());
        let rows = stmt.query_map(params, |row| row.get::<_, String>(0))?;

        rows.map(|row| {
            let json = row?;
            from_str::<repolyze_core::model::RepositoryAnalysis>(&json).map_err(|e| {
                StoreError::Serialization(format!("failed to deserialize snapshot payload: {e}"))
            })
        })
        .collect()
    }
}

impl repolyze_core::service::AnalysisStore for SqliteStore {
    fn load_snapshot(
        &self,
        key: &repolyze_core::service::RepositoryCacheMetadata,
    ) -> Result<Option<repolyze_core::model::RepositoryAnalysis>, repolyze_core::error::RepolyzeError>
    {
        let canonical_path = key.repository_root.to_string_lossy();
        let repo_id: Option<i64> = match self.conn.query_row(
            "SELECT id FROM repositories WHERE canonical_path = ?1",
            params![canonical_path.as_ref()],
            |row| row.get(0),
        ) {
            Ok(val) => Some(val),
            Err(SqliteError::QueryReturnedNoRows) => None,
            Err(e) => return Err(repolyze_core::error::RepolyzeError::Store(e.to_string())),
        };

        let Some(repo_id) = repo_id else {
            return Ok(None);
        };

        let result: Option<String> = match self.conn.query_row(
            "SELECT analysis_payload_json FROM analysis_snapshots
                 WHERE repository_id = ?1 AND history_scope = ?2 AND head_commit_hash = ?3
                 AND repolyze_version = ?4 AND schema_version = ?5
                 AND is_complete = 1
                 ORDER BY snapshot_created_at DESC LIMIT 1",
            params![
                repo_id,
                key.history_scope,
                key.head_commit_hash,
                env!("CARGO_PKG_VERSION"),
                SCHEMA_VERSION,
            ],
            |row| row.get(0),
        ) {
            Ok(val) => Some(val),
            Err(SqliteError::QueryReturnedNoRows) => None,
            Err(e) => return Err(repolyze_core::error::RepolyzeError::Store(e.to_string())),
        };

        match result {
            Some(json) => {
                let analysis: repolyze_core::model::RepositoryAnalysis =
                    serde_json::from_str(&json).map_err(|e| {
                        repolyze_core::error::RepolyzeError::Store(format!(
                            "failed to deserialize cached analysis: {e}"
                        ))
                    })?;
                Ok(Some(analysis))
            }
            None => Ok(None),
        }
    }

    fn save_snapshot(
        &self,
        key: &repolyze_core::service::RepositoryCacheMetadata,
        analysis: &repolyze_core::model::RepositoryAnalysis,
    ) -> Result<(), repolyze_core::error::RepolyzeError> {
        let canonical_path = key.repository_root.to_string_lossy().to_string();
        let display_name = key
            .repository_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical_path.clone());

        let json = serde_json::to_string(analysis).map_err(|e| {
            repolyze_core::error::RepolyzeError::Store(format!("failed to serialize analysis: {e}"))
        })?;

        // Wrap upsert_repository + insert_snapshot_header in a transaction
        self.conn
            .execute_batch("BEGIN")
            .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        let result = (|| -> Result<(), repolyze_core::error::RepolyzeError> {
            let repo_id = self
                .upsert_repository(&canonical_path, &display_name)
                .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

            let analysis_period_start_at = analysis
                .contributions
                .contributors
                .iter()
                .map(|c| c.first_commit.as_str())
                .filter(|value| !value.is_empty())
                .min()
                .map(str::to_string);
            let analysis_period_end_at = analysis
                .contributions
                .contributors
                .iter()
                .map(|c| c.last_commit.as_str())
                .filter(|value| !value.is_empty())
                .max()
                .map(str::to_string);

            let snapshot_id = self
                .insert_snapshot_header(
                    repo_id,
                    &key.history_scope,
                    &key.head_commit_hash,
                    key.branch_name.as_deref(),
                    analysis_period_start_at.as_deref(),
                    analysis_period_end_at.as_deref(),
                    analysis.contributions.total_commits as i64,
                    analysis.contributions.contributors.len() as i64,
                    &json,
                    env!("CARGO_PKG_VERSION"),
                )
                .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

            self.conn
                .execute(
                    "DELETE FROM snapshot_contributor_summaries WHERE snapshot_id = ?1",
                    params![snapshot_id],
                )
                .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;
            self.conn
                .execute(
                    "DELETE FROM snapshot_contributor_weekday_stats WHERE snapshot_id = ?1",
                    params![snapshot_id],
                )
                .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;
            self.conn
                .execute(
                    "DELETE FROM snapshot_contributor_hour_stats WHERE snapshot_id = ?1",
                    params![snapshot_id],
                )
                .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

            let activity_by_email: std::collections::HashMap<_, _> = analysis
                .contributions
                .activity_by_contributor
                .iter()
                .map(|activity| (activity.email.to_lowercase(), activity))
                .collect();

            for contributor in &analysis.contributions.contributors {
                let contributor_id = self
                    .upsert_contributor(&ContributorRecord::new(
                        &contributor.email,
                        &contributor.name,
                    ))
                    .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

                let activity = activity_by_email
                    .get(&contributor.email.to_lowercase())
                    .copied();
                let most_active_weekday =
                    activity.map(|entry| most_active_index(&entry.weekday_commits) as i64);
                let most_active_hour =
                    activity.map(|entry| most_active_index(&entry.hour_commits) as i64);

                self.upsert_snapshot_contributor_summary(
                    snapshot_id,
                    contributor_id,
                    contributor.commits as i64,
                    contributor.lines_added as i64,
                    contributor.lines_deleted as i64,
                    (contributor.lines_added + contributor.lines_deleted) as i64,
                    contributor.files_touched as i64,
                    contributor.active_days as i64,
                    &contributor.first_commit,
                    &contributor.last_commit,
                    most_active_weekday,
                    most_active_hour,
                )
                .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

                if let Some(activity) = activity {
                    for (weekday, commits_count) in activity.weekday_commits.iter().enumerate() {
                        if *commits_count == 0 {
                            continue;
                        }
                        self.upsert_snapshot_contributor_weekday_stat(
                            snapshot_id,
                            contributor_id,
                            weekday as i64,
                            *commits_count as i64,
                            activity.active_dates_by_weekday[weekday].len() as i64,
                        )
                        .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;
                    }

                    for (hour, commits_count) in activity.hour_commits.iter().enumerate() {
                        if *commits_count == 0 {
                            continue;
                        }
                        self.upsert_snapshot_contributor_hour_stat(
                            snapshot_id,
                            contributor_id,
                            hour as i64,
                            *commits_count as i64,
                            activity.active_hour_buckets_by_hour[hour].len() as i64,
                        )
                        .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;
                    }
                }
            }

            Ok(())
        })();

        match &result {
            Ok(()) => {
                if let Err(e) = self.conn.execute_batch("COMMIT") {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    return Err(repolyze_core::error::RepolyzeError::Store(e.to_string()));
                }
            }
            Err(_) => {
                let _ = self.conn.execute_batch("ROLLBACK");
            }
        }

        result
    }

    fn record_scan_result(
        &self,
        key: Option<&repolyze_core::service::RepositoryCacheMetadata>,
        repository_identifier: &str,
        trigger_source: &str,
        cache_status: &str,
        status: &str,
        failure_reason: Option<&str>,
    ) -> Result<(), repolyze_core::error::RepolyzeError> {
        let canonical_path = repository_identifier.to_string();
        let display_name = if repository_identifier.contains("github.com/") {
            // For GitHub URLs, extract "owner/repo" as the display name
            repository_identifier
                .rsplit("github.com/")
                .next()
                .map(|s| s.trim_end_matches('/').to_string())
                .unwrap_or_else(|| canonical_path.clone())
        } else {
            std::path::Path::new(repository_identifier)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| canonical_path.clone())
        };

        let repo_id = self
            .upsert_repository(&canonical_path, &display_name)
            .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        let snapshot_id = match key {
            Some(metadata) => match self.conn.query_row(
                "SELECT id FROM analysis_snapshots WHERE repository_id = ?1 AND history_scope = ?2 AND head_commit_hash = ?3 ORDER BY snapshot_created_at DESC LIMIT 1",
                params![repo_id, metadata.history_scope, metadata.head_commit_hash],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(val) => Some(val),
                Err(SqliteError::QueryReturnedNoRows) => None,
                Err(e) => return Err(repolyze_core::error::RepolyzeError::Store(e.to_string())),
            },
            None => None,
        };

        let now = now_unix_secs();
        self.conn
            .execute(
                "INSERT INTO scan_runs (repository_id, snapshot_id, trigger_source, cache_status, started_at, finished_at, status, failure_reason)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![repo_id, snapshot_id, trigger_source, cache_status, now, now, status, failure_reason],
            )
            .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        Ok(())
    }
}

/// Generate parameterized placeholders for an IN clause: "?1,?2,?3"
fn parameterized_in_clause(count: usize) -> String {
    debug_assert!(count > 0, "parameterized_in_clause called with count=0");
    (1..=count)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn now_unix_secs() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn most_active_index(values: &[u32]) -> usize {
    values
        .iter()
        .enumerate()
        .max_by_key(|(_, value)| *value)
        .map(|(index, _)| index)
        .unwrap_or(0)
}
