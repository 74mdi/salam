//! WebSocket frames — one frame per binary WS message, in both directions.

use crate::envelope::MessageKind;
use crate::ids::{ChatId, ClientMsgId, UserId};
use crate::Envelope;
use serde::{Deserialize, Serialize};

/// Client → server frames.
///
/// Encoded as the variant index followed by the variant fields (bincode).
/// Add new variants only at the end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientFrame {
    /// First frame after connecting. Authenticates the socket and optionally
    /// asks for a backlog replay since a previous cursor.
    Hello {
        /// Opaque session token from `POST /api/login`.
        token: String,
        /// "I already have every envelope up to this seq" — replay from +1.
        resume_from: Option<u64>,
    },
    /// Send a message. The server replies with [`ServerFrame::Sent`].
    Send {
        /// Idempotency key, see [`ClientMsgId`].
        client_msg_id: ClientMsgId,
        /// Target chat.
        chat: ChatId,
        /// Payload kind.
        kind: MessageKind,
        /// Opaque body, capped at [`crate::MAX_MESSAGE_BODY`].
        body: Vec<u8>,
        /// Client-assigned unix-seconds timestamp.
        created_at: i64,
    },
    /// Mark a chat read up to (and including) a sequence number.
    Ack {
        /// Chat that was read.
        chat: ChatId,
        /// Highest envelope seq the user has seen in that chat.
        up_to_seq: u64,
    },
    /// Typing indicator, relayed to other chat members.
    Typing {
        /// Chat where the user is (not) typing.
        chat: ChatId,
        /// `true` = started typing, `false` = stopped.
        on: bool,
    },
    /// Application-level keepalive (WS ping/pong is handled by the transport).
    Ping,
}

/// Server → client frames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerFrame {
    /// Reply to a successful [`ClientFrame::Hello`].
    Hello {
        /// Authenticated user.
        user: UserId,
        /// The user's handle.
        handle: String,
        /// Next inbox seq that will be assigned to this user.
        next_seq: u64,
        /// Server clock, unix seconds (clock-skew hint for the client).
        server_time: i64,
        /// How many backlog envelopes followed this frame immediately.
        resumed_messages: u32,
    },
    /// A fanned-out message addressed to the connected user.
    Envelope {
        /// The message.
        env: Envelope,
    },
    /// Typing indicator relayed from another chat member.
    Typing {
        /// Chat where the user is typing.
        chat: ChatId,
        /// Who is typing.
        from: UserId,
        /// Their handle.
        from_handle: String,
        /// `true` = started, `false` = stopped.
        on: bool,
    },
    /// Ack for a [`ClientFrame::Send`] — the message is durably stored and
    /// fanned out; `seq` is the sender's own inbox seq for the message.
    Sent {
        /// Echo of the client's idempotency key.
        client_msg_id: ClientMsgId,
        /// Allocated seq.
        seq: u64,
    },
    /// Typed error (never a bare string).
    Error {
        /// Machine-readable code.
        code: ErrorCode,
        /// Human-readable detail.
        message: String,
    },
    /// Reply to [`ClientFrame::Ping`].
    Pong,
}

/// Machine-readable error codes used on both WS and HTTP APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Magic bytes indicate a newer protocol than the server speaks.
    UnsupportedVersion,
    /// Missing, unknown or expired token.
    Unauthorized,
    /// Authenticated, but not allowed (e.g. not a chat member).
    Forbidden,
    /// Referenced object does not exist.
    NotFound,
    /// Too many requests from this IP/user.
    RateLimited,
    /// Frame or body exceeds size limits.
    PayloadTooLarge,
    /// Media quota exhausted and could not be freed.
    QuotaExceeded,
    /// Malformed request.
    BadRequest,
    /// Unspecified server-side failure (details in logs).
    Internal,
}

impl ErrorCode {
    /// Stable numeric code for logs and HTTP bodies.
    pub fn code(self) -> u16 {
        match self {
            ErrorCode::UnsupportedVersion => 1,
            ErrorCode::Unauthorized => 2,
            ErrorCode::Forbidden => 3,
            ErrorCode::NotFound => 4,
            ErrorCode::RateLimited => 5,
            ErrorCode::PayloadTooLarge => 6,
            ErrorCode::QuotaExceeded => 7,
            ErrorCode::BadRequest => 8,
            ErrorCode::Internal => 9,
        }
    }
}
