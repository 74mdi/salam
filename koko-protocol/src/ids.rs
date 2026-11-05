//! Newtype IDs used across the protocol.
//!
//! Wrapping raw `i64` values keeps `UserId` and `ChatId` from being mixed up
//! at compile time — an easy class of server bugs eliminated by construction.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A user's numeric id, allocated by the server at registration.
/// `serde(transparent)`: JSON sees a bare number, bincode sees the raw i64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub i64);

impl UserId {
    /// Sentinel used when no user is known (never a valid database row).
    pub const NONE: UserId = UserId(0);
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "user:{}", self.0)
    }
}

/// A chat's numeric id. Chats are direct (2 members) or groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChatId(pub i64);

impl ChatId {
    /// Sentinel used when no chat is known (never a valid database row).
    pub const NONE: ChatId = ChatId(0);
}

impl fmt::Display for ChatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chat:{}", self.0)
    }
}

/// Client-generated unique id for a send attempt.
///
/// The client fills 16 random bytes. The server stores it with the message
/// and uses `(chat, sender, client_msg_id)` as an idempotency key: if a
/// connection drops after the insert but before the ack, a client retry of
/// the same [`ClientMsgId`] returns the original sequence number instead of
/// duplicating the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientMsgId(pub [u8; 16]);

impl ClientMsgId {
    /// Wrap 16 raw bytes. Prefer generating ids in `koko-core` (which owns
    /// the `rand` dependency) via [`crate::ids::ClientMsgId::from_bytes`].
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        ClientMsgId(bytes)
    }

    /// The raw bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for ClientMsgId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}
