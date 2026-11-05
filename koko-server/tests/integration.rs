//! End-to-end integration tests: real HTTP server + real WebSocket clients
//! (tokio-tungstenite) against a real SQLite file.

use actix_test::start as start_server;
use actix_web::{web, App};
use futures_util::{SinkExt, StreamExt};
use koko_protocol::codec::{decode_server, encode_client};
use koko_protocol::{ClientFrame, ClientMsgId, MessageKind, ServerFrame};
use koko_server::config::Config;
use koko_server::hub::Hub;
use koko_server::state::AppState;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

struct TestServer {
    srv: actix_test::TestServer,
    _dir: tempfile::TempDir,
}

async fn start(mutate: impl FnOnce(&mut Config)) -> TestServer {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = Config::for_test(dir.path());
    mutate(&mut cfg);
    let pool = koko_db::connect(&cfg.db_path).await.unwrap();
    let state = web::Data::new(AppState::new(pool, Hub::new(), std::sync::Arc::new(cfg)));
    let srv = start_server(move || {
        App::new()
            .app_data(state.clone())
            .app_data(web::PayloadConfig::new(30 * 1024 * 1024))
            .configure(koko_server::configure_routes)
    });
    TestServer { srv, _dir: dir }
}

impl TestServer {
    fn http(&self) -> String {
        format!("http://{}", self.srv.addr())
    }
    fn ws(&self) -> String {
        format!("ws://{}/ws", self.srv.addr())
    }
}

async fn register(srv: &TestServer, handle: &str) -> Value {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/register", srv.http()))
        .json(&json!({ "handle": handle, "password": "hunter2hunter2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "register {handle} must succeed");
    resp.json().await.unwrap()
}

async fn login(srv: &TestServer, handle: &str) -> Value {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/login", srv.http()))
        .json(&json!({
            "handle": handle,
            "password": "hunter2hunter2",
            "device_name": "test-suite"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "login {handle} must succeed");
    resp.json().await.unwrap()
}

async fn bearer(
    srv: &TestServer,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> reqwest::Response {
    let client = reqwest::Client::new();
    let mut req = client
        .request(
            reqwest::Method::from_bytes(method.as_bytes()).unwrap(),
            format!("{}{}", srv.http(), path),
        )
        .bearer_auth(token);
    if let Some(b) = body {
        req = req.json(&b);
    }
    req.send().await.unwrap()
}

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn ws_connect_and_hello(srv: &TestServer, token: &str) -> (Ws, ServerFrame) {
    let (mut ws, _) = connect_async(srv.ws()).await.unwrap();
    ws.send(WsMessage::Binary(
        encode_client(&ClientFrame::Hello {
            token: token.into(),
            resume_from: None,
        })
        .unwrap(),
    ))
    .await
    .unwrap();
    let hello = read_frame(&mut ws).await;
    match &hello {
        ServerFrame::Hello { .. } => {}
        other => panic!("expected Hello, got {other:?}"),
    }
    (ws, hello)
}

async fn read_frame(ws: &mut Ws) -> ServerFrame {
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
            .await
            .expect("frame within 10s")
            .expect("stream open")
            .expect("ws ok");
        match msg {
            WsMessage::Binary(bytes) => return decode_server(&bytes).expect("valid frame"),
            WsMessage::Ping(p) => {
                ws.send(WsMessage::Pong(p)).await.unwrap();
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }
}

async fn send_frame(ws: &mut Ws, frame: &ClientFrame) {
    ws.send(WsMessage::Binary(encode_client(frame).unwrap()))
        .await
        .unwrap();
}

// --------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::too_many_lines)] // one narrative test, read top to bottom
async fn full_messaging_flow() {
    let srv = start(|_| {}).await;
    register(&srv, "alice").await;
    register(&srv, "bob").await;
    let ta = login(&srv, "alice").await;
    let tb = login(&srv, "bob").await;
    let token_a = ta["token"].as_str().unwrap().to_owned();
    let token_b = tb["token"].as_str().unwrap().to_owned();

    // -- direct chat via REST
    let resp = bearer(
        &srv,
        "POST",
        "/api/chats",
        &token_a,
        Some(json!({
            "kind": "direct", "peer": "bob"
        })),
    )
    .await;
    assert_eq!(resp.status(), 201);
    let chat_id = resp.json::<Value>().await.unwrap()["chat_id"]
        .as_i64()
        .unwrap();

    // -- both sockets online
    let (mut ws_a, hello_a) = ws_connect_and_hello(&srv, &token_a).await;
    let ServerFrame::Hello {
        handle, next_seq, ..
    } = hello_a
    else {
        panic!()
    };
    assert_eq!(handle, "alice");
    assert_eq!(next_seq, 0);

    let (mut ws_b, _) = ws_connect_and_hello(&srv, &token_b).await;

    // -- alice sends
    let cmid = ClientMsgId::from_bytes([0x11; 16]);
    send_frame(
        &mut ws_a,
        &ClientFrame::Send {
            client_msg_id: cmid,
            chat: koko_protocol::ChatId(chat_id),
            kind: MessageKind::Text,
            body: b"hello from the test".to_vec(),
            created_at: 1_700_000_500,
        },
    )
    .await;

    // -- bob receives the envelope
    let mut got_envelope = None;
    let mut got_sent = false;
    // alice sees her own envelope + the Sent ack; bob sees the envelope.
    for _ in 0..3 {
        tokio::select! {
            f = read_frame(&mut ws_b) => {
                if let ServerFrame::Envelope { env } = f {
                    got_envelope = Some(env);
                    break;
                }
            }
        }
    }
    let env = got_envelope.expect("bob must receive the envelope");
    assert_eq!(env.body, b"hello from the test");
    assert_eq!(env.sender_handle, "alice");
    assert_eq!(env.seq, 1);

    // -- alice gets her own copy + Sent
    for _ in 0..2 {
        if let ServerFrame::Sent { client_msg_id, seq } = read_frame(&mut ws_a).await {
            assert_eq!(client_msg_id, cmid);
            assert_eq!(seq, 1);
            got_sent = true;
            break;
        }
    }
    assert!(got_sent, "alice must receive Sent");

    // -- REST sync fallback for bob
    let resp = bearer(&srv, "GET", "/api/messages?since=0", &token_b, None).await;
    assert_eq!(resp.status(), 200);
    let inbox = resp.json::<Value>().await.unwrap();
    assert_eq!(inbox["messages"].as_array().unwrap().len(), 1);
    assert_eq!(inbox["next_cursor"].as_u64().unwrap(), 1);

    // -- bob acks the chat read → unread drops
    send_frame(
        &mut ws_b,
        &ClientFrame::Ack {
            chat: koko_protocol::ChatId(chat_id),
            up_to_seq: 1,
        },
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let resp = bearer(&srv, "GET", "/api/chats", &token_b, None).await;
    let chats = resp.json::<Vec<Value>>().await.unwrap();
    assert_eq!(chats[0]["unread"].as_i64().unwrap(), 0);

    // -- typing indicator relays to alice (skipping her own echo frames)
    send_frame(
        &mut ws_b,
        &ClientFrame::Typing {
            chat: koko_protocol::ChatId(chat_id),
            on: true,
        },
    )
    .await;
    let mut got_typing = None;
    for _ in 0..3 {
        if let ServerFrame::Typing {
            from_handle, on, ..
        } = read_frame(&mut ws_a).await
        {
            got_typing = Some((from_handle, on));
            break;
        }
    }
    let (from_handle, on) = got_typing.expect("alice must receive Typing");
    assert_eq!(from_handle, "bob");
    assert!(on);

    // -- duplicate send is idempotent on the wire
    send_frame(
        &mut ws_a,
        &ClientFrame::Send {
            client_msg_id: cmid,
            chat: koko_protocol::ChatId(chat_id),
            kind: MessageKind::Text,
            body: b"hello from the test".to_vec(),
            created_at: 1_700_000_500,
        },
    )
    .await;
    let mut got_sent = None;
    for _ in 0..3 {
        if let ServerFrame::Sent { client_msg_id, seq } = read_frame(&mut ws_a).await {
            got_sent = Some((client_msg_id, seq));
            break;
        }
    }
    let (acked_id, acked_seq) = got_sent.expect("duplicate send must be acked");
    assert_eq!(acked_id, cmid);
    assert_eq!(acked_seq, 1, "duplicate returns the original seq");

    // -- /healthz is public
    let resp = reqwest::Client::new()
        .get(format!("{}/healthz", srv.http()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let h = resp.json::<Value>().await.unwrap();
    assert!(h["ok"].as_bool().unwrap());
    assert_eq!(h["users"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn bad_token_gets_error_frame_and_close() {
    let srv = start(|_| {}).await;
    register(&srv, "alice").await;
    let (mut ws, ()) = connect_raw(&srv).await;
    ws.send(WsMessage::Binary(
        encode_client(&ClientFrame::Hello {
            token: "forged-token".into(),
            resume_from: None,
        })
        .unwrap(),
    ))
    .await
    .unwrap();
    match read_frame(&mut ws).await {
        ServerFrame::Error { code, .. } => assert_eq!(code, koko_protocol::ErrorCode::Unauthorized),
        other => panic!("expected Error, got {other:?}"),
    }
}

async fn connect_raw(srv: &TestServer) -> (Ws, ()) {
    let (ws, _) = connect_async(srv.ws()).await.unwrap();
    (ws, ())
}

#[tokio::test]
async fn auth_edge_cases() {
    let srv = start(|_| {}).await;
    let client = reqwest::Client::new();

    // duplicate handle → 409
    register(&srv, "alice").await;
    let resp = client
        .post(format!("{}/api/register", srv.http()))
        .json(&json!({ "handle": "alice", "password": "hunter2hunter2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);

    // invalid handle → 400
    let resp = client
        .post(format!("{}/api/register", srv.http()))
        .json(&json!({ "handle": "Not Valid!", "password": "hunter2hunter2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // wrong password → 401
    let resp = client
        .post(format!("{}/api/login", srv.http()))
        .json(&json!({ "handle": "alice", "password": "wrong-wrong" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // unknown user → 401 (same as wrong password)
    let resp = client
        .post(format!("{}/api/login", srv.http()))
        .json(&json!({ "handle": "ghost", "password": "whatever123" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // unauthenticated /api/me → 401
    let resp = client
        .get(format!("{}/api/me", srv.http()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn register_rate_limit() {
    // Fresh server: exactly 5 registrations per hour per IP, then 429.
    let srv = start(|_| {}).await;
    let client = reqwest::Client::new();
    for i in 0..5 {
        let resp = client
            .post(format!("{}/api/register", srv.http()))
            .json(&json!({ "handle": format!("user{i}"), "password": "hunter2hunter2" }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "attempt {i} within budget");
    }
    let resp = client
        .post(format!("{}/api/register", srv.http()))
        .json(&json!({ "handle": "over", "password": "hunter2hunter2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 429);
}

#[tokio::test]
async fn registration_can_be_closed() {
    let srv = start(|cfg| cfg.open_registration = false).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/register", srv.http()))
        .json(&json!({ "handle": "alice", "password": "hunter2hunter2" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn connection_limit_enforced() {
    let srv = start(|cfg| cfg.max_connections = 1).await;
    register(&srv, "alice").await;
    let token = login(&srv, "alice").await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let (_ws1, _hello) = ws_connect_and_hello(&srv, &token).await;
    // Second socket is refused before the upgrade completes.
    let refused = connect_async(srv.ws()).await;
    assert!(refused.is_err(), "second connection must be refused");
}

#[tokio::test]
async fn media_upload_quota_and_lru() {
    // Tiny quota (2500 B), tiny per-file cap (2000 B).
    let srv = start(|cfg| {
        cfg.media_quota = 2500;
        cfg.max_file_size = 2000;
    })
    .await;
    register(&srv, "alice").await;
    let token = login(&srv, "alice").await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let client = reqwest::Client::new();
    let upload = |name: &str, bytes: Vec<u8>| {
        let client = client.clone();
        let token = token.clone();
        let url = format!("{}/api/media?name={name}", srv.http());
        async move {
            client
                .post(url)
                .bearer_auth(&token)
                .header("Content-Type", "application/octet-stream")
                .body(bytes)
                .send()
                .await
                .unwrap()
        }
    };

    // Oversized → 413.
    let resp = upload("big.bin", vec![7u8; 2500]).await;
    assert_eq!(resp.status(), 413);

    // Two files fill the quota.
    let r1 = upload("one.bin", vec![1u8; 1000]).await;
    assert_eq!(r1.status(), 201);
    let id1 = r1.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let r2 = upload("two.bin", vec![2u8; 1000]).await;
    assert_eq!(r2.status(), 201);
    let id2 = r2.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Download id1 → bumps its LRU clock above id2.
    let resp = client
        .get(format!("{}/api/media/{id1}", srv.http()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Vec<u8> = resp.bytes().await.unwrap().to_vec();
    assert_eq!(body, vec![1u8; 1000]);
    // (LRU touch is awaited server-side — no sleep needed)

    // Third upload evicts id2 (least recently used), keeps id1.
    let r3 = upload("three.bin", vec![3u8; 1000]).await;
    assert_eq!(r3.status(), 201);
    let _id3 = r3.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let resp = client
        .get(format!("{}/api/media/{id2}", srv.http()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "id2 must have been evicted");
    let resp = client
        .get(format!("{}/api/media/{id1}", srv.http()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "id1 (recently used) survives");
}

#[tokio::test]
async fn group_chat_fanout() {
    let srv = start(|_| {}).await;
    for h in ["alice", "bob", "carol"] {
        register(&srv, h).await;
    }
    let ta = login(&srv, "alice").await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let tb = login(&srv, "bob").await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let resp = bearer(
        &srv,
        "POST",
        "/api/chats",
        &ta,
        Some(json!({
            "kind": "group", "title": "self-hosting crew", "members": ["bob", "carol"]
        })),
    )
    .await;
    assert_eq!(resp.status(), 201);
    let chat_id = resp.json::<Value>().await.unwrap()["chat_id"]
        .as_i64()
        .unwrap();

    // Chat list shows the group with all members.
    let resp = bearer(&srv, "GET", "/api/chats", &tb, None).await;
    let chats = resp.json::<Vec<Value>>().await.unwrap();
    let group = chats
        .iter()
        .find(|c| c["id"].as_i64() == Some(chat_id))
        .unwrap();
    assert_eq!(group["title"].as_str().unwrap(), "self-hosting crew");
    assert_eq!(group["member_count"].as_i64().unwrap(), 3);

    // Carol (offline) gets her copy from the REST fallback after alice sends.
    let (mut ws_a, _) = ws_connect_and_hello(&srv, &ta).await;
    send_frame(
        &mut ws_a,
        &ClientFrame::Send {
            client_msg_id: ClientMsgId::from_bytes([9; 16]),
            chat: koko_protocol::ChatId(chat_id),
            kind: MessageKind::Text,
            body: b"group hello".to_vec(),
            created_at: 1,
        },
    )
    .await;
    // Wait for the Sent ack — the durable commit point — before checking carol.
    let mut got_sent = false;
    for _ in 0..3 {
        if let ServerFrame::Sent { .. } = read_frame(&mut ws_a).await {
            got_sent = true;
            break;
        }
    }
    assert!(got_sent, "group send must be acked");

    let tc = login(&srv, "carol").await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let resp = bearer(&srv, "GET", "/api/messages?since=0", &tc, None).await;
    let inbox = resp.json::<Value>().await.unwrap();
    let msgs = inbox["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["sender_handle"].as_str().unwrap(), "alice");
}
