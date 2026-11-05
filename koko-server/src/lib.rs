//! # koko-server
//!
//! One binary. One SQLite file. Zero external services.
//!
//! * HTTP: auth + sync + media under `/api`, health at `/healthz`.
//! * WebSocket: `/ws` — see [`ws`] for the session lifecycle.
//! * Fanout: an in-process `tokio::sync::broadcast` hub ([`hub`]); SQLite is
//!   the source of truth, the hub is only the fast path to online sockets.
//!
//! This crate is both a library (used by the integration tests and any
//! embedding) and a binary (`koko-server`, see `main.rs`).

pub mod api;
pub mod config;
pub mod error;
pub mod hub;
pub mod limits;
pub mod state;
pub mod ws;

use actix_web::web;

/// Mount every route (used by `main.rs` and the test harness).
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(api::root)
        .service(api::health::health)
        .route("/ws", web::get().to(ws::ws_route))
        .service(web::scope("/api").configure(api::configure));
}
