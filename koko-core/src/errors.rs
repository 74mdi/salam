//! Shared error vocabulary used across the workspace.
//!
//! `koko-core` deliberately has **no sqlx / actix dependencies**: it is pure
//! domain logic that compiles for native *and* wasm targets. Database errors
//! are wrapped where they occur (`koko-db`), and `koko-server` maps
//! everything onto HTTP statuses / [`ErrorCode`] frames.

use koko_protocol::ErrorCode;
use thiserror::Error;

/// Domain-level failure.
#[derive(Debug, Error)]
pub enum KokoError {
    /// Handle (or display name) failed validation.
    #[error("{0}")]
    InvalidHandle(#[from] crate::handles::HandleError),
    /// Password failed validation or hashing.
    #[error("{0}")]
    Password(#[from] crate::password::PasswordError),
    /// Password does not match the stored hash.
    #[error("invalid credentials")]
    BadCredentials,
    /// Object (user/chat/media/session) does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// Authenticated but not permitted (e.g. not a chat member).
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// Handle already registered.
    #[error("handle already taken")]
    HandleTaken,
    /// Rate limit exceeded.
    #[error("rate limited")]
    RateLimited,
    /// Payload exceeds a size limit.
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),
    /// Media quota exhausted.
    #[error("media quota exceeded")]
    QuotaExceeded,
    /// Registration disabled on this server.
    #[error("registration is closed on this server")]
    RegistrationClosed,
    /// Unspecified internal failure — details are logged server-side only.
    #[error("internal error")]
    Internal,
    /// Anything else. The message is safe to show to a client.
    #[error("{0}")]
    BadRequest(String),
}

impl KokoError {
    /// Map onto the protocol's machine-readable code.
    pub fn code(&self) -> ErrorCode {
        match self {
            KokoError::InvalidHandle(_) | KokoError::Password(_) | KokoError::BadRequest(_) => {
                ErrorCode::BadRequest
            }
            KokoError::BadCredentials => ErrorCode::Unauthorized,
            KokoError::NotFound(_) => ErrorCode::NotFound,
            KokoError::Forbidden(_) | KokoError::RegistrationClosed => ErrorCode::Forbidden,
            KokoError::HandleTaken => ErrorCode::BadRequest,
            KokoError::RateLimited => ErrorCode::RateLimited,
            KokoError::PayloadTooLarge(_) => ErrorCode::PayloadTooLarge,
            KokoError::QuotaExceeded => ErrorCode::QuotaExceeded,
            KokoError::Internal => ErrorCode::Internal,
        }
    }
}
