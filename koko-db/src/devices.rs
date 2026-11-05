//! Devices: one row per named client ("linux-desktop", "pixel-8").

use sqlx::SqlitePool;

use super::DbError;

/// A device row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    /// Numeric id.
    pub id: i64,
    /// Owning user id.
    pub user_id: i64,
    /// Human name chosen at login.
    pub name: String,
    /// Unix seconds.
    pub last_seen: i64,
}

/// Register or refresh a device, returning its stable id.
pub async fn upsert_device(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    now: i64,
) -> Result<i64, DbError> {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO devices (user_id, name, created_at, last_seen) VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT (user_id, name) DO UPDATE SET last_seen = ?3
         RETURNING id",
    )
    .bind(user_id)
    .bind(name)
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// List a user's devices (for the "logout everywhere" UI).
pub async fn list_devices(pool: &SqlitePool, user_id: i64) -> Result<Vec<DeviceRow>, DbError> {
    sqlx::query_as::<_, DeviceRow>(
        "SELECT id, user_id, name, last_seen FROM devices WHERE user_id = ?1 ORDER BY last_seen DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Heartbeat: bump a device's `last_seen`.
pub async fn touch_device(pool: &SqlitePool, device_id: i64, now: i64) -> Result<(), DbError> {
    sqlx::query("UPDATE devices SET last_seen = ?2 WHERE id = ?1")
        .bind(device_id)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}
