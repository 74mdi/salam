//! Messages and the fanout transaction — the heart of the sync engine.

use koko_protocol::{ChatId, Envelope, MessageKind, UserId};
use sqlx::SqlitePool;

use super::DbError;

/// A send request (already validated: member of chat, body within limits).
#[derive(Debug)]
pub struct NewMessage<'a> {
    /// Target chat.
    pub chat_id: i64,
    /// Sender.
    pub sender_id: i64,
    /// Idempotency key.
    pub client_msg_id: [u8; 16],
    /// `MessageKind` code.
    pub kind: i64,
    /// Opaque body.
    pub body: &'a [u8],
    /// Client timestamp (unix seconds).
    pub created_at: i64,
}

/// Result of a successful (or duplicate) insert.
#[derive(Debug)]
pub struct Fanout {
    /// Row id of the message.
    pub message_id: i64,
    /// The sender's own inbox seq for this message (what `Sent` reports).
    pub sender_seq: u64,
    /// Every recipient with their allocated seq.
    pub recipients: Vec<Recipient>,
    /// `true` when this call found the message already stored (client retry).
    pub duplicate: bool,
}

/// One recipient's allocated seq.
#[derive(Debug, Clone)]
pub struct Recipient {
    /// User id.
    pub user_id: i64,
    /// Their inbox seq for this message.
    pub seq: u64,
}

/// Insert a message and allocate per-recipient seqs in ONE transaction.
///
/// The transaction is the serialization point that makes Koko's sync model
/// work: SQLite's single writer guarantees no two messages can observe the
/// same `(user_id, seq)`, so a client that has cursor N can always ask for
/// `N+1..` and trust the result is gapless and ordered.
///
/// Idempotency: if `(chat, sender, client_msg_id)` already exists the
/// original allocation is returned with `duplicate = true` and nothing new
/// is written — safe to retry sends after a dropped connection.
pub async fn insert_message(pool: &SqlitePool, msg: &NewMessage<'_>) -> Result<Fanout, DbError> {
    let mut tx = pool.begin().await?;

    // 1. The sender must be a member.
    let (member,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM chat_members WHERE chat_id = ?1 AND user_id = ?2")
            .bind(msg.chat_id)
            .bind(msg.sender_id)
            .fetch_one(&mut *tx)
            .await?;
    if member == 0 {
        return Err(DbError::NotMember(msg.chat_id));
    }

    // 2. Idempotency probe.
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM messages WHERE chat_id = ?1 AND sender_id = ?2 AND client_msg_id = ?3",
    )
    .bind(msg.chat_id)
    .bind(msg.sender_id)
    .bind(&msg.client_msg_id[..])
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((message_id,)) = existing {
        let recipients = load_recipients(&mut tx, message_id).await?;
        let sender_seq = recipients
            .iter()
            .find(|r| r.user_id == msg.sender_id)
            .map_or(0, |r| r.seq);
        return Ok(Fanout {
            message_id,
            sender_seq,
            recipients,
            duplicate: true,
        });
    }

    // 3. Store the message.
    let (message_id,): (i64,) = sqlx::query_as(
        "INSERT INTO messages (chat_id, sender_id, client_msg_id, kind, body, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
    )
    .bind(msg.chat_id)
    .bind(msg.sender_id)
    .bind(&msg.client_msg_id[..])
    .bind(msg.kind)
    .bind(msg.body)
    .bind(msg.created_at)
    .fetch_one(&mut *tx)
    .await?;

    // 4. Fan out: allocate each member's next seq and their delivery row.
    let members: Vec<(i64,)> =
        sqlx::query_as("SELECT user_id FROM chat_members WHERE chat_id = ?1")
            .bind(msg.chat_id)
            .fetch_all(&mut *tx)
            .await?;
    let mut recipients = Vec::with_capacity(members.len());
    for (user_id,) in members {
        let (seq,): (i64,) = sqlx::query_as(
            "INSERT INTO inbox_counters (user_id, next_seq) VALUES (?1, 2)
             ON CONFLICT (user_id) DO UPDATE SET next_seq = inbox_counters.next_seq + 1
             RETURNING next_seq - 1",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO deliveries (user_id, seq, chat_id, message_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(user_id)
        .bind(seq)
        .bind(msg.chat_id)
        .bind(message_id)
        .bind(msg.created_at)
        .execute(&mut *tx)
        .await?;
        recipients.push(Recipient {
            user_id,
            seq: seq.max(0) as u64,
        });
    }

    tx.commit().await?;

    let sender_seq = recipients
        .iter()
        .find(|r| r.user_id == msg.sender_id)
        .map_or(0, |r| r.seq);
    Ok(Fanout {
        message_id,
        sender_seq,
        recipients,
        duplicate: false,
    })
}

async fn load_recipients(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    message_id: i64,
) -> Result<Vec<Recipient>, DbError> {
    let rows: Vec<(i64, i64)> =
        sqlx::query_as("SELECT user_id, seq FROM deliveries WHERE message_id = ?1")
            .bind(message_id)
            .fetch_all(&mut **tx)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(user_id, seq)| Recipient {
            user_id,
            seq: seq.max(0) as u64,
        })
        .collect())
}

/// Replay a user's inbox after `since_seq`, oldest first.
pub async fn fetch_since(
    pool: &SqlitePool,
    user_id: i64,
    since_seq: u64,
    limit: i64,
) -> Result<Vec<Envelope>, DbError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        seq: i64,
        chat_id: i64,
        sender_id: i64,
        handle: String,
        kind: i64,
        body: Vec<u8>,
        created_at: i64,
    }
    let rows: Vec<Row> = sqlx::query_as::<_, Row>(
        "SELECT d.seq, m.chat_id, m.sender_id, u.handle, m.kind, m.body, m.created_at
         FROM deliveries d
         JOIN messages m ON m.id = d.message_id
         JOIN users u ON u.id = m.sender_id
         WHERE d.user_id = ?1 AND d.seq > ?2
         ORDER BY d.seq ASC LIMIT ?3",
    )
    .bind(user_id)
    .bind(since_seq as i64)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Envelope {
            seq: r.seq.max(0) as u64,
            chat: ChatId(r.chat_id),
            sender: UserId(r.sender_id),
            sender_handle: r.handle,
            kind: MessageKind::from_code(r.kind),
            body: r.body,
            created_at: r.created_at,
        })
        .collect())
}

/// The highest seq currently allocated for a user (their cursor "now").
pub async fn current_seq(pool: &SqlitePool, user_id: i64) -> Result<u64, DbError> {
    let (seq,): (i64,) =
        sqlx::query_as("SELECT COALESCE((SELECT MAX(seq) FROM deliveries WHERE user_id = ?1), 0)")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    Ok(seq.max(0) as u64)
}

/// One chat-view message (bootstrap query; clients normally use their mirror).
#[derive(Debug, sqlx::FromRow)]
pub struct ChatMessage {
    /// Row id.
    pub message_id: i64,
    /// Recipient's inbox seq.
    pub seq: i64,
    /// Sender.
    pub sender_id: i64,
    /// Sender's handle.
    pub handle: String,
    /// Kind code.
    pub kind: i64,
    /// Opaque body.
    pub body: Vec<u8>,
    /// Unix seconds.
    pub created_at: i64,
}

/// Page through one chat (user-scoped: reads via that user's deliveries).
pub async fn fetch_chat_messages(
    pool: &SqlitePool,
    user_id: i64,
    chat_id: i64,
    after_message_id: i64,
    limit: i64,
) -> Result<Vec<ChatMessage>, DbError> {
    let rows: Vec<ChatMessage> = sqlx::query_as::<_, ChatMessage>(
        "SELECT m.id AS message_id, d.seq, m.sender_id, u.handle, m.kind, m.body, m.created_at
         FROM deliveries d
         JOIN messages m ON m.id = d.message_id
         JOIN users u ON u.id = m.sender_id
         WHERE d.user_id = ?1 AND d.chat_id = ?2 AND m.id > ?3
         ORDER BY m.id ASC LIMIT ?4",
    )
    .bind(user_id)
    .bind(chat_id)
    .bind(after_message_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Total stored messages (health endpoint).
pub async fn count_messages(pool: &SqlitePool) -> Result<i64, DbError> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
        .fetch_one(pool)
        .await?;
    Ok(n)
}
