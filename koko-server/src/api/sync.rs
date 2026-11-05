//! The REST sync fallback: replay the global inbox by cursor.
//!
//! Used (a) after WS backlog overflow, (b) after `Lagged`, (c) by clients
//! that want a cold boot without the WS handshake.

use actix_web::{get, web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::state::{AppState, AuthedUser};

#[derive(Debug, Deserialize)]
pub struct InboxQuery {
    /// Replay envelopes with seq greater than this.
    #[serde(default)]
    pub since: u64,
    /// Page size (default 500, max 1000).
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    500
}

/// `GET /api/messages?since=0&limit=500`
#[get("/messages")]
pub async fn inbox(
    auth: AuthedUser,
    state: web::Data<AppState>,
    query: web::Query<InboxQuery>,
) -> Result<HttpResponse, ApiError> {
    let limit = query.limit.clamp(1, 1000);
    let messages =
        koko_db::messages::fetch_since(&state.db, auth.user_id, query.since, limit).await?;
    let next_cursor = koko_db::messages::current_seq(&state.db, auth.user_id).await?;
    Ok(HttpResponse::Ok().json(json!({
        "messages": messages,
        "next_cursor": next_cursor,
    })))
}
