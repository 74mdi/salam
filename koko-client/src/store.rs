//! The local SQLite mirror — the client is offline-first.
//!
//! The server is a *sync target*, not the UI's data source: every envelope
//! (WS live, WS backlog, REST catch-up) lands here first, and every view
//! reads from here. `(chat_id, seq)` is the primary key, so the at-least-once
//! delivery model dedupes for free.

use rusqlite::Connection;

use koko_protocol::Envelope;

/// A stored message.
#[derive(Debug, Clone)]
pub struct MirrorMsg {
    /// Sender id (0 for optimistic local echoes).
    pub sender_id: i64,
    /// Sender handle.
    pub sender_handle: String,
    /// Kind code.
    pub kind: i64,
    /// Opaque body.
    pub body: Vec<u8>,
    /// Unix seconds.
    pub created_at: i64,
    /// Client-side idempotency key (local echo matching).
    pub client_msg_id: Option<[u8; 16]>,
    /// `true` once the server's `Sent` ack landed.
    pub delivered: bool,
}

/// The local mirror (newtype over `rusqlite::Connection`).
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (and create if needed) the mirror database.
    pub fn open(path: &str) -> rusqlite::Result<Store> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "wal")?;
        conn.pragma_update(None, "synchronous", "normal")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
             chat_id        INTEGER NOT NULL,
             seq            INTEGER NOT NULL,
             sender_id      INTEGER NOT NULL,
             sender_handle  TEXT NOT NULL,
             kind           INTEGER NOT NULL,
             body           BLOB NOT NULL,
             created_at     INTEGER NOT NULL,
             client_msg_id  BLOB,
             delivered      INTEGER NOT NULL DEFAULT 1,
             hidden         INTEGER NOT NULL DEFAULT 0,
             PRIMARY KEY (chat_id, seq)
         );
         CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(chat_id, created_at);
         CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);",
        )?;
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0",
            [],
        );
        Ok(Store { conn })
    }

    /// Apply an envelope (idempotent by `(chat, seq)`).
    pub fn apply_envelope(&self, env: &Envelope) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO messages
                 (chat_id, seq, sender_id, sender_handle, kind, body, created_at, delivered)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            rusqlite::params![
                env.chat.0,
                env.seq as i64,
                env.sender.0,
                env.sender_handle,
                env.kind.code(),
                env.body,
                env.created_at,
            ],
        )?;
        Ok(())
    }

    /// Apply a REST bootstrap row (same dedupe key).
    pub fn apply_bootstrap(
        &self,
        chat_id: i64,
        m: &crate::api::BootstrapMsg,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO messages
                 (chat_id, seq, sender_id, sender_handle, kind, body, created_at, delivered)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            rusqlite::params![
                chat_id,
                m.seq as i64,
                m.sender_id,
                m.sender_handle,
                m.kind,
                m.body,
                m.created_at,
            ],
        )?;
        Ok(())
    }

    /// Insert an optimistic local echo with a negative temporary sequence.
    pub fn add_pending(&self, chat_id: i64, m: &MirrorMsg) -> rusqlite::Result<()> {
        let client_msg_id = m.client_msg_id.unwrap_or([0; 16]);
        let mut local_seq_bytes = [0; 8];
        local_seq_bytes.copy_from_slice(&client_msg_id[..8]);
        let local_seq = -((u64::from_le_bytes(local_seq_bytes) % i64::MAX as u64) as i64 + 1);
        self.conn.execute(
            "INSERT OR REPLACE INTO messages
                 (chat_id, seq, sender_id, sender_handle, kind, body, created_at, client_msg_id, delivered)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
            rusqlite::params![
                chat_id,
                local_seq,
                m.sender_id,
                m.sender_handle,
                m.kind,
                m.body,
                m.created_at,
                m.client_msg_id,
            ],
        )?;
        Ok(())
    }

    /// Mark a pending echo by id when the target chat is not currently open.
    pub fn mark_delivered_any(&self, client_msg_id: &[u8; 16], seq: u64) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM messages
             WHERE client_msg_id = ?1 AND delivered = 0
               AND EXISTS (
                   SELECT 1 FROM messages existing
                   WHERE existing.chat_id = messages.chat_id AND existing.seq = ?2
               )",
            rusqlite::params![client_msg_id, seq as i64],
        )?;
        self.conn.execute(
            "UPDATE messages SET seq = ?2, delivered = 1
             WHERE client_msg_id = ?1 AND delivered = 0
               AND NOT EXISTS (
                   SELECT 1 FROM messages existing
                   WHERE existing.chat_id = messages.chat_id AND existing.seq = ?2
               )",
            rusqlite::params![client_msg_id, seq as i64],
        )?;
        Ok(())
    }

    /// All messages of a chat, oldest first.
    pub fn chat_messages(&self, chat_id: i64) -> rusqlite::Result<Vec<MirrorMsg>> {
        let mut stmt = self.conn.prepare(
            "SELECT chat_id, seq, sender_id, sender_handle, kind, body, created_at, client_msg_id, delivered
             FROM messages WHERE chat_id = ?1 AND hidden = 0 ORDER BY created_at ASC, seq ASC",
        )?;
        let rows = stmt.query_map([chat_id], |row| {
            Ok(MirrorMsg {
                sender_id: row.get(2)?,
                sender_handle: row.get(3)?,
                kind: row.get(4)?,
                body: row.get(5)?,
                created_at: row.get(6)?,
                client_msg_id: row.get(7)?,
                delivered: row.get::<_, i64>(8)? == 1,
            })
        })?;
        rows.collect()
    }

    /// Hide a message in this local mirror. The server remains authoritative
    /// for other devices; this is intentionally a "delete for me" action.
    pub fn hide_message(&self, chat_id: i64, message: &MirrorMsg) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE messages SET hidden = 1
             WHERE rowid = (
                 SELECT rowid FROM messages
                 WHERE chat_id = ?1 AND kind = ?2 AND body = ?3 AND created_at = ?4
                   AND hidden = 0
                 ORDER BY seq DESC LIMIT 1
             )",
            rusqlite::params![chat_id, message.kind, message.body, message.created_at],
        )?;
        Ok(())
    }

    /// Return whether a cached message in a chat contains the local search text.
    pub fn chat_contains_text(&self, chat_id: i64, query: &str) -> bool {
        let needle = query.to_lowercase();
        let Ok(mut stmt) = self
            .conn
            .prepare("SELECT body FROM messages WHERE chat_id = ?1 AND hidden = 0")
        else {
            return false;
        };
        let Ok(rows) = stmt.query_map([chat_id], |row| row.get::<_, Vec<u8>>(0)) else {
            return false;
        };
        let found = rows.flatten().any(|body| {
            String::from_utf8_lossy(&body)
                .to_lowercase()
                .contains(&needle)
        });
        found
    }

    /// Return the latest cached text as a compact list preview.
    pub fn latest_preview(&self, chat_id: i64) -> Option<String> {
        let body = self
            .conn
            .query_row(
                "SELECT body FROM messages WHERE chat_id = ?1 AND hidden = 0
                 ORDER BY created_at DESC, seq DESC LIMIT 1",
                [chat_id],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .ok()?;
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(filename) = value.get("filename").and_then(serde_json::Value::as_str) {
                let kind = value
                    .get("mime")
                    .and_then(serde_json::Value::as_str)
                    .filter(|mime| mime.starts_with("image/"))
                    .map_or("File", |_| "Image");
                return Some(format!("{kind} · {filename}"));
            }
        }
        let preview = String::from_utf8_lossy(&body).replace(['\n', '\r'], " ");
        let preview = preview.trim();
        if preview.is_empty() {
            None
        } else if preview.chars().count() > 56 {
            Some(format!("{}…", preview.chars().take(55).collect::<String>()))
        } else {
            Some(preview.to_owned())
        }
    }

    /// Highest inbox seq known locally (the sync cursor).
    pub fn max_seq(&self) -> u64 {
        self.conn
            .query_row("SELECT COALESCE(MAX(seq), 0) FROM messages", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_or(0, |v| v.max(0) as u64)
    }

    /// Read a persisted local preference, falling back when it has not been set.
    pub fn preference(&self, key: &str, default: bool) -> bool {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .map_or(default, |value| value == "1")
    }

    /// Persist a local on/off preference in the mirror database.
    pub fn set_preference(&self, key: &str, value: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, if value { "1" } else { "0" }],
        )?;
        Ok(())
    }

    /// Read a persisted local text preference, falling back when it is absent.
    pub fn text_preference(&self, key: &str, default: &str) -> String {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .unwrap_or_else(|_| default.to_owned())
    }

    /// Persist a local text preference in the mirror database.
    pub fn set_text_preference(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }
}
