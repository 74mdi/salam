//! Shared application state.

use std::time::Instant;

use sqlx::SqlitePool;

use crate::config::Config;
use crate::error::ApiError;
use crate::hub::Hub;
use crate::limits::RateLimiter;

/// The single state object shared by every handler and WS session.
pub struct AppState {
    /// SQLite pool (WAL).
    pub db: SqlitePool,
    /// In-process fanout + connection counter.
    pub hub: Hub,
    /// Resolved config.
    pub cfg: std::sync::Arc<Config>,
    /// Fixed-window rate limiter (register/login/media).
    pub limiter: RateLimiter,
    /// Server start time (uptime, banner).
    pub started: Instant,
    /// A hash verified against failed logins for *missing* users, equalizing
    /// the "no such user" and "wrong password" code paths (enumeration
    /// defense). Computed once at startup.
    pub dummy_hash: String,
}

/// Identity attached to an authenticated request / socket.
#[derive(Debug, Clone)]
pub struct AuthedUser {
    /// Numeric user id.
    pub user_id: i64,
    /// Handle.
    pub handle: String,
    /// Device id for this session.
    pub device_id: i64,
}

impl AppState {
    /// Build state (computes the login-timing equalizer hash once).
    pub fn new(db: SqlitePool, hub: Hub, cfg: std::sync::Arc<Config>) -> Self {
        let dummy_hash = koko_core::password::hash_password("koko-timing-equalizer")
            .unwrap_or_else(|_| String::new());
        AppState {
            db,
            hub,
            cfg,
            limiter: RateLimiter::new(),
            started: Instant::now(),
            dummy_hash,
        }
    }

    /// Resolve a raw session token into an identity.
    pub async fn authenticate(&self, token: &str) -> Result<AuthedUser, ApiError> {
        let hash = koko_core::token::token_hash(token);
        let sess = koko_db::sessions::lookup_session(&self.db, &hash, koko_core::time::unix_now())
            .await?
            .ok_or(ApiError::from(koko_core::KokoError::BadCredentials))?;
        // Opportunistic device heartbeat.
        let db = self.db.clone();
        let device_id = sess.device_id;
        let now = koko_core::time::unix_now();
        actix_web::rt::spawn(async move {
            let _ = koko_db::devices::touch_device(&db, device_id, now).await;
        });
        Ok(AuthedUser {
            user_id: sess.user_id,
            handle: sess.handle,
            device_id: sess.device_id,
        })
    }
}
