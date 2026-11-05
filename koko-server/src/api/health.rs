//! `GET /healthz` — unauthenticated, safe to expose to a load balancer.
//! Reports counts and RSS only; no user data.

use actix_web::{get, web, HttpResponse};
use serde_json::json;

use crate::state::AppState;

/// `GET /healthz`
#[get("/healthz")]
pub async fn health(state: web::Data<AppState>) -> HttpResponse {
    let users = koko_db::users::count_users(&state.db).await.unwrap_or(-1);
    let messages = koko_db::messages::count_messages(&state.db)
        .await
        .unwrap_or(-1);
    let media_bytes = koko_db::media::total_bytes(&state.db).await.unwrap_or(-1);
    HttpResponse::Ok().json(json!({
        "ok": true,
        "name": "koko-server",
        "version": koko_core::KOKO_VERSION,
        "uptime_secs": state.started.elapsed().as_secs(),
        "connections": state.hub.connections(),
        "max_connections": state.cfg.max_connections,
        "users": users,
        "messages": messages,
        "media_bytes": media_bytes,
        "rss_kib": rss_kib(),
    }))
}

/// RSS from /proc (Linux); `null` elsewhere.
fn rss_kib() -> Option<i64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kib: i64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kib);
        }
    }
    None
}
