use std::path::Path;

use rusqlite::{Connection, params};

use crate::error::StoreError;
use crate::migrations::{MIGRATION_V1, SCHEMA_VERSION};
use crate::models::{CommitFileChangeRecord, CommitRecord, ContributorRecord};

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
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn upsert_repository(
        &self,
        canonical_path: &str,
        display_name: &str,
    ) -> Result<i64, StoreError> {
        let now = now_iso();
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
        let now = now_iso();
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
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM repository_commits WHERE repository_id = ?1 AND commit_hash = ?2",
                params![commit.repository_id, commit.commit_hash],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            return Ok(id);
        }

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
        let now = now_iso();
        self.conn.execute(
            "INSERT INTO analysis_snapshots (repository_id, history_scope, head_commit_hash, branch_name, analysis_period_start_at, analysis_period_end_at, commits_count, contributors_count, analysis_payload_json, snapshot_created_at, repolyze_version, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
        Ok(self.conn.last_insert_rowid())
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

    pub fn users_contribution_rows_for_snapshots(
        &self,
        snapshot_ids: &[i64],
    ) -> Result<Vec<crate::models::UsersContributionRowRecord>, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT c.canonical_email, s.commits_count, s.lines_modified, s.files_touched_count, s.most_active_weekday
             FROM snapshot_contributor_summaries s
             JOIN contributors c ON c.id = s.contributor_id
             WHERE s.snapshot_id IN ({placeholders})
             ORDER BY s.commits_count DESC, c.canonical_email ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let email: String = row.get(0)?;
            let commits: i64 = row.get(1)?;
            let lines_modified: i64 = row.get(2)?;
            let files_touched: i64 = row.get(3)?;
            let most_active_weekday: Option<i64> = row.get(4)?;
            let lines_per_commit = if commits > 0 {
                lines_modified as f64 / commits as f64
            } else {
                0.0
            };
            let weekday_name = most_active_weekday
                .map(weekday_name_from_number)
                .unwrap_or_else(|| "N/A".to_string());
            Ok(crate::models::UsersContributionRowRecord {
                email,
                commits,
                lines_modified,
                lines_per_commit,
                files_touched,
                most_active_week_day: weekday_name,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn user_activity_rows_for_snapshots(
        &self,
        snapshot_ids: &[i64],
    ) -> Result<Vec<crate::models::UserActivityRowRecord>, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        // Get summary + weekday/hour stats per contributor
        let sql = format!(
            "SELECT c.canonical_email, s.commits_count, s.active_days_count, s.most_active_weekday, s.most_active_hour
             FROM snapshot_contributor_summaries s
             JOIN contributors c ON c.id = s.contributor_id
             WHERE s.snapshot_id IN ({placeholders})
             ORDER BY s.commits_count DESC, c.canonical_email ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let email: String = row.get(0)?;
            let commits_count: i64 = row.get(1)?;
            let active_days_count: i64 = row.get(2)?;
            let most_active_weekday: Option<i64> = row.get(3)?;
            let most_active_hour: Option<i64> = row.get(4)?;
            Ok((
                email,
                commits_count,
                active_days_count,
                most_active_weekday,
                most_active_hour,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (email, commits_count, active_days_count, most_active_weekday, most_active_hour) =
                row?;

            let weekday_name = most_active_weekday
                .map(weekday_name_from_number)
                .unwrap_or_else(|| "N/A".to_string());

            // Get most active weekday commits
            let most_active_weekday_commits = if let Some(wd) = most_active_weekday {
                self.weekday_commits_for_contributor(&email, snapshot_ids, wd)?
            } else {
                0
            };
            let most_active_weekday_dates = if let Some(wd) = most_active_weekday {
                self.weekday_dates_for_contributor(&email, snapshot_ids, wd)?
            } else {
                0
            };

            // Get most active hour commits
            let most_active_hour_commits = if let Some(h) = most_active_hour {
                self.hour_commits_for_contributor(&email, snapshot_ids, h)?
            } else {
                0
            };
            let most_active_hour_buckets = if let Some(h) = most_active_hour {
                self.hour_buckets_for_contributor(&email, snapshot_ids, h)?
            } else {
                0
            };

            // Total hour stats for average
            let total_hour_buckets: i64 =
                self.total_hour_buckets_for_contributor(&email, snapshot_ids)?;

            let average_commits_per_day = if active_days_count > 0 {
                commits_count as f64 / active_days_count as f64
            } else {
                0.0
            };

            let average_commits_per_day_in_most_active_day = if most_active_weekday_dates > 0 {
                most_active_weekday_commits as f64 / most_active_weekday_dates as f64
            } else {
                0.0
            };

            let average_commits_per_hour = if total_hour_buckets > 0 {
                commits_count as f64 / total_hour_buckets as f64
            } else {
                0.0
            };

            let average_commits_per_hour_in_most_active_hour = if most_active_hour_buckets > 0 {
                most_active_hour_commits as f64 / most_active_hour_buckets as f64
            } else {
                0.0
            };

            result.push(crate::models::UserActivityRowRecord {
                email,
                most_active_week_day: weekday_name,
                average_commits_per_day_in_most_active_day,
                average_commits_per_day,
                average_commits_per_hour_in_most_active_hour,
                average_commits_per_hour,
            });
        }

        Ok(result)
    }

    fn weekday_commits_for_contributor(
        &self,
        email: &str,
        snapshot_ids: &[i64],
        weekday: i64,
    ) -> Result<i64, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(w.commits_count), 0)
             FROM snapshot_contributor_weekday_stats w
             JOIN contributors c ON c.id = w.contributor_id
             WHERE w.snapshot_id IN ({placeholders})
               AND c.canonical_email = ?1
               AND w.weekday = ?2"
        );
        let count = self
            .conn
            .query_row(&sql, params![email, weekday], |row| row.get(0))?;
        Ok(count)
    }

    fn weekday_dates_for_contributor(
        &self,
        email: &str,
        snapshot_ids: &[i64],
        weekday: i64,
    ) -> Result<i64, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(w.active_dates_count), 0)
             FROM snapshot_contributor_weekday_stats w
             JOIN contributors c ON c.id = w.contributor_id
             WHERE w.snapshot_id IN ({placeholders})
               AND c.canonical_email = ?1
               AND w.weekday = ?2"
        );
        let count = self
            .conn
            .query_row(&sql, params![email, weekday], |row| row.get(0))?;
        Ok(count)
    }

    fn hour_commits_for_contributor(
        &self,
        email: &str,
        snapshot_ids: &[i64],
        hour: i64,
    ) -> Result<i64, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(h.commits_count), 0)
             FROM snapshot_contributor_hour_stats h
             JOIN contributors c ON c.id = h.contributor_id
             WHERE h.snapshot_id IN ({placeholders})
               AND c.canonical_email = ?1
               AND h.hour_of_day = ?2"
        );
        let count = self
            .conn
            .query_row(&sql, params![email, hour], |row| row.get(0))?;
        Ok(count)
    }

    fn hour_buckets_for_contributor(
        &self,
        email: &str,
        snapshot_ids: &[i64],
        hour: i64,
    ) -> Result<i64, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(h.active_hour_buckets_count), 0)
             FROM snapshot_contributor_hour_stats h
             JOIN contributors c ON c.id = h.contributor_id
             WHERE h.snapshot_id IN ({placeholders})
               AND c.canonical_email = ?1
               AND h.hour_of_day = ?2"
        );
        let count = self
            .conn
            .query_row(&sql, params![email, hour], |row| row.get(0))?;
        Ok(count)
    }

    fn total_hour_buckets_for_contributor(
        &self,
        email: &str,
        snapshot_ids: &[i64],
    ) -> Result<i64, StoreError> {
        let placeholders = snapshot_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COALESCE(SUM(h.active_hour_buckets_count), 0)
             FROM snapshot_contributor_hour_stats h
             JOIN contributors c ON c.id = h.contributor_id
             WHERE h.snapshot_id IN ({placeholders})
               AND c.canonical_email = ?1"
        );
        let count = self
            .conn
            .query_row(&sql, params![email], |row| row.get(0))?;
        Ok(count)
    }
}

impl repolyze_core::service::AnalysisStore for SqliteStore {
    fn load_snapshot(
        &self,
        key: &repolyze_core::service::RepositoryCacheMetadata,
    ) -> Result<Option<repolyze_core::model::RepositoryAnalysis>, repolyze_core::error::RepolyzeError>
    {
        let canonical_path = key.repository_root.to_string_lossy();
        let repo_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM repositories WHERE canonical_path = ?1",
                params![canonical_path.as_ref()],
                |row| row.get(0),
            )
            .ok();

        let Some(repo_id) = repo_id else {
            return Ok(None);
        };

        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT analysis_payload_json FROM analysis_snapshots
                 WHERE repository_id = ?1 AND history_scope = ?2 AND head_commit_hash = ?3
                 AND is_complete = 1
                 ORDER BY snapshot_created_at DESC LIMIT 1",
                params![repo_id, key.history_scope, key.head_commit_hash],
                |row| row.get(0),
            )
            .ok();

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

        let repo_id = self
            .upsert_repository(&canonical_path, &display_name)
            .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        self.insert_snapshot_header(
            repo_id,
            &key.history_scope,
            &key.head_commit_hash,
            key.branch_name.as_deref(),
            None,
            None,
            analysis.contributions.total_commits as i64,
            analysis.contributions.contributors.len() as i64,
            &json,
            env!("CARGO_PKG_VERSION"),
        )
        .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        Ok(())
    }

    fn record_scan_failure(
        &self,
        repository_root: &std::path::Path,
        reason: &str,
    ) -> Result<(), repolyze_core::error::RepolyzeError> {
        let canonical_path = repository_root.to_string_lossy().to_string();
        let display_name = repository_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical_path.clone());

        let repo_id = self
            .upsert_repository(&canonical_path, &display_name)
            .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        let now = now_iso();
        self.conn
            .execute(
                "INSERT INTO scan_runs (repository_id, trigger_source, cache_status, started_at, status, failure_reason)
                 VALUES (?1, 'cli', 'miss', ?2, 'failed', ?3)",
                params![repo_id, now, reason],
            )
            .map_err(|e| repolyze_core::error::RepolyzeError::Store(e.to_string()))?;

        Ok(())
    }
}

fn now_iso() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn weekday_name_from_number(weekday: i64) -> String {
    match weekday {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
    .to_string()
}
