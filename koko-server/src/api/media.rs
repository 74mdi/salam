//! Media: upload (raw body), download, delete. Quota + LRU eviction.
//!
//! * Single-file cap: `--max-file-size` (default 25 MiB).
//! * Per-user quota: `--media-quota` (default 4 GiB), enforced per upload.
//! * When a new upload would exceed the quota, the server evicts that
//!   user's least-recently-downloaded files until it fits — automatic LRU
//!   cleanup, per the spec.

use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use koko_core::time::unix_now_millis;
use koko_core::KokoError;
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::limits::policy;
use crate::state::{AppState, AuthedUser};

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    /// Original filename (display only; stored media ids are random hex).
    #[serde(default)]
    pub name: Option<String>,
}

fn random_media_id() -> String {
    use std::fmt::Write;
    let mut raw = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let mut out = String::with_capacity(32);
    for b in raw {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// `POST /api/media?name=photo.jpg` — body is the raw file bytes.
#[post("/media")]
pub async fn upload(
    auth: AuthedUser,
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<UploadQuery>,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    if !state.limiter.allow(
        &format!("media:{}", auth.user_id),
        policy::MEDIA_MAX,
        policy::MEDIA_WINDOW,
    ) {
        return Err(ApiError::from(KokoError::RateLimited));
    }

    let incoming = body.len() as i64;
    if incoming == 0 {
        return Err(ApiError::from(KokoError::BadRequest("empty upload".into())));
    }
    if incoming > state.cfg.max_file_size {
        return Err(ApiError::from(KokoError::PayloadTooLarge(format!(
            "file exceeds {} bytes",
            state.cfg.max_file_size
        ))));
    }
    if incoming > state.cfg.media_quota {
        return Err(ApiError::from(KokoError::QuotaExceeded));
    }

    // Quota enforcement with LRU eviction — all before touching the disk.
    let usage = koko_db::media::usage_bytes(&state.db, auth.user_id).await?;
    if usage + incoming > state.cfg.media_quota {
        let need = usage + incoming - state.cfg.media_quota;
        let evicted = koko_db::media::evict_lru(&state.db, auth.user_id, need).await?;
        for row in &evicted {
            let path = state.cfg.media_dir.join(&row.id);
            let _ = tokio::fs::remove_file(&path).await;
        }
        if !evicted.is_empty() {
            log::info!(
                "media: evicted {} file(s) from {} to fit {} bytes",
                evicted.len(),
                auth.handle,
                incoming
            );
        }
        let usage_after = koko_db::media::usage_bytes(&state.db, auth.user_id).await?;
        if usage_after + incoming > state.cfg.media_quota {
            return Err(ApiError::from(KokoError::QuotaExceeded));
        }
    }

    // Write the file (tmp + rename so nobody reads a half-written blob).
    let id = random_media_id();
    tokio::fs::create_dir_all(&state.cfg.media_dir).await?;
    let path = state.cfg.media_dir.join(&id);
    let tmp = state.cfg.media_dir.join(format!("{id}.part"));
    if let Err(e) = tokio::fs::write(&tmp, &body).await {
        return Err(ApiError::from(e));
    }
    if let Err(e) = tokio::fs::rename(&tmp, &path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(ApiError::from(e));
    }

    let mime = req
        .headers()
        .get(actix_web::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_owned();
    let filename = query.name.clone().unwrap_or_else(|| id.clone());
    let now = unix_now_millis();
    if let Err(e) = koko_db::media::insert_media(
        &state.db,
        &id,
        auth.user_id,
        &filename,
        &mime,
        incoming,
        now,
    )
    .await
    {
        let _ = tokio::fs::remove_file(&path).await; // never leak orphaned files
        return Err(e.into());
    }
    log::info!(
        "media: {} uploaded {} bytes ({})",
        auth.handle,
        incoming,
        mime
    );

    Ok(HttpResponse::Created().json(json!({
        "id": id,
        "url": format!("/api/media/{id}"),
        "filename": filename,
        "mime": mime,
        "size": incoming,
    })))
}

/// `GET /api/media/{id}` — authenticated download (LRU-touched).
#[get("/media/{id}")]
pub async fn download(
    _auth: AuthedUser,
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let row = koko_db::media::get_media(&state.db, &id)
        .await?
        .ok_or_else(|| KokoError::NotFound("media".into()))?;

    // LRU touch (one cheap UPDATE — awaited so quota behavior is deterministic).
    let _ = koko_db::media::touch_media(&state.db, &row.id, unix_now_millis()).await;

    let mime: mime::Mime = row.mime.parse().unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let file = actix_files::NamedFile::open(state.cfg.media_dir.join(&row.id))
        .map_err(|_| KokoError::NotFound("media".into()))?
        .set_content_type(mime)
        .disable_content_disposition();
    Ok(file.into_response(&req))
}

/// `DELETE /api/media/{id}` — owner only.
#[delete("/media/{id}")]
pub async fn remove(
    auth: AuthedUser,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let row = koko_db::media::get_media(&state.db, &id)
        .await?
        .ok_or_else(|| KokoError::NotFound("media".into()))?;
    if row.owner_id != auth.user_id {
        return Err(ApiError::from(KokoError::Forbidden(
            "not the media owner".into(),
        )));
    }
    koko_db::media::delete_media(&state.db, &id).await?;
    let _ = tokio::fs::remove_file(state.cfg.media_dir.join(&id)).await;
    log::info!("media: {} deleted {id}", auth.handle);
    Ok(HttpResponse::NoContent().finish())
}
