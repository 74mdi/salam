//! The WebSocket session: `/ws`, one binary frame = one protocol frame.
//!
//! Lifecycle:
//!
//! ```text
//! connect → ClientFrame::Hello{token, resume_from?} (10 s deadline)
//!         → ServerFrame::Hello{next_seq, ...}
//!         → [backlog envelopes when resuming]
//!         → live loop: select! { incoming frames | hub events }
//!         → close
//! ```
//!
//! Design notes:
//!
//! * The session owns a broadcast subscription and filters events by its
//!   user id — fanout cost per message is O(subscribers), not O(users).
//! * Sends happen inline; a slow client manifests as `Lagged`, after which
//!   the client is told to REST-catch-up. Slow sockets can never stall the
//!   write transaction that allocates seqs.
//! * Every send from a client is idempotent (`client_msg_id`), so a dropped
//!   connection mid-send is always safe to retry.

use std::time::Duration;

use actix_web::web;
use actix_web::{HttpRequest, HttpResponse};
use actix_ws::{Message, MessageStream, Session};
use futures_util::StreamExt;
use koko_protocol::codec::{decode_client, encode_server};
use koko_protocol::{ChatId, ClientFrame, ClientMsgId, Envelope, MessageKind, ServerFrame};

use crate::error::ApiError;
use crate::hub::HubEvent;
use crate::state::{AppState, AuthedUser};

/// How long a socket may stay unauthenticated after connecting.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// Backlog replay cap right after `Hello` (bigger catch-ups go via REST).
const BACKLOG_CAP: i64 = 256;
/// Per-socket send budget (abuse guard on top of transport limits).
const SEND_BUDGET_MAX: u32 = 120;
const SEND_BUDGET_WINDOW: Duration = Duration::from_secs(60);

/// `GET /ws`
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Enforce the connection cap before spending resources on the upgrade.
    if state.hub.connections() >= state.cfg.max_connections {
        return Ok(HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": { "code": 5, "message": "connection limit reached" }
        })));
    }

    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    state.hub.inc_connections();
    let state_for_guard = state.clone();
    actix_web::rt::spawn(async move {
        session_loop(session, msg_stream, state).await;
        state_for_guard.hub.dec_connections();
    });

    Ok(response)
}

/// Send one server frame (best-effort: socket closure is detected by the
/// read side ending, not by individual send results).
async fn send(session: &mut Session, frame: &ServerFrame) {
    match encode_server(frame) {
        Ok(bytes) => {
            let _ = session.binary(bytes).await;
        }
        Err(e) => log::debug!("frame encode failed: {e}"),
    }
}

/// The whole session, start to close.
async fn session_loop(
    mut session: Session,
    mut msg_stream: MessageStream,
    state: web::Data<AppState>,
) {
    let Some(authed) = greet(&mut session, &mut msg_stream, &state).await else {
        let _ = session.close(None).await;
        return;
    };

    // ---- Live loop --------------------------------------------------------
    let mut rx = state.hub.subscribe();
    let mut budget = SendBudget::default();

    loop {
        tokio::select! {
            maybe = msg_stream.next() => {
                match maybe {
                    None | Some(Err(_) | Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(b))) => { let _ = session.pong(&b[..]).await; }
                    Some(Ok(Message::Binary(bytes))) => {
                        handle_client_frame(
                            &mut session, &state, &authed, &bytes, &mut budget,
                        ).await;
                    }
                    // Text frames, pongs, etc. — the protocol is binary-only.
                    Some(Ok(_)) => {}
                }
            }
            ev = rx.recv() => {
                match ev {
                    Ok(HubEvent::Message { recipient, envelope }) => {
                        if recipient.0 == authed.user_id {
                            send(&mut session, &ServerFrame::Envelope { env: envelope }).await;
                        }
                    }
                    Ok(HubEvent::Typing { recipient, chat, from, from_handle, on }) => {
                        if recipient.0 == authed.user_id && from.0 != authed.user_id {
                            send(&mut session, &ServerFrame::Typing { chat, from, from_handle, on }).await;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("ws: {} lagged by {n} events, advising REST catch-up", authed.handle);
                        send(&mut session, &ServerFrame::Error {
                            code: koko_protocol::ErrorCode::Internal,
                            message: format!("lagged {n} events; re-sync via /api/messages"),
                        }).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    log::info!("ws: {} disconnected", authed.handle);
    let _ = session.close(None).await;
}

/// What `wait_for_hello` saw.
enum HelloOutcome {
    /// Got a hello frame: (token, `resume_from`).
    Hello {
        /// Raw token.
        token: String,
        /// Cursor to replay from, if any.
        resume_from: Option<u64>,
    },
    /// Peer went away or sent nothing in time.
    Closed,
    /// No frame within [`HELLO_TIMEOUT`].
    Timeout,
}

/// Phase 1+2: authenticate the socket, send `Hello`, replay the backlog.
/// Returns `None` when the session should end (the caller closes the
/// socket — `Session::close` consumes the session, so only its owner may).
async fn greet(
    session: &mut Session,
    msg_stream: &mut MessageStream,
    state: &web::Data<AppState>,
) -> Option<AuthedUser> {
    let (token, resume_from) = match wait_for_hello(msg_stream).await {
        HelloOutcome::Hello { token, resume_from } => (token, resume_from),
        HelloOutcome::Timeout => {
            log::debug!("ws: no hello within 10s");
            return None;
        }
        HelloOutcome::Closed => return None,
    };

    let authed = match state.authenticate(&token).await {
        Ok(a) => a,
        Err(e) => {
            let code = match e.domain() {
                Some(d) => d.code(),
                None => koko_protocol::ErrorCode::Internal,
            };
            send(
                session,
                &ServerFrame::Error {
                    code,
                    message: "authentication failed".into(),
                },
            )
            .await;
            return None;
        }
    };
    log::info!(
        "ws: {} connected (device {})",
        authed.handle,
        authed.device_id
    );

    let next_seq = match koko_db::messages::current_seq(&state.db, authed.user_id).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("ws: cursor read failed: {e}");
            return None;
        }
    };

    let backlog = match resume_from {
        Some(from) => koko_db::messages::fetch_since(&state.db, authed.user_id, from, BACKLOG_CAP)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let resumed = backlog.len() as u32;
    send(
        session,
        &ServerFrame::Hello {
            user: koko_protocol::UserId(authed.user_id),
            handle: authed.handle.clone(),
            next_seq,
            server_time: koko_core::time::unix_now(),
            resumed_messages: resumed,
        },
    )
    .await;
    for env in backlog {
        send(session, &ServerFrame::Envelope { env }).await;
    }
    Some(authed)
}

async fn wait_for_hello(msg_stream: &mut MessageStream) -> HelloOutcome {
    let first = tokio::time::timeout(HELLO_TIMEOUT, msg_stream.next()).await;
    let bytes = match first {
        Err(_) => return HelloOutcome::Timeout,
        Ok(Some(Ok(Message::Binary(bytes)))) => bytes,
        // Closed, protocol error, or a non-binary first frame.
        Ok(None | Some(_)) => return HelloOutcome::Closed,
    };
    match decode_client(&bytes) {
        Ok(ClientFrame::Hello { token, resume_from }) => HelloOutcome::Hello { token, resume_from },
        _ => HelloOutcome::Closed, // first frame must be a Hello
    }
}

/// Fixed-window send budget per socket.
struct SendBudget {
    window_start: std::time::Instant,
    count: u32,
}

impl Default for SendBudget {
    fn default() -> Self {
        SendBudget {
            window_start: std::time::Instant::now(),
            count: 0,
        }
    }
}

impl SendBudget {
    fn allow(&mut self) -> bool {
        if self.window_start.elapsed() >= SEND_BUDGET_WINDOW {
            self.window_start = std::time::Instant::now();
            self.count = 0;
        }
        self.count += 1;
        self.count <= SEND_BUDGET_MAX
    }
}

async fn handle_client_frame(
    session: &mut Session,
    state: &web::Data<AppState>,
    authed: &AuthedUser,
    bytes: &[u8],
    budget: &mut SendBudget,
) {
    let frame = match decode_client(bytes) {
        Ok(f) => f,
        Err(e) => {
            send(
                session,
                &err_frame(koko_protocol::ErrorCode::BadRequest, &e.to_string()),
            )
            .await;
            return;
        }
    };
    match frame {
        ClientFrame::Hello { .. } => {
            send(
                session,
                &err_frame(
                    koko_protocol::ErrorCode::BadRequest,
                    "already authenticated",
                ),
            )
            .await;
        }
        ClientFrame::Ping => {
            send(session, &ServerFrame::Pong).await;
        }
        ClientFrame::Send {
            client_msg_id,
            chat,
            kind,
            body,
            created_at,
        } => {
            process_send(
                session,
                state,
                authed,
                budget,
                client_msg_id,
                chat,
                kind,
                body,
                created_at,
            )
            .await;
        }
        ClientFrame::Ack { chat, up_to_seq } => {
            let _ =
                koko_db::chats::set_last_read(&state.db, chat.0, authed.user_id, up_to_seq as i64)
                    .await;
        }
        ClientFrame::Typing { chat, on } => {
            // Relay to the other members of the chat.
            if let Ok(members) = koko_db::chats::member_ids(&state.db, chat.0).await {
                for uid in members {
                    if uid == authed.user_id {
                        continue;
                    }
                    state.hub.publish(HubEvent::Typing {
                        recipient: koko_protocol::UserId(uid),
                        chat,
                        from: koko_protocol::UserId(authed.user_id),
                        from_handle: authed.handle.clone(),
                        on,
                    });
                }
            }
        }
    }
}

/// The `Send` path: budget → size check → idempotent insert → fanout → ack.
#[allow(clippy::too_many_arguments)]
async fn process_send(
    session: &mut Session,
    state: &web::Data<AppState>,
    authed: &AuthedUser,
    budget: &mut SendBudget,
    client_msg_id: ClientMsgId,
    chat: ChatId,
    kind: MessageKind,
    body: Vec<u8>,
    created_at: i64,
) {
    if !budget.allow() {
        send(
            session,
            &err_frame(
                koko_protocol::ErrorCode::RateLimited,
                "send budget exhausted",
            ),
        )
        .await;
        return;
    }
    if body.len() > koko_protocol::MAX_MESSAGE_BODY {
        send(
            session,
            &err_frame(
                koko_protocol::ErrorCode::PayloadTooLarge,
                "message body exceeds 256 KiB; use the media store",
            ),
        )
        .await;
        return;
    }
    let new_msg = koko_db::messages::NewMessage {
        chat_id: chat.0,
        sender_id: authed.user_id,
        client_msg_id: client_msg_id.0,
        kind: kind.code(),
        body: &body,
        created_at,
    };
    match koko_db::messages::insert_message(&state.db, &new_msg).await {
        Ok(fanout) => {
            if fanout.duplicate {
                // Already stored & fanned out on a previous attempt:
                // no re-publish (online members saw it; offline members
                // get it via sync). The Sent ack still goes out so the
                // client can clear its retry queue.
                log::debug!("ws: duplicate send from {} (idempotent)", authed.handle);
            } else {
                for r in &fanout.recipients {
                    state.hub.publish(HubEvent::Message {
                        recipient: koko_protocol::UserId(r.user_id),
                        envelope: Envelope {
                            seq: r.seq,
                            chat,
                            sender: koko_protocol::UserId(authed.user_id),
                            sender_handle: authed.handle.clone(),
                            kind,
                            body: body.clone(),
                            created_at,
                        },
                    });
                }
            }
            send(
                session,
                &ServerFrame::Sent {
                    client_msg_id,
                    seq: fanout.sender_seq,
                },
            )
            .await;
        }
        Err(e) => {
            let err: ApiError = e.into();
            let code = err.domain().map_or(
                koko_protocol::ErrorCode::Internal,
                koko_core::KokoError::code,
            );
            send(session, &err_frame(code, &err.to_string())).await;
        }
    }
}

fn err_frame(code: koko_protocol::ErrorCode, message: &str) -> ServerFrame {
    ServerFrame::Error {
        code,
        message: message.to_owned(),
    }
}
