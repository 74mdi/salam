//! Sessions. Tokens are random on the wire, hashed at rest.
//! (Devices live in `devices.rs`.)

use sqlx::SqlitePool;

use super::DbError;

/// A live session (device bound).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    /// Numeric id.
    pub id: i64,
    /// User id.
    pub user_id: i64,
    /// Device id.
    pub device_id: i64,
    /// Handle (joined in for convenience).
    pub handle: String,
}

/// Create a session; `token_hash` is `SHA-256(token)` — the raw token is
/// never stored.
pub async fn create_session(
    pool: &SqlitePool,
    token_hash: &[u8],
    user_id: i64,
    device_id: i64,
    created_at: i64,
    ttl_secs: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, device_id, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?4 + ?5)",
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(device_id)
    .bind(created_at)
    .bind(ttl_secs)
    .execute(pool)
    .await?;
    Ok(())
}

/// Resolve a token hash into a live session (expiry-checked here).
pub async fn lookup_session(
    pool: &SqlitePool,
    token_hash: &[u8],
    now: i64,
) -> Result<Option<SessionRow>, DbError> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT s.id, s.user_id, s.device_id, u.handle
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = ?1 AND s.expires_at > ?2",
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Delete one session (logout). Returns `true` if a session was removed.
pub async fn delete_session(pool: &SqlitePool, token_hash: &[u8]) -> Result<bool, DbError> {
    let res = sqlx::query("DELETE FROM sessions WHERE token_hash = ?1")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// Revoke every session of a user ("logout everywhere").
pub async fn revoke_all_for_user(pool: &SqlitePool, user_id: i64) -> Result<u64, DbError> {
    let res = sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Housekeeping: drop expired sessions. Returns rows removed.
pub async fn delete_expired(pool: &SqlitePool, now: i64) -> Result<u64, DbError> {
    let res = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?1")
        .bind(now)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
