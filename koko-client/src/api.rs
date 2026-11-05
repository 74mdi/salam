//! REST client for the koko-server HTTP API.

use serde::Deserialize;

use koko_protocol::Envelope;

/// Make a server address safe to append API and WebSocket paths to.
pub fn normalize_base_url(base: &str) -> String {
    base.trim().trim_end_matches('/').to_owned()
}

/// Authenticated identity returned by login.
#[derive(Debug, Clone, Deserialize)]
pub struct AuthOk {
    /// Opaque session token.
    pub token: String,
    /// The user.
    pub user: UserBrief,
}

/// Public user shape.
#[derive(Debug, Clone, Deserialize)]
pub struct UserBrief {
    /// Numeric id.
    pub id: i64,
    /// Handle.
    pub handle: String,
    /// User-selected display name.
    pub display_name: String,
    /// Public biography.
    #[serde(default)]
    pub bio: String,
    /// Profile image media id.
    #[serde(default)]
    pub avatar_media_id: Option<String>,
}

/// One chat in the list.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatRow {
    /// Chat id.
    pub id: i64,
    /// Group title (None for direct chats).
    pub title: Option<String>,
    /// Member handles.
    pub members: Vec<String>,
    /// Member count.
    pub member_count: i64,
    /// Unix seconds of the newest message.
    pub last_activity: i64,
    /// Unread envelope count.
    pub unread: i64,
}

/// One bootstrapped message from `GET /api/chats/{id}/messages`.
#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapMsg {
    /// Recipient inbox seq.
    pub seq: u64,
    /// Sender.
    pub sender_id: i64,
    /// Sender handle.
    pub sender_handle: String,
    /// Kind code (`MessageKind` as i64).
    pub kind: i64,
    /// Opaque body.
    pub body: Vec<u8>,
    /// Unix seconds.
    pub created_at: i64,
}

async fn req(
    method: reqwest::Method,
    url: String,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::new();
    let mut r = client.request(method, &url);
    if let Some(t) = token {
        r = r.bearer_auth(t);
    }
    if let Some(b) = body {
        r = r.json(&b);
    }
    r.send().await.map_err(|e| format!("network: {e}"))
}

async fn json<T: for<'de> Deserialize<'de>>(
    method: reqwest::Method,
    url: String,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    let resp = req(method, url, token, body).await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("http {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("bad json: {e}"))
}

/// `POST /api/register`
pub async fn register(base: &str, handle: &str, password: &str) -> Result<(), String> {
    let url = format!("{base}/api/register");
    req(
        reqwest::Method::POST,
        url,
        None,
        Some(serde_json::json!({ "handle": handle, "password": password })),
    )
    .await?
    .error_for_status()
    .map(|_| ())
    .map_err(|e| format!("register: {e}"))
}

/// `POST /api/login`
pub async fn login(
    base: &str,
    handle: &str,
    password: &str,
    device: &str,
) -> Result<AuthOk, String> {
    json(
        reqwest::Method::POST,
        format!("{base}/api/login"),
        None,
        Some(serde_json::json!({
            "handle": handle,
            "password": password,
            "device_name": device,
        })),
    )
    .await
}

/// Register then immediately log in (the "create account" button).
pub async fn register_then_login(
    base: &str,
    handle: &str,
    password: &str,
    device: &str,
) -> Result<AuthOk, String> {
    register(base, handle, password).await?;
    login(base, handle, password, device).await
}

/// `GET /api/chats`
pub async fn chats(base: &str, token: &str) -> Result<Vec<ChatRow>, String> {
    json(
        reqwest::Method::GET,
        format!("{base}/api/chats"),
        Some(token),
        None,
    )
    .await
}

/// `GET /api/messages?since=` — global inbox (REST sync fallback).
pub async fn inbox(base: &str, token: &str, since: u64) -> Result<Vec<Envelope>, String> {
    #[derive(Deserialize)]
    struct Inbox {
        messages: Vec<Envelope>,
    }
    let inbox: Inbox = json(
        reqwest::Method::GET,
        format!("{base}/api/messages?since={since}&limit=500"),
        Some(token),
        None,
    )
    .await?;
    Ok(inbox.messages)
}

/// `GET /api/chats/{id}/messages?after=` — chat bootstrap.
pub async fn chat_messages(
    base: &str,
    token: &str,
    chat: i64,
    after: i64,
) -> Result<Vec<BootstrapMsg>, String> {
    json(
        reqwest::Method::GET,
        format!("{base}/api/chats/{chat}/messages?after={after}&limit=200"),
        Some(token),
        None,
    )
    .await
}

/// `POST /api/chats` — open (or find) a direct chat with `peer`.
pub async fn direct_chat(base: &str, token: &str, peer: &str) -> Result<i64, String> {
    #[derive(Deserialize)]
    struct Created {
        chat_id: i64,
    }
    let created: Created = json(
        reqwest::Method::POST,
        format!("{base}/api/chats"),
        Some(token),
        Some(serde_json::json!({ "kind": "direct", "peer": peer })),
    )
    .await?;
    Ok(created.chat_id)
}

/// `POST /api/chats/self` — open the current user's notes chat.
pub async fn note_to_self(base: &str, token: &str) -> Result<i64, String> {
    #[derive(Deserialize)]
    struct Created {
        chat_id: i64,
    }
    let created: Created = json(
        reqwest::Method::POST,
        format!("{base}/api/chats/self"),
        Some(token),
        None,
    )
    .await?;
    Ok(created.chat_id)
}

/// `PUT /api/me` — update the current display name.
pub async fn update_profile(
    base: &str,
    token: &str,
    display_name: &str,
    bio: &str,
    avatar_media_id: Option<&str>,
) -> Result<UserBrief, String> {
    json(
        reqwest::Method::PUT,
        format!("{base}/api/me"),
        Some(token),
        Some(serde_json::json!({
            "display_name": display_name,
            "bio": bio,
            "avatar_media_id": avatar_media_id,
        })),
    )
    .await
}

/// Upload raw media bytes and return the server-side media reference.
pub async fn upload_media(
    base: &str,
    token: &str,
    filename: &str,
    mime: &str,
    bytes: Vec<u8>,
) -> Result<MediaUpload, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base}/api/media"))
        .query(&[("name", filename)])
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, mime)
        .body(bytes)
        .send()
        .await
        .map_err(|e| format!("upload: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("upload http {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|e| format!("upload response: {e}"))
}

/// Download authenticated media bytes, used for profile images.
pub async fn download_media(base: &str, token: &str, media_id: &str) -> Result<Vec<u8>, String> {
    let response = reqwest::Client::new()
        .get(format!("{base}/api/media/{media_id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("media: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("media http {status}"));
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| format!("media body: {e}"))
}

/// Server-side media metadata returned after an upload.
#[derive(Debug, Clone, Deserialize)]
pub struct MediaUpload {
    pub id: String,
    pub filename: String,
    pub mime: String,
    pub size: i64,
}

#[cfg(test)]
mod tests {
    use super::normalize_base_url;

    #[test]
    fn normalizes_pasted_server_addresses() {
        assert_eq!(
            normalize_base_url(" https://chat.example/ "),
            "https://chat.example"
        );
        assert_eq!(
            normalize_base_url("http://127.0.0.1:8080///"),
            "http://127.0.0.1:8080"
        );
    }
}
