//! # koko-db
//!
//! SQLite persistence for Koko. One file. No other database, ever.
//!
//! Guarantees enforced here:
//!
//! * **WAL mode** — readers never block the (single) writer, which is what
//!   makes "assign seq inside the insert transaction" cheap enough for a
//!   1 vCPU box.
//! * **`foreign_keys = ON`** — SQLite disables FKs by default; forgetting
//!   this is how orphaned rows happen.
//! * **`busy_timeout = 5s`** — under write contention the pool waits instead
//!   of erroring.
//! * **Embedded migrations** — `sqlx::migrate!` compiles the SQL into the
//!   binary, so `koko-server` upgrades its own schema on boot. There is no
//!   migration step in the self-hosting runbook.
//!
//! The critical function is [`messages::insert_message`]: membership check,
//! message insert, per-recipient seq allocation and fanout rows all happen
//! in **one transaction**, so a message either exists for every member with
//! a stable seq, or doesn't exist at all.

pub mod chats;
pub mod devices;
pub mod media;
pub mod messages;
pub mod sessions;
pub mod users;

use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use thiserror::Error;

/// Errors from the persistence layer.
#[derive(Debug, Error)]
pub enum DbError {
    /// sqlx/SQLite failure.
    #[error("database error: {0}")]
    Sql(#[from] sqlx::Error),
    /// Row looked up by id/handle does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// Uniqueness constraint violated (e.g. handle taken).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Actor is not a member of the chat.
    #[error("not a member of chat {0}")]
    NotMember(i64),
}

impl From<DbError> for koko_core::KokoError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(what) => koko_core::KokoError::NotFound(what),
            DbError::Conflict(what) => {
                if what.contains("users.handle") {
                    koko_core::KokoError::HandleTaken
                } else {
                    koko_core::KokoError::BadRequest(what)
                }
            }
            DbError::NotMember(chat) => {
                koko_core::KokoError::Forbidden(format!("not a member of chat {chat}"))
            }
            DbError::Sql(e) => {
                log::error!("sqlite failure: {e}");
                koko_core::KokoError::Internal
            }
        }
    }
}

/// Open (and migrate) the database at `path`.
///
/// Creates the file if missing. Applies pending migrations idempotently.
pub async fn connect(path: &Path) -> Result<SqlitePool, DbError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            DbError::Conflict(format!(
                "cannot create db directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| DbError::Sql(e.into()))?;
    log::info!("sqlite ready at {} (wal, foreign_keys on)", path.display());
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_creates_and_migrates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("koko.db");
        let pool = connect(&path).await.unwrap();
        // Schema exists and pragmas are on.
        let (fk,): (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(fk, 1);
        let (jm,): (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(jm.to_lowercase(), "wal");
        // Migrations are idempotent.
        connect(&path).await.unwrap();
    }
}
