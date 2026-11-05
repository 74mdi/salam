//! Chats: create (direct/group), list, page messages, read receipts, members.

use actix_web::{get, post, web, HttpResponse};
use koko_core::time::unix_now;
use koko_core::KokoError;
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::state::{AppState, AuthedUser};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CreateChatRequest {
    /// Open (or find) the direct chat with `peer`.
    Direct {
        /// Peer handle.
        peer: String,
    },
    /// Create a group chat.
    Group {
        /// Group title (1–128 chars).
        title: String,
        /// Member handles (creator is added automatically).
        #[serde(default)]
        members: Vec<String>,
    },
}

/// `POST /api/chats`
#[post("/chats")]
pub async fn create(
    auth: AuthedUser,
    state: web::Data<AppState>,
    body: web::Json<CreateChatRequest>,
) -> Result<HttpResponse, ApiError> {
    let now = unix_now();
    let chat_id = match body.into_inner() {
        CreateChatRequest::Direct { peer } => {
            let peer_user = koko_db::users::get_by_handle(&state.db, &peer)
                .await?
                .ok_or_else(|| KokoError::NotFound(format!("user {peer}")))?;
            if peer_user.id == auth.user_id {
                return Err(ApiError::from(KokoError::BadRequest(
                    "cannot open a direct chat with yourself".into(),
                )));
            }
            koko_db::chats::direct_chat(&state.db, auth.user_id, peer_user.id, now).await?
        }
        CreateChatRequest::Group { title, members } => {
            let title_len = title.chars().count();
            if !(1..=128).contains(&title_len) {
                return Err(ApiError::from(KokoError::BadRequest(
                    "group title must be 1-128 characters".into(),
                )));
            }
            // Resolve handles → ids (dedup, keep order, skip unknown handles).
            let mut ids = vec![auth.user_id];
            for handle in members {
                let Some(u) = koko_db::users::get_by_handle(&state.db, &handle).await? else {
                    return Err(ApiError::from(KokoError::NotFound(format!(
                        "user {handle}"
                    ))));
                };
                if !ids.contains(&u.id) {
                    ids.push(u.id);
                }
            }
            koko_db::chats::create_group(&state.db, auth.user_id, &title, &ids, now).await?
        }
    };
    Ok(HttpResponse::Created().json(json!({ "chat_id": chat_id })))
}

/// `POST /api/chats/self` — a private one-person chat for notes.
#[post("/chats/self")]
pub async fn note_to_self(
    auth: AuthedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let chat_id = koko_db::chats::note_to_self(&state.db, auth.user_id, unix_now()).await?;
    Ok(HttpResponse::Created().json(json!({ "chat_id": chat_id })))
}

/// `GET /api/chats` — the chat list, most recently active first.
#[get("/chats")]
pub async fn list(auth: AuthedUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let chats = koko_db::chats::list_chats(&state.db, auth.user_id).await?;
    let out: Vec<_> = chats
        .into_iter()
        .map(|c| {
            json!({
                "id": c.id,
                "kind": if c.kind == 1 { "direct" } else { "group" },
                "title": c.title,
                "members": c.members.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                "member_count": c.member_count,
                "last_activity": c.last_activity,
                "last_kind": c.last_kind,
                "unread": c.unread,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    /// Only messages with row id greater than this.
    #[serde(default)]
    pub after: Option<i64>,
    /// Page size (default 100, max 500).
    #[serde(default = "default_page_limit")]
    pub limit: i64,
}

fn default_page_limit() -> i64 {
    100
}

/// `GET /api/chats/{id}/messages?after=&limit=`
#[get("/chats/{id}/messages")]
pub async fn chat_messages(
    auth: AuthedUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let chat_id = path.into_inner();
    if !koko_db::chats::is_member(&state.db, chat_id, auth.user_id).await? {
        return Err(ApiError::from(KokoError::Forbidden(format!(
            "not a member of chat {chat_id}"
        ))));
    }
    let after = query.after.unwrap_or(0);
    let limit = query.limit.clamp(1, 500);
    let rows =
        koko_db::messages::fetch_chat_messages(&state.db, auth.user_id, chat_id, after, limit)
            .await?;
    let out: Vec<_> = rows
        .into_iter()
        .map(|m| {
            json!({
                "message_id": m.message_id,
                "seq": m.seq,
                "sender_id": m.sender_id,
                "sender_handle": m.handle,
                "kind": m.kind,
                "body": m.body,
                "created_at": m.created_at,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

#[derive(Debug, Deserialize)]
pub struct ReadRequest {
    /// Highest seq considered read.
    pub up_to_seq: i64,
}

/// `POST /api/chats/{id}/read`
#[post("/chats/{id}/read")]
pub async fn read(
    auth: AuthedUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<ReadRequest>,
) -> Result<HttpResponse, ApiError> {
    let chat_id = path.into_inner();
    if !koko_db::chats::is_member(&state.db, chat_id, auth.user_id).await? {
        return Err(ApiError::from(KokoError::Forbidden(format!(
            "not a member of chat {chat_id}"
        ))));
    }
    koko_db::chats::set_last_read(&state.db, chat_id, auth.user_id, body.up_to_seq).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    /// Handle to add.
    pub handle: String,
}

/// `POST /api/chats/{id}/members` — add a member to a group (v0.1: any
/// member may add; see docs/ARCHITECTURE.md#roadmap).
#[post("/chats/{id}/members")]
pub async fn add_member(
    auth: AuthedUser,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<AddMemberRequest>,
) -> Result<HttpResponse, ApiError> {
    let chat_id = path.into_inner();
    let chat = koko_db::chats::get_chat(&state.db, chat_id)
        .await?
        .ok_or_else(|| KokoError::NotFound(format!("chat {chat_id}")))?;
    if chat.kind != koko_db::chats::chat_kind::GROUP {
        return Err(ApiError::from(KokoError::BadRequest(
            "members can only be added to group chats".into(),
        )));
    }
    if !koko_db::chats::is_member(&state.db, chat_id, auth.user_id).await? {
        return Err(ApiError::from(KokoError::Forbidden(format!(
            "not a member of chat {chat_id}"
        ))));
    }
    let user = koko_db::users::get_by_handle(&state.db, &body.handle)
        .await?
        .ok_or_else(|| KokoError::NotFound(format!("user {}", body.handle)))?;
    koko_db::chats::add_member(&state.db, chat_id, user.id, unix_now()).await?;
    Ok(HttpResponse::NoContent().finish())
}
