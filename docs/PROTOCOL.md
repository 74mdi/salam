# The koko wire protocol

Version 1 (`KOK1`). Canonical source: the `koko-protocol` crate — this
document describes it, it does not define it. If they disagree, the code
wins and this file gets fixed.

## Transports

| Channel | Encoding | Purpose |
|---|---|---|
| WebSocket `/ws` | binary frames, this protocol | messaging, typing, live sync |
| HTTP `/api/*` | JSON | register/login, chat lists, sync fallback, media |

Both directions of a conversation matter: HTTP is the bootstrap and
fallback; WS is the live path. Clients must implement both.

## Frame format

One WS binary message = exactly one frame:

```text
+--------+---------------------------+
| "KOK1" | bincode payload           |
| 4 B    | rest of the message       |
+--------+---------------------------+
```

- Magic `0x4B 0x4F 0x4B 0x31` — the trailing `1` is the protocol version.
  An incompatible future change becomes `KOK2`, and old servers answer
  `ErrorCode::UnsupportedVersion` instead of mis-decoding.
- Payload: `bincode` 1.3 default options — **fixint, little-endian**.
  Enums encode as a `u32` variant index (add new variants only at the end).
- Frames must be sent as single, unfragmented binary WS messages.
- Decode cap: 1 MiB payload (`MAX_FRAME_SIZE`); message bodies cap at
  256 KiB (`MAX_MESSAGE_BODY`) — larger content belongs in the media store.

## Frames

### Client → server

| Variant | Fields | Notes |
|---|---|---|
| `Hello` | `token: String`, `resume_from: Option<u64>` | First frame, within 10 s. Authenticates the socket; `resume_from` = "I have everything ≤ N" |
| `Send` | `client_msg_id: [u8;16]`, `chat: i64`, `kind: u8-enum`, `body: bytes`, `created_at: i64` | Idempotent on `(chat, sender, client_msg_id)` — safe to retry |
| `Ack` | `chat: i64`, `up_to_seq: u64` | Read receipt watermark (monotonic server-side) |
| `Typing` | `chat: i64`, `on: bool` | Relayed to other members |
| `Ping` | — | App-level keepalive → `Pong` |

### Server → client

| Variant | Fields | Notes |
|---|---|---|
| `Hello` | `user: i64`, `handle: String`, `next_seq: u64`, `server_time: i64`, `resumed_messages: u32` | Followed by that many backlog envelopes |
| `Envelope` | `env: Envelope` | A message addressed to you |
| `Typing` | `chat`, `from: i64`, `from_handle: String`, `on: bool` | Relay |
| `Sent` | `client_msg_id: [u8;16]`, `seq: u64` | Durable ack: stored + fanned out; `seq` = your own inbox number for it |
| `Error` | `code: ErrorCode`, `message: String` | Typed, never a bare string |
| `Pong` | — | |

## The envelope

```rust
pub struct Envelope {
    pub seq: u64,            // recipient's inbox sequence (starts at 1)
    pub chat: i64,
    pub sender: i64,
    pub sender_handle: String,
    pub kind: MessageKind,   // Text=0 Image=1 Voice=2 Video=3 File=4 System=5
    pub body: Vec<u8>,       // opaque to the server — E2E ciphertext later
    pub created_at: i64,     // unix seconds, client-assigned
}
```

`seq` is **per recipient**: one stored message fans out to N delivery
rows, each with that member's next seq. A reconnecting client replays
`since_seq+1..` and the stream is gapless by construction (allocation
happens inside the insert transaction).

## Error codes

| Code | Name | HTTP | Meaning |
|---|---|---|---|
| 1 | UnsupportedVersion | — | Magic says a newer protocol |
| 2 | Unauthorized | 401 | Missing/expired/invalid token |
| 3 | Forbidden | 403 | Authenticated but not allowed |
| 4 | NotFound | 404 | Referenced object missing |
| 5 | RateLimited | 429 | Budget exceeded |
| 6 | PayloadTooLarge | 413 | Size limit hit |
| 7 | QuotaExceeded | 413 | Media quota exhausted |
| 8 | BadRequest | 400 | Malformed input |
| 9 | Internal | 500 | Unspecified (details in server logs) |

HTTP error bodies: `{"error": {"code": <n>, "message": "..."}}`.

## Lifecycles

Connect and authenticate:

```text
client                          server
  │ ── ws connect /ws ────────────▶│
  │ ── Hello{token, resume?} ─────▶│      (10 s deadline)
  │ ◀── Hello{user, next_seq, …} ──│
  │ ◀── Envelope × resumed ────────│      (only if resume_from given)
  │                                │
  │        live: select { frames, hub events }
```

Send (idempotent):

```text
  │ ── Send{cmid, chat, body} ────▶│
  │        (SQLite txn: message + per-user seqs)
  │ ◀── Envelope{seq} ─────────────│      to every online member incl. sender
  │ ◀── Sent{cmid, seq} ───────────│      durable ack
  │ ── Send{cmid, chat, body} ────▶│      retry after a drop
  │ ◀── Sent{cmid, seq} ───────────│      same seq, nothing re-stored
```

Offline catch-up (REST, authoritative):

```text
GET /api/messages?since=41&limit=500
← {"messages":[Envelope…], "next_cursor": 45}
```

## HTTP API (JSON)

| Method + path | Auth | Purpose |
|---|---|---|
| `POST /api/register` | — | `{handle, display_name?, password}` → 201 user |
| `POST /api/login` | — | `{handle, password, device_name}` → `{token, expires_at, user, device_id}` |
| `POST /api/logout` | bearer | revoke this session |
| `POST /api/logout-all` | bearer | revoke every session |
| `GET /api/me` | bearer | user + devices |
| `GET /api/users?q=pre` | bearer | handle prefix search |
| `GET /api/users/{handle}` | bearer | user lookup |
| `POST /api/chats` | bearer | `{kind:"direct",peer}` or `{kind:"group",title,members:[handle]}` |
| `GET /api/chats` | bearer | chat list (members, unread, last activity) |
| `GET /api/chats/{id}/messages?after=&limit=` | bearer | chat paging (mirror bootstrap) |
| `POST /api/chats/{id}/read` | bearer | `{up_to_seq}` |
| `POST /api/chats/{id}/members` | bearer | `{handle}` (groups; v0.1: any member) |
| `GET /api/messages?since=&limit=` | bearer | global inbox replay |
| `POST /api/media?name=` | bearer | raw body upload → `{id,url,size,mime}` |
| `GET /api/media/{id}` | bearer | download (LRU-touched) |
| `DELETE /api/media/{id}` | bearer | owner delete |
| `GET /healthz` | — | public health snapshot |

## Limits (v1 defaults)

256 KiB message body · 25 MiB per file · 4 GiB per-user media quota (LRU)
· 512 concurrent WS · 120 sends/min/socket · 1 MiB frame decode cap.
