//! # koko-web
//!
//! The browser client — Rust compiled to WASM via Leptos, sharing
//! `koko-protocol` with the native client and the server (one envelope
//! implementation, everywhere).
//!
//! Build: `trunk build --release` (or `trunk serve` for development).
//!
//! v0.1 scope: login/register, live chat list, conversation view, sending
//! over the same binary WS protocol. Multi-device keys, calls and the full
//! glass motion system land with the web milestone — see docs/ARCHITECTURE.

use leptos::*;
use serde::Deserialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use koko_protocol::codec::{decode_server, encode_client};
use koko_protocol::{ClientFrame, ClientMsgId, Envelope, MessageKind, ServerFrame};

#[derive(Clone, Debug, Deserialize)]
struct AuthOk {
    token: String,
    user: UserBrief,
}

#[derive(Clone, Debug, Deserialize)]
struct UserBrief {
    id: i64,
    handle: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ChatRow {
    id: i64,
    title: Option<String>,
    members: Vec<String>,
    member_count: i64,
    unread: i64,
}

// ------------------------------------------------------------------- fetch

async fn api(
    method: &str,
    url: &str,
    token: Option<&str>,
    body: Option<String>,
) -> Result<serde_json::Value, String> {
    let opts = web_sys::RequestInit::new();
    opts.set_method(method);
    if let Some(b) = &body {
        opts.set_body(&js_sys::JsString::from(b.as_str()));
    }
    let req = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|_| "could not build request".to_string())?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|_| "headers".to_string())?;
    if let Some(t) = token {
        req.headers()
            .set("Authorization", &format!("Bearer {t}"))
            .map_err(|_| "headers".to_string())?;
    }
    let window = web_sys::window().ok_or("no window")?;
    let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|_| "network error".to_string())?;
    let resp: web_sys::Response = web_sys::Response::from(resp);
    if !resp.ok() {
        return Err(format!("http {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|_| "body".to_string())?)
        .await
        .map_err(|_| "body".to_string())?;
    let text = js_sys::JsString::from(text).as_string().unwrap_or_default();
    serde_json::from_str(&text).map_err(|e| format!("bad json: {e}"))
}

// --------------------------------------------------------------------- app

#[component]
pub fn App() -> impl IntoView {
    // ---- auth state
    let (token, set_token) = create_signal(None::<String>);
    let (me, set_me) = create_signal(None::<UserBrief>);
    let (server, set_server) = create_signal("http://127.0.0.1:8080".to_string());
    let (handle, set_handle) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(None::<String>);

    // ---- chat state
    let (chats, set_chats) = create_signal(Vec::<ChatRow>::new());
    let (envelopes, set_envelopes) = create_signal(Vec::<Envelope>::new());
    let (active_chat, set_active_chat) = create_signal(None::<i64>);
    let (draft, set_draft) = create_signal(String::new());
    let (connected, set_connected) = create_signal(false);
    // The socket is kept in a signal so the send path can reach it.
    let (socket, set_socket) = create_signal(None::<web_sys::WebSocket>);

    // ---- login / register
    let do_auth = move |register: bool| {
        let server = server.get();
        let handle = handle.get();
        let password = password.get();
        spawn_local(async move {
            set_error.set(None);
            let url = format!(
                "{server}/api/{}",
                if register { "register" } else { "login" }
            );
            let body = serde_json::json!({
                "handle": handle,
                "password": password,
                "device_name": "koko-web",
            })
            .to_string();
            let result = api("POST", &url, None, Some(body)).await;
            match result {
                Ok(v) => match serde_json::from_value::<AuthOk>(v) {
                    Ok(auth) => {
                        let ws_url = server.replace("http", "ws") + "/ws";
                        open_socket(
                            &ws_url,
                            &auth.token,
                            set_connected,
                            set_envelopes,
                            set_socket,
                        );
                        set_token.set(Some(auth.token.clone()));
                        set_me.set(Some(auth.user));
                        // Load the chat list.
                        spawn_local(load_chats(server, auth.token, set_chats, set_error));
                    }
                    Err(e) => set_error.set(Some(format!("login: {e}"))),
                },
                Err(e) => set_error.set(Some(e)),
            }
        });
    };
    let sign_in = move |_| do_auth(false);
    let create_account = move |_| do_auth(true);

    // ---- sending
    let send = move |_| {
        let text = draft.get().trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(chat) = active_chat.get() else {
            return;
        };
        let Some(user) = me.get() else { return };
        set_draft.set(String::new());
        let frame = ClientFrame::Send {
            client_msg_id: ClientMsgId::from_bytes(koko_core::token::new_client_msg_id()),
            chat: koko_protocol::ChatId(chat),
            kind: MessageKind::Text,
            body: text.clone().into_bytes(),
            created_at: koko_core::time::unix_now(),
        };
        if let (Ok(bytes), Some(ws)) = (encode_client(&frame), socket.get()) {
            let _ = ws.send_with_u8_array(&bytes);
            // Optimistic local echo (the server envelope will dedupe by seq).
            set_envelopes.update(|envs| {
                envs.push(Envelope {
                    seq: 0,
                    chat: koko_protocol::ChatId(chat),
                    sender: koko_protocol::UserId(user.id),
                    sender_handle: user.handle.clone(),
                    kind: MessageKind::Text,
                    body: text.into_bytes(),
                    created_at: koko_core::time::unix_now(),
                });
            });
        }
    };

    // ---- views
    let login_screen = move || {
        let error_line = move || {
            error
                .get()
                .map(|e| {
                    view! { <div class="error">{e}</div> }
                })
                .unwrap_or_else(|| view! { <div></div> })
        };
        view! {
            <div class="center">
                <div class="glass panel login">
                    <div class="logo"><span class="k">K</span><span class="word">koko</span></div>
                    <div class="tagline">"chat that feels like light"</div>
                    <input
                        class="field"
                        placeholder="server"
                        prop:value=server
                        on:input=move |ev| set_server.set(event_target_value(&ev))
                    />
                    <input
                        class="field"
                        placeholder="username"
                        prop:value=handle
                        on:input=move |ev| set_handle.set(event_target_value(&ev))
                    />
                    <input
                        class="field"
                        type="password"
                        placeholder="password"
                        prop:value=password
                        on:input=move |ev| set_password.set(event_target_value(&ev))
                    />
                    <button class="accent" on:click=sign_in>"Sign in"</button>
                    <button class="ghost" on:click=create_account>"Create account on your server"</button>
                    {error_line}
                </div>
            </div>
        }
    };

    let chat_screen = move || {
        let my_handle = move || me.get().map(|m| m.handle).unwrap_or_default();
        let chat_list = move || {
            chats
                .get()
                .into_iter()
                .map(|c| {
                    let id = c.id;
                    let title = c.title.clone().unwrap_or_else(|| {
                        c.members
                            .iter()
                            .filter(|m| *m != &my_handle())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    });
                    let unread = c.unread;
                    let selected = active_chat.get() == Some(id);
                    view! {
                        <div class="chat-item" class:selected=selected on:click=move |_| set_active_chat.set(Some(id))>
                            <div class="chat-title">{title}</div>
                            <div class="chat-sub">
                                {if unread > 0 { format!("{unread} unread") } else { format!("{} member(s)", c.member_count) }}
                            </div>
                        </div>
                    }
                })
                .collect::<Vec<_>>()
        };
        let conversation = move || {
            let active = active_chat.get();
            match active {
                None => view! {
                    <div class="empty">
                        <div class="big-k">"K"</div>
                        <div>"Select a chat"</div>
                        <div class="muted">"Your messages stay on your server"</div>
                    </div>
                },
                Some(chat) => {
                    let my_id = me.get().map(|m| m.id).unwrap_or(-1);
                    let messages = envelopes
                        .get()
                        .into_iter()
                        .filter(|e| e.chat.0 == chat)
                        .map(|e| {
                            let mine = e.sender.0 == my_id;
                            let text = String::from_utf8_lossy(&e.body).to_string();
                            view! {
                                <div class="row-msg">
                                    {if mine {
                                        view! { <div class="spacer"></div> }
                                    } else {
                                        view! { <div></div> }
                                    }}
                                    <div class=if mine { "bubble mine" } else { "bubble" }>
                                        {text}
                                    </div>
                                    {if mine {
                                        view! { <div></div> }
                                    } else {
                                        view! { <div class="spacer"></div> }
                                    }}
                                </div>
                            }
                        })
                        .collect::<Vec<_>>();
                    let header = chats
                        .get()
                        .into_iter()
                        .find(|c| c.id == chat)
                        .map(|c| c.title.clone().unwrap_or_else(|| c.members.join(", ")))
                        .unwrap_or_else(|| "chat".into());
                    view! {
                        <div class="conversation">
                            <div class="conv-header">{header}</div>
                            <div class="messages">{messages}</div>
                            <div class="composer">
                                <input
                                    class="field"
                                    placeholder="Message"
                                    prop:value=draft
                                    on:input=move |ev| set_draft.set(event_target_value(&ev))
                                />
                                <button class="accent send" on:click=send>"send"</button>
                            </div>
                        </div>
                    }
                }
            }
        };
        let status = move || {
            if connected.get() {
                "connected".to_string()
            } else {
                "reconnecting…".to_string()
            }
        };
        view! {
            <div class="app">
                <div class="glass panel sidebar">
                    <div class="logo"><span class="k">K</span><span class="word">koko</span></div>
                    <div class="status">{status}</div>
                    <div class="chat-list">{chat_list()}</div>
                    <button
                        class="ghost"
                        on:click=move |_| {
                            set_token.set(None);
                            set_me.set(None);
                            set_chats.set(Vec::new());
                            set_envelopes.set(Vec::new());
                            set_active_chat.set(None);
                        }
                    >
                        "sign out"
                    </button>
                </div>
                <div class="glass panel main">{conversation()}</div>
            </div>
        }
    };

    view! {
        <div class="root">
            {move || if token.get().is_some() { chat_screen() } else { login_screen() }}
        </div>
    }
}

/// Open the WS connection; incoming envelopes land in the signal.
fn open_socket(
    ws_url: &str,
    token: &str,
    set_connected: WriteSignal<bool>,
    set_envelopes: WriteSignal<Vec<Envelope>>,
    set_socket: WriteSignal<Option<web_sys::WebSocket>>,
) {
    let Ok(ws) = web_sys::WebSocket::new(ws_url) else {
        return;
    };
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // Say hello as soon as the socket opens.
    let hello = ClientFrame::Hello {
        token: token.to_owned(),
        resume_from: None,
    };
    let hello_ws = ws.clone();
    let on_open = Closure::<dyn FnMut()>::new(move || {
        if let Ok(bytes) = encode_client(&hello) {
            let _ = hello_ws.send_with_u8_array(&bytes);
        }
    });
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let on_close = Closure::<dyn FnMut()>::new(move || set_connected.set(false));
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    on_close.forget();

    let on_message =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            let data = e.data();
            if data.has_type::<js_sys::ArrayBuffer>() {
                let bytes = js_sys::Uint8Array::new(&data).to_vec();
                if let Ok(ServerFrame::Envelope { env }) = decode_server(&bytes) {
                    set_envelopes.update(|envs| envs.push(env));
                }
            }
        });
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    set_socket.set(Some(ws));
    set_connected.set(true);
}

/// Load the chat list into the signal.
async fn load_chats(
    server: String,
    token: String,
    set_chats: WriteSignal<Vec<ChatRow>>,
    set_error: WriteSignal<Option<String>>,
) {
    match api("GET", &format!("{server}/api/chats"), Some(&token), None).await {
        Ok(v) => match serde_json::from_value::<Vec<ChatRow>>(v) {
            Ok(rows) => set_chats.set(rows),
            Err(e) => set_error.set(Some(format!("chats: {e}"))),
        },
        Err(e) => set_error.set(Some(e)),
    }
}

pub fn main() {
    mount_to_body(App);
}
