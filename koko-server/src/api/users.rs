//! User lookup (finding people to chat with).

use actix_web::{get, web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::state::{AppState, AuthedUser};

#[derive(Debug, Deserialize)]
pub struct FindQuery {
    /// Handle prefix (min 2 chars).
    pub q: String,
}

/// `GET /api/users?q=pre` — prefix search.
#[get("/users")]
pub async fn find(
    auth: AuthedUser,
    state: web::Data<AppState>,
    query: web::Query<FindQuery>,
) -> Result<HttpResponse, ApiError> {
    if query.q.chars().count() < 2 {
        return Ok(HttpResponse::Ok().json(Vec::<serde_json::Value>::new()));
    }
    let users = koko_db::users::find_by_prefix(&state.db, auth.user_id, &query.q, 20).await?;
    let out: Vec<_> = users
        .into_iter()
        .map(|u| {
            json!({
                "id": u.id,
                "handle": u.handle,
                "display_name": u.display_name,
                "bio": u.bio,
                "avatar_media_id": u.avatar_media_id,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(out))
}

/// `GET /api/users/{handle}`
#[get("/users/{handle}")]
pub async fn get_user(
    _auth: AuthedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    match koko_db::users::get_by_handle(&state.db, &path.into_inner()).await? {
        Some(u) => Ok(HttpResponse::Ok().json(json!({
            "id": u.id,
            "handle": u.handle,
            "display_name": u.display_name,
            "bio": u.bio,
            "avatar_media_id": u.avatar_media_id,
        }))),
        None => Err(ApiError::from(koko_core::KokoError::NotFound(
            "user".into(),
        ))),
    }
}
