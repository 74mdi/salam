-- Koko schema v1.
-- One SQLite file. WAL mode + foreign_keys are enforced at connect time
-- (see koko-db/src/lib.rs), not here, so file-level tooling also works.

PRAGMA user_version = 1;

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    handle        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name  TEXT NOT NULL,
    password_hash TEXT NOT NULL,              -- PHC string (Argon2id)
    created_at    INTEGER NOT NULL,           -- unix seconds
    is_admin      INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE devices (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_seen  INTEGER NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE sessions (
    id          INTEGER PRIMARY KEY,
    token_hash  BLOB NOT NULL UNIQUE,         -- SHA-256(token); never the token
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id   INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL,
    expires_at  INTEGER NOT NULL
);
CREATE INDEX idx_sessions_user ON sessions(user_id);

CREATE TABLE chats (
    id         INTEGER PRIMARY KEY,
    kind       INTEGER NOT NULL,              -- 1 = direct, 2 = group
    title      TEXT,                          -- groups only
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at INTEGER NOT NULL
);

CREATE TABLE chat_members (
    chat_id        INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at      INTEGER NOT NULL,
    last_read_seq  INTEGER NOT NULL DEFAULT 0, -- unread watermark
    PRIMARY KEY (chat_id, user_id)
);
CREATE INDEX idx_chat_members_user ON chat_members(user_id);

CREATE TABLE messages (
    id             INTEGER PRIMARY KEY,
    chat_id        INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    sender_id      INTEGER NOT NULL REFERENCES users(id),
    client_msg_id  BLOB NOT NULL,             -- idempotency key from the client
    kind           INTEGER NOT NULL,          -- MessageKind code
    body           BLOB NOT NULL,             -- opaque to the server
    created_at     INTEGER NOT NULL,
    UNIQUE (chat_id, sender_id, client_msg_id)
);
CREATE INDEX idx_messages_chat ON messages(chat_id, id);

-- Per-recipient fanout: every user has their own monotonic inbox seq.
-- This table IS the sync protocol: reconnecting clients replay from cursor.
CREATE TABLE deliveries (
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    chat_id    INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, seq)
);
CREATE INDEX idx_deliveries_message ON deliveries(message_id);

-- next_seq per user, bumped inside the same transaction as message insert.
CREATE TABLE inbox_counters (
    user_id  INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    next_seq INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE media (
    id          TEXT PRIMARY KEY,             -- random hex, also the filename
    owner_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    filename    TEXT NOT NULL,
    mime        TEXT NOT NULL,
    size        INTEGER NOT NULL,             -- bytes
    blurhash    TEXT,                         -- reserved for lazy placeholders
    created_at  INTEGER NOT NULL,
    last_access INTEGER NOT NULL              -- LRU clock
);
CREATE INDEX idx_media_owner_lru ON media(owner_id, last_access);
