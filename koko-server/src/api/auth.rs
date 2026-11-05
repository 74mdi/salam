//! Registration, login, logout, identity.
//!
//! Bearer-token auth: `Authorization: Bearer <token>`. Tokens are 43-char
//! base64url strings; only their SHA-256 digest is stored.

use std::future::Future;
use std::pin::Pin;

use actix_web::dev::Payload;
use actix_web::http::header::{HeaderName, HeaderValue, AUTHORIZATION};
use actix_web::{get, post, put, web, FromRequest, HttpRequest, HttpResponse};
use futures_util::future::LocalBoxFuture;
use koko_core::time::unix_now;
use koko_core::{validate_display_name, validate_handle, KokoError};
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::limits::policy;
use crate::state::{AppState, AuthedUser};

/// Extract a bearer token from the Authorization header.
fn bearer_token(req: &HttpRequest) -> Result<String, KokoError> {
    let value = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(KokoError::BadCredentials)?;
    value
        .strip_prefix("Bearer ")
        .map(str::to_owned)
        .ok_or(KokoError::BadCredentials)
}

/// Extracts the authenticated user for any handler that lists it first.
impl FromRequest for AuthedUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let fut: Pin<Box<dyn Future<Output = Result<AuthedUser, ApiError>> + 'static>> = {
            let req = req.clone();
            Box::pin(async move {
                let state = req
                    .app_data::<web::Data<AppState>>()
                    .ok_or(ApiError::from(KokoError::Internal))?;
                let token = bearer_token(&req)?;
                state.authenticate(&token).await
            })
        };
        fut
    }
}

/// Response headers we attach to auth responses (no caching of tokens).
fn no_store(mut res: HttpResponse) -> HttpResponse {
    res.headers_mut().insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-store"),
    );
    res
}

// ---------------------------------------------------------------- register

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Desired handle.
    pub handle: String,
    /// Optional display name (defaults to the handle).
    #[serde(default)]
    pub display_name: Option<String>,
    /// Password (min 8 chars).
    pub password: String,
}

/// `POST /api/register`
#[post("/register")]
pub async fn register(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
    let ip = req
        .peer_addr()
        .map_or_else(|| "unknown".into(), |a| a.ip().to_string());
    if !state.limiter.allow(
        &format!("register:{ip}"),
        policy::REGISTER_MAX,
        policy::REGISTER_WINDOW,
    ) {
        return Err(ApiError::from(KokoError::RateLimited));
    }
    if !state.cfg.open_registration {
        return Err(ApiError::from(KokoError::RegistrationClosed));
    }

    validate_handle(&body.handle)?;
    let display_name = body
        .display_name
        .clone()
        .unwrap_or_else(|| body.handle.clone());
    validate_display_name(&display_name)?;

    // Argon2 is CPU/memory work — keep it off the async workers.
    let pw = body.password.clone();
    let hash = web::block(move || koko_core::password::hash_password(&pw))
        .await
        .map_err(|_| ApiError::from(KokoError::Internal))?
        .map_err(|e| ApiError::from(KokoError::from(e)))?;

    let user =
        koko_db::users::create_user(&state.db, &body.handle, &display_name, &hash, unix_now())
            .await?;
    log::info!("register: {} → user {}", user.handle, user.id);

    Ok(no_store(HttpResponse::Created().json(json!({
        "id": user.id,
        "handle": user.handle,
        "display_name": user.display_name,
                "bio": user.bio,
                "avatar_media_id": user.avatar_media_id,
    }))))
}

// ------------------------------------------------------------------- login

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Handle.
    pub handle: String,
    /// Password.
    pub password: String,
    /// Device name (stored, shown in "active devices").
    #[serde(default)]
    pub device_name: Option<String>,
}

/// `POST /api/login`
#[post("/login")]
pub async fn login(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let ip = req
        .peer_addr()
        .map_or_else(|| "unknown".into(), |a| a.ip().to_string());
    let ok_ip = state.limiter.allow(
        &format!("login-ip:{ip}"),
        policy::LOGIN_IP_MAX,
        policy::LOGIN_WINDOW,
    );
    let ok_handle = state.limiter.allow(
        &format!("login-handle:{}", body.handle),
        policy::LOGIN_HANDLE_MAX,
        policy::LOGIN_WINDOW,
    );
    if !ok_ip || !ok_handle {
        return Err(ApiError::from(KokoError::RateLimited));
    }

    let now = unix_now();
    let user = koko_db::users::get_by_handle_with_hash(&state.db, &body.handle).await?;

    match user {
        None => {
            // Equalize timing with the "wrong password" path (anti-enumeration).
            let dummy = state.dummy_hash.clone();
            let pw = body.password.clone();
            let _ = web::block(move || koko_core::password::verify_password(&pw, &dummy)).await;
            Err(ApiError::from(KokoError::BadCredentials))
        }
        Some(user) => {
            let pw = body.password.clone();
            let stored = user.password_hash.clone();
            let ok = web::block(move || koko_core::password::verify_password(&pw, &stored))
                .await
                .map_err(|_| ApiError::from(KokoError::Internal))?
                .unwrap_or(false);
            if !ok {
                return Err(ApiError::from(KokoError::BadCredentials));
            }

            let device_name = body
                .device_name
                .clone()
                .unwrap_or_else(|| "unnamed device".into());
            let device_id =
                koko_db::devices::upsert_device(&state.db, user.id, &device_name, now).await?;

            let token = koko_core::token::new_token();
            koko_db::sessions::create_session(
                &state.db,
                &koko_core::token::token_hash(&token),
                user.id,
                device_id,
                now,
                state.cfg.token_ttl_secs,
            )
            .await?;
            log::info!("login: {} (device {device_name})", user.handle);

            Ok(no_store(HttpResponse::Ok().json(json!({
                "token": token,
                "token_type": "Bearer",
                "expires_at": now + state.cfg.token_ttl_secs,
                "user": {
                    "id": user.id,
                    "handle": user.handle,
                    "display_name": user.display_name,
                    "bio": user.bio,
                    "avatar_media_id": user.avatar_media_id,
                },
                "device_id": device_id,
                "device_name": device_name,
            }))))
        }
    }
}

// ------------------------------------------------------------------ logout

/// `POST /api/logout` — revokes the presented token's session.
#[post("/logout")]
pub async fn logout(
    auth: AuthedUser,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let token = bearer_token(&req)?;
    let _ =
        koko_db::sessions::delete_session(&state.db, &koko_core::token::token_hash(&token)).await?;
    log::info!("logout: {}", auth.handle);
    Ok(HttpResponse::NoContent().finish())
}

/// `POST /api/logout-all` — revokes every session of the user.
#[post("/logout-all")]
pub async fn logout_all(
    auth: AuthedUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let n = koko_db::sessions::revoke_all_for_user(&state.db, auth.user_id).await?;
    log::info!("logout-all: {} ({} sessions)", auth.handle, n);
    Ok(HttpResponse::NoContent().finish())
}

// ---------------------------------------------------------------------- me

/// `GET /api/me`
#[get("/me")]
pub async fn me(auth: AuthedUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let devices = koko_db::devices::list_devices(&state.db, auth.user_id).await?;
    let user = koko_db::users::get_by_id(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| KokoError::NotFound("user".into()))?;
    Ok(HttpResponse::Ok().json(json!({
        "id": user.id,
        "handle": user.handle,
        "display_name": user.display_name,
        "bio": user.bio,
        "avatar_media_id": user.avatar_media_id,
        "devices": devices
            .into_iter()
            .map(|d| json!({ "id": d.id, "name": d.name, "last_seen": d.last_seen }))
            .collect::<Vec<_>>(),
    })))
}

/// Profile fields a user can update without changing their login handle.
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    /// Name shown to other people.
    pub display_name: String,
    /// Public biography.
    #[serde(default)]
    pub bio: String,
    /// Previously uploaded profile image.
    #[serde(default)]
    pub avatar_media_id: Option<String>,
}

/// `PUT /api/me` — update the current user's display name.
#[put("/me")]
pub async fn update_profile(
    auth: AuthedUser,
    state: web::Data<AppState>,
    body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, ApiError> {
    validate_display_name(&body.display_name)?;
    if body.bio.chars().count() > 160 || body.bio.chars().any(char::is_control) {
        return Err(ApiError::from(KokoError::BadRequest(
            "bio must be at most 160 characters".into(),
        )));
    }
    let user = koko_db::users::update_profile(
        &state.db,
        auth.user_id,
        &body.display_name,
        &body.bio,
        body.avatar_media_id.as_deref(),
    )
    .await?;
    Ok(no_store(HttpResponse::Ok().json(json!({
        "id": user.id,
        "handle": user.handle,
        "display_name": user.display_name,
        "bio": user.bio,
        "avatar_media_id": user.avatar_media_id,
    }))))
}
