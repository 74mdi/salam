//! Media metadata. Bytes live on disk under `data/media/<id>`;
//! this table tracks ownership, size (quota accounting) and the LRU clock.

use sqlx::SqlitePool;

use super::DbError;

/// A media row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MediaRow {
    /// Random hex id (also the on-disk filename).
    pub id: String,
    /// Owning user.
    pub owner_id: i64,
    /// Original filename (display only, never used on disk).
    pub filename: String,
    /// MIME type.
    pub mime: String,
    /// Size in bytes.
    pub size: i64,
    /// Unix seconds.
    pub created_at: i64,
    /// Unix seconds of last download (LRU clock).
    pub last_access: i64,
}

/// Insert metadata (call after the file is durably on disk). `now_millis`
/// seeds the LRU clock; millisecond resolution avoids same-second ties.
pub async fn insert_media(
    pool: &SqlitePool,
    id: &str,
    owner_id: i64,
    filename: &str,
    mime: &str,
    size: i64,
    now_millis: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO media (id, owner_id, filename, mime, size, created_at, last_access)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6 / 1000, ?6)",
    )
    .bind(id)
    .bind(owner_id)
    .bind(filename)
    .bind(mime)
    .bind(size)
    .bind(now_millis)
    .execute(pool)
    .await?;
    Ok(())
}

/// Look up one media row.
pub async fn get_media(pool: &SqlitePool, id: &str) -> Result<Option<MediaRow>, DbError> {
    sqlx::query_as::<_, MediaRow>(
        "SELECT id, owner_id, filename, mime, size, created_at, last_access FROM media WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Bytes used by one owner.
pub async fn usage_bytes(pool: &SqlitePool, owner_id: i64) -> Result<i64, DbError> {
    let (n,): (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(size), 0) FROM media WHERE owner_id = ?1")
            .bind(owner_id)
            .fetch_one(pool)
            .await?;
    Ok(n)
}

/// Bump the LRU clock (millisecond resolution — a download and its upload
/// can easily happen within the same second, and ties break on random ids).
pub async fn touch_media(pool: &SqlitePool, id: &str, now_millis: i64) -> Result<(), DbError> {
    sqlx::query("UPDATE media SET last_access = ?2 WHERE id = ?1")
        .bind(id)
        .bind(now_millis)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete one media row. Returns `true` if it existed.
pub async fn delete_media(pool: &SqlitePool, id: &str) -> Result<bool, DbError> {
    let res = sqlx::query("DELETE FROM media WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// LRU eviction: remove the owner's least-recently-used media rows until
/// `bytes_to_free` are freed. Returns the evicted rows so the caller can
/// unlink the files. Runs in one transaction — quota is never double-spent.
pub async fn evict_lru(
    pool: &SqlitePool,
    owner_id: i64,
    bytes_to_free: i64,
) -> Result<Vec<MediaRow>, DbError> {
    if bytes_to_free <= 0 {
        return Ok(Vec::new());
    }
    let mut tx = pool.begin().await?;
    let rows: Vec<MediaRow> = sqlx::query_as::<_, MediaRow>(
        "SELECT id, owner_id, filename, mime, size, created_at, last_access
         FROM media WHERE owner_id = ?1 ORDER BY last_access ASC, id ASC",
    )
    .bind(owner_id)
    .fetch_all(&mut *tx)
    .await?;

    let mut freed = 0i64;
    let mut evicted = Vec::new();
    for row in rows {
        if freed >= bytes_to_free {
            break;
        }
        sqlx::query("DELETE FROM media WHERE id = ?1")
            .bind(&row.id)
            .execute(&mut *tx)
            .await?;
        freed += row.size;
        evicted.push(row);
    }
    tx.commit().await?;
    Ok(evicted)
}

/// Total bytes stored across all owners (health endpoint).
pub async fn total_bytes(pool: &SqlitePool) -> Result<i64, DbError> {
    let (n,): (i64,) = sqlx::query_as("SELECT COALESCE(SUM(size), 0) FROM media")
        .fetch_one(pool)
        .await?;
    Ok(n)
}
