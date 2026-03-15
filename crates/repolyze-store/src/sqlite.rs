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
        &mut self,
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

    pub fn upsert_contributor(&mut self, record: &ContributorRecord) -> Result<i64, StoreError> {
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
        &mut self,
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
}

fn now_iso() -> String {
    // Simple UTC timestamp without external dependency
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
