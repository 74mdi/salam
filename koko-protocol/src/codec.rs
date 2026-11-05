//! Binary codec: magic header + bincode payload.
//!
//! ```text
//! +--------+-----------------------+
//! | "KOK1" | bincode(frame)        |
//! | 4 byte | rest of the message   |
//! +--------+-----------------------+
//! ```
//!
//! The trailing `1` in the magic is the protocol version. Everything after
//! it is `bincode` 1.3 with default options (fixint, little-endian), so the
//! layout is fully deterministic across platforms.

use crate::frame::{ClientFrame, ServerFrame};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

/// Magic prefix of every WS binary message. The last byte is the version.
pub const MAGIC: [u8; 4] = *b"KOK1";

/// Refuse to decode frames whose payload exceeds this size (defensive bound;
/// `MAX_MESSAGE_BODY` is the real, much tighter content limit).
pub const MAX_FRAME_SIZE: usize = 1024 * 1024;

/// Codec-level failure (before the payload even becomes a frame).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WireError {
    /// Fewer than 4 bytes — cannot even read the magic.
    #[error("frame too short: {len} bytes")]
    TooShort {
        /// Number of bytes received.
        len: usize,
    },
    /// Magic mismatch: likely a different protocol version or garbage.
    #[error("bad magic: expected {expected:02x?}, got {got:02x?}")]
    BadMagic {
        /// Expected magic bytes.
        expected: [u8; 4],
        /// Received magic bytes (padded/truncated to 4).
        got: [u8; 4],
    },
    /// Payload larger than [`MAX_FRAME_SIZE`].
    #[error("frame too large: {len} bytes")]
    TooLarge {
        /// Payload length in bytes.
        len: usize,
    },
    /// bincode could not deserialize the payload into the requested type.
    #[error("decode failed: {0}")]
    Decode(String),
}

impl From<bincode::Error> for WireError {
    fn from(err: bincode::Error) -> Self {
        WireError::Decode(err.to_string())
    }
}

/// Serialize any frame with the `KOK1` header.
pub fn encode<T: Serialize>(frame: &T) -> Result<Vec<u8>, WireError> {
    let payload = bincode::serialize(frame)?;
    let mut out = Vec::with_capacity(MAGIC.len() + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Deserialize a frame from raw bytes (magic + payload).
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, WireError> {
    if bytes.len() < MAGIC.len() {
        return Err(WireError::TooShort { len: bytes.len() });
    }
    let (magic, payload) = bytes.split_at(MAGIC.len());
    let mut got = [0u8; 4];
    got.copy_from_slice(&magic[..4]);
    if got != MAGIC {
        return Err(WireError::BadMagic {
            expected: MAGIC,
            got,
        });
    }
    if payload.len() > MAX_FRAME_SIZE {
        return Err(WireError::TooLarge { len: payload.len() });
    }
    bincode::deserialize(payload).map_err(WireError::from)
}

/// Encode a client frame (convenience wrapper around [`encode`]).
pub fn encode_client(frame: &ClientFrame) -> Result<Vec<u8>, WireError> {
    encode(frame)
}

/// Encode a server frame (convenience wrapper around [`encode`]).
pub fn encode_server(frame: &ServerFrame) -> Result<Vec<u8>, WireError> {
    encode(frame)
}

/// Decode a client frame (convenience wrapper around [`decode`]).
pub fn decode_client(bytes: &[u8]) -> Result<ClientFrame, WireError> {
    decode(bytes)
}

/// Decode a server frame (convenience wrapper around [`decode`]).
pub fn decode_server(bytes: &[u8]) -> Result<ServerFrame, WireError> {
    decode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ChatId;
    use crate::MessageKind;

    #[test]
    fn encode_decode_roundtrip() {
        let frame = ClientFrame::Send {
            client_msg_id: crate::ids::ClientMsgId::from_bytes([7; 16]),
            chat: ChatId(3),
            kind: MessageKind::Text,
            body: b"hello koko".to_vec(),
            created_at: 1_772_000_000,
        };
        let bytes = encode_client(&frame).unwrap();
        assert_eq!(&bytes[..4], b"KOK1");
        let back = decode_client(&bytes).unwrap();
        assert_eq!(back, frame);
    }

    #[test]
    fn rejects_bad_magic() {
        let err = decode_client(b"NOPE....").unwrap_err();
        assert!(matches!(err, WireError::BadMagic { .. }));
    }

    #[test]
    fn rejects_short_input() {
        let err = decode_client(b"KO").unwrap_err();
        assert!(matches!(err, WireError::TooShort { len: 2 }));
    }
}
