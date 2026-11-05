//! End-to-end db behavior tests: the fanout transaction, idempotent sends,
//! cursor replay and media quota/LRU eviction.

use koko_db::{chats, media, messages, users};

async fn fresh_db() -> (sqlx::SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let pool = koko_db::connect(&dir.path().join("koko.db")).await.unwrap();
    (pool, dir)
}

async fn user(pool: &sqlx::SqlitePool, handle: &str) -> i64 {
    users::create_user(pool, handle, handle, "$argon2id$phc", 1_700_000_000)
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn fanout_allocates_monotonic_seqs_per_user() {
    let (pool, _dir) = fresh_db().await;
    let (a, b, c) = (
        user(&pool, "alice").await,
        user(&pool, "bob").await,
        user(&pool, "carol").await,
    );
    let now = 1_700_000_100;
    let chat = chats::create_group(&pool, a, "crew", &[a, b, c], now)
        .await
        .unwrap();

    for i in 0..5i64 {
        let fanout = messages::insert_message(
            &pool,
            &messages::NewMessage {
                chat_id: chat,
                sender_id: a,
                client_msg_id: [i as u8; 16],
                kind: 0,
                body: b"hello",
                created_at: now + i,
            },
        )
        .await
        .unwrap();
        assert!(!fanout.duplicate);
        assert_eq!(fanout.recipients.len(), 3);
        assert_eq!(fanout.sender_seq, (i + 1) as u64);
        for r in &fanout.recipients {
            assert_eq!(r.seq, (i + 1) as u64, "every member gets their own seq");
        }
    }

    // Cursor replay for bob: everything from 0, in order.
    let envs = messages::fetch_since(&pool, b, 0, 100).await.unwrap();
    assert_eq!(envs.len(), 5);
    assert!(envs.windows(2).all(|w| w[0].seq < w[1].seq));
    assert_eq!(messages::current_seq(&pool, b).await.unwrap(), 5);
}

#[tokio::test]
async fn duplicate_send_is_idempotent() {
    let (pool, _dir) = fresh_db().await;
    let (a, b) = (user(&pool, "alice").await, user(&pool, "bob").await);
    let chat = chats::direct_chat(&pool, a, b, 1).await.unwrap();

    let msg = messages::NewMessage {
        chat_id: chat,
        sender_id: a,
        client_msg_id: [42; 16],
        kind: 0,
        body: b"retry me",
        created_at: 1,
    };
    let first = messages::insert_message(&pool, &msg).await.unwrap();
    let second = messages::insert_message(&pool, &msg).await.unwrap();

    assert!(second.duplicate);
    assert_eq!(first.message_id, second.message_id);
    assert_eq!(first.sender_seq, second.sender_seq);

    // Exactly one message stored, one delivery per member.
    assert_eq!(messages::count_messages(&pool).await.unwrap(), 1);
    let envs = messages::fetch_since(&pool, b, 0, 100).await.unwrap();
    assert_eq!(envs.len(), 1);
}

#[tokio::test]
async fn non_member_cannot_send() {
    let (pool, _dir) = fresh_db().await;
    let (a, b, e) = (
        user(&pool, "alice").await,
        user(&pool, "bob").await,
        user(&pool, "eve").await,
    );
    let chat = chats::direct_chat(&pool, a, b, 1).await.unwrap();
    let err = messages::insert_message(
        &pool,
        &messages::NewMessage {
            chat_id: chat,
            sender_id: e,
            client_msg_id: [1; 16],
            kind: 0,
            body: b"intrusion",
            created_at: 1,
        },
    )
    .await;
    assert!(matches!(err, Err(koko_db::DbError::NotMember(_))));
}

#[tokio::test]
async fn direct_chats_are_reused_not_duplicated() {
    let (pool, _dir) = fresh_db().await;
    let (a, b) = (user(&pool, "alice").await, user(&pool, "bob").await);
    let c1 = chats::direct_chat(&pool, a, b, 1).await.unwrap();
    let c2 = chats::direct_chat(&pool, b, a, 2).await.unwrap();
    assert_eq!(c1, c2, "same pair must map to the same direct chat");
}

#[tokio::test]
async fn notes_chat_is_private_and_reused() {
    let (pool, _dir) = fresh_db().await;
    let alice = user(&pool, "alice").await;
    let first = chats::note_to_self(&pool, alice, 1).await.unwrap();
    let second = chats::note_to_self(&pool, alice, 2).await.unwrap();

    assert_eq!(first, second);
    let summary = chats::list_chats(&pool, alice).await.unwrap();
    assert_eq!(summary.len(), 1);
    assert_eq!(summary[0].title.as_deref(), Some("Notes to self"));
    assert_eq!(summary[0].member_count, 1);
}

#[tokio::test]
async fn display_name_can_be_updated() {
    let (pool, _dir) = fresh_db().await;
    let alice = user(&pool, "alice").await;
    let updated = users::update_display_name(&pool, alice, "Alice Example")
        .await
        .unwrap();

    assert_eq!(updated.handle, "alice");
    assert_eq!(updated.display_name, "Alice Example");
}

#[tokio::test]
async fn media_quota_lru_eviction() {
    let (pool, _dir) = fresh_db().await;
    let owner = user(&pool, "alice").await;
    let now = 1_700_000_000;

    // Three files: oldest access first, newest last.
    for (i, size) in [1000, 1000, 1000].into_iter().enumerate() {
        media::insert_media(
            &pool,
            &format!("m{i}"),
            owner,
            "f.bin",
            "application/octet-stream",
            size,
            now,
        )
        .await
        .unwrap();
        // m0 accessed long ago, m2 just now.
        media::touch_media(&pool, &format!("m{i}"), now + (i as i64) * 100)
            .await
            .unwrap();
    }
    assert_eq!(media::usage_bytes(&pool, owner).await.unwrap(), 3000);

    // Free 2000 bytes: evicts m0 and m1 (oldest access), keeps m2.
    let evicted = media::evict_lru(&pool, owner, 2000).await.unwrap();
    assert_eq!(evicted.len(), 2);
    assert_eq!(evicted[0].id, "m0");
    assert_eq!(evicted[1].id, "m1");
    assert_eq!(media::usage_bytes(&pool, owner).await.unwrap(), 1000);
    assert!(media::get_media(&pool, "m0").await.unwrap().is_none());
    assert!(media::get_media(&pool, "m2").await.unwrap().is_some());
}

#[tokio::test]
async fn unread_counts_follow_read_watermark() {
    let (pool, _dir) = fresh_db().await;
    let (a, b) = (user(&pool, "alice").await, user(&pool, "bob").await);
    let chat = chats::direct_chat(&pool, a, b, 1).await.unwrap();
    for i in 0..4i64 {
        messages::insert_message(
            &pool,
            &messages::NewMessage {
                chat_id: chat,
                sender_id: a,
                client_msg_id: [i as u8; 16],
                kind: 0,
                body: b"x",
                created_at: i,
            },
        )
        .await
        .unwrap();
    }
    let list = chats::list_chats(&pool, b).await.unwrap();
    assert_eq!(list[0].unread, 4);

    chats::set_last_read(&pool, chat, b, 3).await.unwrap();
    let list = chats::list_chats(&pool, b).await.unwrap();
    assert_eq!(list[0].unread, 1, "seq 3 and below are read");

    // Watermark is monotonic: moving it back changes nothing.
    chats::set_last_read(&pool, chat, b, 1).await.unwrap();
    let list = chats::list_chats(&pool, b).await.unwrap();
    assert_eq!(list[0].unread, 1);
}
