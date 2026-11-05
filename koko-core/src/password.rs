//! Password hashing with Argon2id.
//!
//! ## Parameter choice and its tradeoff (read this before changing)
//!
//! Koko servers are specified to run on as little as **1 vCPU / 2 GB RAM**.
//! Argon2's memory-hardness is the expensive axis, so the parameters are
//! deliberately tuned *down* from the OWASP-recommended 19 MiB-and-up class:
//!
//! * `m = 32 MiB`, `t = 2`, `p = 1`
//!
//! This keeps: peak hashing memory ~35 MB (safe next to a 2 GB budget even
//! with concurrent logins), and latency in the tens of milliseconds on a
//! weak vCPU (login stays snappy; a login-storm cannot OOM the box).
//!
//! The cost: a well-funded GPU rig gets more attempts per second than with
//! 128 MiB parameters. For a single-tenant self-hosted server holding
//! usernames only (no email, no payment data) this is the right trade —
//! availability of a $4 box beats a marginal hardness bump. If you run Koko
//! on beefier hardware, raise `m` via the constants below and re-hash lazily
//! on next login (the PHC string stores the parameters, so old hashes keep
//! verifying while new ones use the stronger settings).

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use thiserror::Error;

/// Memory cost in KiB (32 MiB).
pub const ARGON2_M_KIB: u32 = 32 * 1024;
/// Time cost (passes).
pub const ARGON2_T: u32 = 2;
/// Parallelism (lanes).
pub const ARGON2_P: u32 = 1;

/// Password-related failures.
#[derive(Debug, Error)]
pub enum PasswordError {
    /// Password is shorter than the minimum length.
    #[error("password must be at least {0} characters")]
    TooShort(usize),
    /// Cryptographic hash failure (malformed stored hash, param mismatch).
    #[error("hash error: {0}")]
    Hash(String),
}

const MIN_PASSWORD_LEN: usize = 8;

fn hasher() -> Result<Argon2<'static>, PasswordError> {
    let params = Params::new(ARGON2_M_KIB, ARGON2_T, ARGON2_P, None)
        .map_err(|e| PasswordError::Hash(e.to_string()))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Hash a password with the tuned Argon2id parameters.
///
/// Returns a self-contained PHC string (`$argon2id$v=19$m=32768,t=2,p=1$…`)
/// that embeds the parameters, a random salt, and the derived tag — store it
/// verbatim in the `users.password_hash` column.
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(PasswordError::TooShort(MIN_PASSWORD_LEN));
    }
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    hasher()?
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| PasswordError::Hash(e.to_string()))
}

/// Verify a password against a stored PHC string.
///
/// Returns `Ok(false)` when the password simply does not match; returns an
/// error only when the stored hash is malformed.
pub fn verify_password(password: &str, stored: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(stored).map_err(|e| PasswordError::Hash(e.to_string()))?;
    // Verify with the *stored* parameters so upgraded hashes still work.
    let ok = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.contains("m=32768,t=2,p=1"));
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_password("wrong password", &hash).unwrap());
    }

    #[test]
    fn rejects_short_passwords() {
        assert!(matches!(
            hash_password("short"),
            Err(PasswordError::TooShort(8))
        ));
    }

    #[test]
    fn unique_salts_per_hash() {
        let a = hash_password("same-password").unwrap();
        let b = hash_password("same-password").unwrap();
        assert_ne!(a, b, "every hash must use a fresh random salt");
    }
}
