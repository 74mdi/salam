//! Chats: direct (2 members) and groups.

use sqlx::SqlitePool;

use super::DbError;

/// Chat kind codes (match the schema).
pub mod chat_kind {
    /// Two members, no title.
    pub const DIRECT: i64 = 1;
    /// Many members, has a title.
    pub const GROUP: i64 = 2;
}

/// One chat as seen in a chat list.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatSummary {
    /// Chat id.
    pub id: i64,
    /// 1 = direct, 2 = group.
    pub kind: i64,
    /// Group title (NULL for direct chats).
    pub title: Option<String>,
    /// Comma-joined member handles (display convenience).
    pub members: String,
    /// Member count.
    pub member_count: i64,
    /// Unix seconds of the newest message (chat creation if empty).
    pub last_activity: i64,
    /// Kind code of the newest message, if any.
    pub last_kind: Option<i64>,
    /// Envelopes in this chat the user has not marked read.
    pub unread: i64,
}

/// Basic chat row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatRow {
    /// Chat id.
    pub id: i64,
    /// 1 = direct, 2 = group.
    pub kind: i64,
    /// Group title.
    pub title: Option<String>,
    /// Creator.
    pub created_by: i64,
}

/// Find the direct chat between two users, if any.
async fn find_direct(pool: &SqlitePool, a: i64, b: i64) -> Result<Option<i64>, DbError> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT m1.chat_id FROM chat_members m1
         JOIN chat_members m2 ON m2.chat_id = m1.chat_id
         JOIN chats c ON c.id = m1.chat_id
         WHERE c.kind = 1 AND m1.user_id = ?1 AND m2.user_id = ?2",
    )
    .bind(a)
    .bind(b)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

/// Get (or create) the direct chat between `a` and `b`. Returns the chat id.
pub async fn direct_chat(pool: &SqlitePool, a: i64, b: i64, now: i64) -> Result<i64, DbError> {
    if let Some(id) = find_direct(pool, a, b).await? {
        return Ok(id);
    }
    let mut tx = pool.begin().await?;
    let (chat_id,): (i64,) = sqlx::query_as(
        "INSERT INTO chats (kind, title, created_by, created_at) VALUES (1, NULL, ?1, ?2) RETURNING id",
    )
    .bind(a)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    for uid in [a, b] {
        sqlx::query("INSERT INTO chat_members (chat_id, user_id, joined_at) VALUES (?1, ?2, ?3)")
            .bind(chat_id)
            .bind(uid)
            .bind(now)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(chat_id)
}

/// Create a group chat. `member_ids` must include the creator.
pub async fn create_group(
    pool: &SqlitePool,
    creator: i64,
    title: &str,
    member_ids: &[i64],
    now: i64,
) -> Result<i64, DbError> {
    if member_ids.is_empty() {
        return Err(DbError::Conflict("group needs at least one member".into()));
    }
    let mut tx = pool.begin().await?;
    let (chat_id,): (i64,) = sqlx::query_as(
        "INSERT INTO chats (kind, title, created_by, created_at) VALUES (2, ?1, ?2, ?3) RETURNING id",
    )
    .bind(title)
    .bind(creator)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    for &uid in member_ids {
        sqlx::query("INSERT INTO chat_members (chat_id, user_id, joined_at) VALUES (?1, ?2, ?3)")
            .bind(chat_id)
            .bind(uid)
            .bind(now)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(chat_id)
}

/// Get (or create) the user's private notes chat.
pub async fn note_to_self(pool: &SqlitePool, user_id: i64, now: i64) -> Result<i64, DbError> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT c.id FROM chats c
         JOIN chat_members cm ON cm.chat_id = c.id
         WHERE c.kind = ?1 AND c.created_by = ?2 AND c.title = 'Notes to self'
         GROUP BY c.id HAVING COUNT(cm.user_id) = 1",
    )
    .bind(chat_kind::GROUP)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    match existing {
        Some((id,)) => Ok(id),
        None => create_group(pool, user_id, "Notes to self", &[user_id], now).await,
    }
}

/// Add a member to a group (idempotent).
pub async fn add_member(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: i64,
    now: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO chat_members (chat_id, user_id, joined_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (chat_id, user_id) DO NOTHING",
    )
    .bind(chat_id)
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Membership test.
pub async fn is_member(pool: &SqlitePool, chat_id: i64, user_id: i64) -> Result<bool, DbError> {
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM chat_members WHERE chat_id = ?1 AND user_id = ?2")
            .bind(chat_id)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    Ok(n > 0)
}

/// Member ids of a chat.
pub async fn member_ids(pool: &SqlitePool, chat_id: i64) -> Result<Vec<i64>, DbError> {
    let rows: Vec<(i64,)> = sqlx::query_as("SELECT user_id FROM chat_members WHERE chat_id = ?1")
        .bind(chat_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Get one chat.
pub async fn get_chat(pool: &SqlitePool, chat_id: i64) -> Result<Option<ChatRow>, DbError> {
    sqlx::query_as::<_, ChatRow>("SELECT id, kind, title, created_by FROM chats WHERE id = ?1")
        .bind(chat_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

/// List a user's chats, most recently active first.
pub async fn list_chats(pool: &SqlitePool, user_id: i64) -> Result<Vec<ChatSummary>, DbError> {
    // Unread = deliveries past the per-chat read watermark.
    sqlx::query_as::<_, ChatSummary>(
        "SELECT c.id, c.kind, c.title,
                COALESCE(GROUP_CONCAT(DISTINCT u.handle), '') AS members,
                COUNT(DISTINCT u.id) AS member_count,
                COALESCE(
                    (SELECT m.created_at FROM messages m WHERE m.chat_id = c.id ORDER BY m.id DESC LIMIT 1),
                    c.created_at) AS last_activity,
                (SELECT m.kind FROM messages m WHERE m.chat_id = c.id ORDER BY m.id DESC LIMIT 1) AS last_kind,
                (SELECT COUNT(*) FROM deliveries d
                  WHERE d.user_id = ?1 AND d.chat_id = c.id
                    AND d.seq > (SELECT cm.last_read_seq FROM chat_members cm
                                 WHERE cm.chat_id = c.id AND cm.user_id = ?1)) AS unread
         FROM chats c
         JOIN chat_members cm ON cm.chat_id = c.id AND cm.user_id = ?1
         JOIN chat_members m2 ON m2.chat_id = c.id
         JOIN users u ON u.id = m2.user_id
         GROUP BY c.id
         ORDER BY last_activity DESC",
    )
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Advance the read watermark (monotonic — never moves backwards).
pub async fn set_last_read(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: i64,
    up_to_seq: i64,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE chat_members SET last_read_seq = MAX(last_read_seq, ?3)
         WHERE chat_id = ?1 AND user_id = ?2",
    )
    .bind(chat_id)
    .bind(user_id)
    .bind(up_to_seq)
    .execute(pool)
    .await?;
    Ok(())
}
