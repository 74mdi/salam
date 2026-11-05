//! The message envelope — the only thing that travels when a message moves.

use crate::ids::{ChatId, UserId};
use serde::{Deserialize, Serialize};

/// What kind of payload the (opaque) body carries.
///
/// Encoded on the wire as the variant index (bincode fixint `u32`).
/// Add new kinds only at the end — never reorder, never renumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageKind {
    /// UTF-8 text (in the E2E future: ciphertext that decrypts to text).
    Text = 0,
    /// Reference to media: small JSON/struct payload with media id.
    Image = 1,
    /// Voice note (Opus in an Ogg container), referenced like Image.
    Voice = 2,
    /// Video clip, referenced like Image.
    Video = 3,
    /// Arbitrary file, referenced like Image.
    File = 4,
    /// Server/client-generated notice ("mira joined the group").
    System = 5,
}

impl MessageKind {
    /// Stable numeric code (also used in the SQLite `messages.kind` column).
    pub fn code(self) -> i64 {
        self as i64
    }

    /// Inverse of [`MessageKind::code`]. Unknown codes become [`MessageKind::System`].
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => MessageKind::Text,
            1 => MessageKind::Image,
            2 => MessageKind::Voice,
            3 => MessageKind::Video,
            4 => MessageKind::File,
            _ => MessageKind::System,
        }
    }
}

/// One message, fanned out per recipient.
///
/// `seq` is **per recipient**: every user has their own monotonic inbox
/// sequence, allocated inside the same SQLite transaction that stores the
/// message. A reconnecting client says "I have everything up to seq 41" and
/// the server replays 42.. — that cursor is all the sync state there is.
///
/// `body` is opaque to the server (see the crate docs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    /// Recipient's inbox sequence number (starts at 1).
    pub seq: u64,
    /// Chat the message belongs to.
    pub chat: ChatId,
    /// Sender's user id.
    pub sender: UserId,
    /// Sender's handle, copied in for display convenience.
    pub sender_handle: String,
    /// Payload kind.
    pub kind: MessageKind,
    /// Opaque payload (see [`MessageKind`] for expected contents).
    pub body: Vec<u8>,
    /// Unix seconds, assigned by the client, kept by the server.
    pub created_at: i64,
}
