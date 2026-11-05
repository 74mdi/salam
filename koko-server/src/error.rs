//! One error type crossing the API boundary, mapped to HTTP + JSON.

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use koko_core::KokoError;
use koko_db::DbError;
use serde_json::json;

/// Errors convertible into HTTP responses.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Domain error (the vocabulary shared with the WS protocol).
    #[error("{0}")]
    Domain(#[from] KokoError),
    /// Filesystem failure (media store).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        ApiError::Domain(err.into())
    }
}

impl From<koko_core::handles::HandleError> for ApiError {
    fn from(err: koko_core::handles::HandleError) -> Self {
        ApiError::Domain(err.into())
    }
}

impl From<koko_core::password::PasswordError> for ApiError {
    fn from(err: koko_core::password::PasswordError) -> Self {
        ApiError::Domain(err.into())
    }
}

impl ApiError {
    /// The domain error, if this is one.
    pub fn domain(&self) -> Option<&KokoError> {
        match self {
            ApiError::Domain(e) => Some(e),
            ApiError::Io(_) => None,
        }
    }

    /// HTTP status for this error.
    fn status(&self) -> StatusCode {
        match self.domain() {
            Some(
                KokoError::InvalidHandle(_) | KokoError::Password(_) | KokoError::BadRequest(_),
            ) => StatusCode::BAD_REQUEST,
            Some(KokoError::BadCredentials) => StatusCode::UNAUTHORIZED,
            Some(KokoError::NotFound(_)) => StatusCode::NOT_FOUND,
            Some(KokoError::Forbidden(_) | KokoError::RegistrationClosed) => StatusCode::FORBIDDEN,
            Some(KokoError::HandleTaken) => StatusCode::CONFLICT,
            Some(KokoError::RateLimited) => StatusCode::TOO_MANY_REQUESTS,
            Some(KokoError::PayloadTooLarge(_) | KokoError::QuotaExceeded) => {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            Some(KokoError::Internal) | None => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status()
    }

    fn error_response(&self) -> HttpResponse {
        let code = self
            .domain()
            .map_or(koko_protocol::ErrorCode::Internal, KokoError::code);
        // Internal details never leak to clients — they live in the log only.
        let message = match self.domain() {
            Some(KokoError::Internal) | None => "internal error".to_string(),
            Some(_) => self.to_string(),
        };
        HttpResponse::build(self.status()).json(json!({
            "error": { "code": code.code(), "message": message }
        }))
    }
}
