//! HTTP API. Everything lives under `/api`; health and root sit at `/`.

pub mod auth;
pub mod chats;
pub mod health;
pub mod media;
pub mod sync;
pub mod users;

use actix_web::{get, web};

/// Mount the `/api` scope (called from `configure_routes`).
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth::register)
        .service(auth::login)
        .service(auth::logout)
        .service(auth::logout_all)
        .service(auth::me)
        .service(auth::update_profile)
        .service(users::find)
        .service(users::get_user)
        .service(chats::create)
        .service(chats::note_to_self)
        .service(chats::list)
        .service(chats::chat_messages)
        .service(chats::read)
        .service(chats::add_member)
        .service(sync::inbox)
        .service(media::upload)
        .service(media::download)
        .service(media::remove);
}

/// `GET /` — tiny service descriptor for curious browsers.
#[get("/")]
pub async fn root() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "name": "koko-server",
        "version": koko_core::KOKO_VERSION,
        "websocket": "/ws",
        "docs": "https://github.com/koko-chat/koko"
    }))
}
