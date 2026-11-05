//! Users: registration and lookup.

use sqlx::SqlitePool;

use super::DbError;

/// Public shape of a user (never includes the password hash).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    /// Numeric id.
    pub id: i64,
    /// Handle (unique, canonical lowercase).
    pub handle: String,
    /// Display name.
    pub display_name: String,
    /// Short public biography.
    pub bio: String,
    /// Media id for the profile image, if present.
    pub avatar_media_id: Option<String>,
    /// Unix seconds.
    pub created_at: i64,
}

/// A user row including the password hash (login path only).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserWithHash {
    /// Numeric id.
    pub id: i64,
    /// Handle.
    pub handle: String,
    /// Display name.
    pub display_name: String,
    /// Short public biography.
    pub bio: String,
    /// Media id for the profile image, if present.
    pub avatar_media_id: Option<String>,
    /// PHC string.
    pub password_hash: String,
}

/// Insert a new user. Hashes must come from `koko_core::password::hash_password`.
pub async fn create_user(
    pool: &SqlitePool,
    handle: &str,
    display_name: &str,
    password_hash: &str,
    created_at: i64,
) -> Result<UserRow, DbError> {
    let res = sqlx::query(
        "INSERT INTO users (handle, display_name, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(handle)
    .bind(display_name)
    .bind(password_hash)
    .bind(created_at)
    .execute(pool)
    .await;
    match res {
        Ok(_) => {}
        Err(sqlx::Error::Database(ref db_err)) if db_err.message().contains("users.handle") => {
            return Err(DbError::Conflict("users.handle".into()));
        }
        Err(e) => return Err(e.into()),
    }
    get_by_handle(pool, handle)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("user {handle}")))
}

/// Look up a user by exact handle.
pub async fn get_by_handle(pool: &SqlitePool, handle: &str) -> Result<Option<UserRow>, DbError> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, handle, display_name, bio, avatar_media_id, created_at FROM users WHERE handle = ?1",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Look up a user by handle including the password hash (login only).
pub async fn get_by_handle_with_hash(
    pool: &SqlitePool,
    handle: &str,
) -> Result<Option<UserWithHash>, DbError> {
    sqlx::query_as::<_, UserWithHash>(
        "SELECT id, handle, display_name, bio, avatar_media_id, password_hash FROM users WHERE handle = ?1",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Look up a user by id.
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<UserRow>, DbError> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, handle, display_name, bio, avatar_media_id, created_at FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Update a user's public display name.
pub async fn update_display_name(
    pool: &SqlitePool,
    user_id: i64,
    display_name: &str,
) -> Result<UserRow, DbError> {
    sqlx::query_as::<_, UserRow>(
        "UPDATE users SET display_name = ?2 WHERE id = ?1
         RETURNING id, handle, display_name, bio, avatar_media_id, created_at",
    )
    .bind(user_id)
    .bind(display_name)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

/// Update the public profile fields for a user.
pub async fn update_profile(
    pool: &SqlitePool,
    user_id: i64,
    display_name: &str,
    bio: &str,
    avatar_media_id: Option<&str>,
) -> Result<UserRow, DbError> {
    sqlx::query_as::<_, UserRow>(
        "UPDATE users SET display_name = ?2, bio = ?3, avatar_media_id = ?4 WHERE id = ?1
         RETURNING id, handle, display_name, bio, avatar_media_id, created_at",
    )
    .bind(user_id)
    .bind(display_name)
    .bind(bio)
    .bind(avatar_media_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

/// Handle prefix search (starting a chat), excluding the requester.
pub async fn find_by_prefix(
    pool: &SqlitePool,
    exclude: i64,
    prefix: &str,
    limit: i64,
) -> Result<Vec<UserRow>, DbError> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, handle, display_name, bio, avatar_media_id, created_at FROM users
         WHERE handle LIKE ?1 || '%' AND id != ?2 ORDER BY handle LIMIT ?3",
    )
    .bind(prefix)
    .bind(exclude)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Total registered users (health endpoint).
pub async fn count_users(pool: &SqlitePool) -> Result<i64, DbError> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(n)
}
