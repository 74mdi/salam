//! Session tokens: random on the wire, hashed at rest.
//!
//! * Login mints a 32-byte random token (base64url, 43 chars) that the client
//!   keeps in memory / its local store.
//! * The server stores only `SHA-256(token)` in SQLite. A stolen database
//!   yields no usable credentials.
//! * Verification hashes the presented token and looks the digest up in a
//!   unique index. Because a valid token is 256 bits of uniform randomness,
//!   timing side channels on the index probe reveal nothing exploitable;
//!   in-memory comparisons (used in tests and future hot paths) go through
//!   `subtle`'s constant-time equality.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Length of the raw token entropy, in bytes.
pub const TOKEN_BYTES: usize = 32;
/// Length of the base64url token string shown to clients.
pub const TOKEN_STR_LEN: usize = 43;

/// Mint a fresh token (`OsRng`).
pub fn new_token() -> String {
    let mut raw = [0u8; TOKEN_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}

/// The digest stored at rest for a token string.
pub fn token_hash(token: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, token.as_bytes());
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

/// Constant-time equality of two 32-byte digests.
pub fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.ct_eq(b).unwrap_u8() == 1
}

/// Random 16-byte idempotency key for message sends.
pub fn new_client_msg_id() -> [u8; 16] {
    let mut raw = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_correct_length() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), TOKEN_STR_LEN);
        assert_ne!(a, b);
    }

    #[test]
    fn hash_is_deterministic_and_32_bytes() {
        let h1 = token_hash("abc");
        let h2 = token_hash("abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
        assert_ne!(token_hash("abd"), h1);
        assert!(constant_time_eq(&h1, &h2));
    }
}
