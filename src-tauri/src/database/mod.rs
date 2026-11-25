//! SQLite connection setup and versioned migrations.

use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::error::AppResult;

pub const DB_FILE_NAME: &str = "labforge.db";

/// Open (creating if needed) the database file at `path` and bring it to the latest schema.
pub async fn connect(path: &Path) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // Foreign keys are off by default in SQLite; the schema relies on them.
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Open the database inside `data_dir`, creating the directory first.
pub async fn open_in_dir(data_dir: &Path) -> AppResult<SqlitePool> {
    std::fs::create_dir_all(data_dir)?;
    connect(&data_dir.join(DB_FILE_NAME)).await
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use tempfile::TempDir;

    /// A real on-disk database in a temp directory that is removed on drop.
    pub struct TestDb {
        pub pool: SqlitePool,
        pub dir: TempDir,
    }

    pub async fn fresh() -> TestDb {
        let dir = TempDir::new().expect("temp dir");
        let pool = open_in_dir(dir.path()).await.expect("open test db");
        TestDb { pool, dir }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_support::fresh;

    #[tokio::test]
    async fn fresh_database_applies_all_migrations() {
        let db = fresh().await;
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE '\\_%' ESCAPE '\\' \
             AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&db.pool)
        .await
        .unwrap();
        let names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        for expected in [
            "attempt_steps",
            "attempts",
            "challenge_evidence",
            "challenge_steps",
            "challenges",
            "notes",
            "step_choices",
            "step_hints",
        ] {
            assert!(
                names.contains(&expected),
                "missing table {expected}: {names:?}"
            );
        }
    }

    #[tokio::test]
    async fn reopening_an_existing_database_is_a_no_op() {
        let db = fresh().await;
        sqlx::query(
            "INSERT INTO challenges (slug, version, source, title, summary, difficulty, objectives_md) \
             VALUES ('x', 1, 'custom', 'T', 'S', 'beginner', '')",
        )
        .execute(&db.pool)
        .await
        .unwrap();
        db.pool.close().await;

        let reopened = open_in_dir(db.dir.path()).await.unwrap();
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM challenges")
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced_on_every_connection() {
        let db = fresh().await;
        let result =
            sqlx::query("INSERT INTO attempts (challenge_id, status) VALUES (999, 'in_progress')")
                .execute(&db.pool)
                .await;
        assert!(result.is_err(), "orphan attempt must be rejected");
    }
}
