//! End-to-end encryption boundary.
//!
//! The protocol carries **opaque bodies** (see `koko-protocol` docs), so the
//! server never needs to know what a body contains. This module defines the
//! seam where real end-to-end encryption plugs in.
//!
//! ## Status — honest and explicit
//!
//! * **Today:** [`NoopCipher`] passes bodies through. The demo client sends
//!   UTF-8 text, so messages are readable by the server operator. Treat a
//!   v0.1 deployment as *transport-secured, not end-to-end encrypted*.
//! * **Next (tracked as the `e2e` milestone):** a `SignalCipher` based on
//!   `libsignal-protocol` (X3DH key agreement + Double Ratchet, per-chat
//!   sender keys for groups). It implements the same [`Cipher`] trait, so
//!   neither the wire protocol nor the storage schema changes — only the
//!   bytes inside `envelope.body` become ciphertext.
//!
//! Nothing in the codebase assumes bodies are readable, which is what makes
//! the swap non-breaking.

use koko_protocol::ChatId;

/// Errors from sealing/opening message bodies.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// The ciphertext is malformed or the ratchet state is out of sync.
    #[error("cipher failure: {0}")]
    Cipher(String),
}

/// A body cipher, implemented by the (future) Signal cipher and by the
/// transparent demo cipher.
pub trait Cipher: Send + Sync {
    /// Stable name (reported in diagnostics).
    fn name(&self) -> &'static str;
    /// Encrypt a plaintext body for a chat.
    fn seal(&self, chat: ChatId, plaintext: &[u8]) -> Result<Vec<u8>, SealError>;
    /// Decrypt a body for a chat.
    fn open(&self, chat: ChatId, ciphertext: &[u8]) -> Result<Vec<u8>, SealError>;
}

/// Transparent pass-through cipher used until the Signal cipher lands.
///
/// Included so that every client and the server exercise the *same* code path
/// that the real cipher will use — swapping in [`Cipher`] implementations is
/// the only change the E2E milestone requires.
pub struct NoopCipher;

impl Cipher for NoopCipher {
    fn name(&self) -> &'static str {
        "noop-transparent"
    }

    fn seal(&self, _chat: ChatId, plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
        Ok(plaintext.to_vec())
    }

    fn open(&self, _chat: ChatId, ciphertext: &[u8]) -> Result<Vec<u8>, SealError> {
        Ok(ciphertext.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_cipher_is_transparent() {
        let c = NoopCipher;
        let sealed = c.seal(ChatId(1), b"hello").unwrap();
        assert_eq!(c.open(ChatId(1), &sealed).unwrap(), b"hello");
        assert_eq!(c.name(), "noop-transparent");
    }
}
