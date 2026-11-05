//! Roundtrip tests over the full frame vocabulary.

use koko_protocol::codec::{decode_client, decode_server, encode_client, encode_server};

fn roundtrip_client(frame: &koko_protocol::ClientFrame) {
    let bytes = encode_client(frame).unwrap();
    assert_eq!(decode_client(&bytes).unwrap(), *frame);
}

fn roundtrip_server(frame: &koko_protocol::ServerFrame) {
    let bytes = encode_server(frame).unwrap();
    assert_eq!(decode_server(&bytes).unwrap(), *frame);
}

#[test]
fn every_client_frame_roundtrips() {
    use koko_protocol::ClientFrame;
    use koko_protocol::MessageKind;

    roundtrip_client(&ClientFrame::Hello {
        token: "tok_tok_tok".into(),
        resume_from: None,
    });
    roundtrip_client(&ClientFrame::Hello {
        token: "tok".into(),
        resume_from: Some(41),
    });
    roundtrip_client(&ClientFrame::Send {
        client_msg_id: koko_protocol::ClientMsgId::from_bytes([0xab; 16]),
        chat: koko_protocol::ChatId(9),
        kind: MessageKind::Voice,
        body: vec![1, 2, 3, 255, 0],
        created_at: 1_700_000_000,
    });
    roundtrip_client(&ClientFrame::Ack {
        chat: koko_protocol::ChatId(9),
        up_to_seq: 12,
    });
    roundtrip_client(&ClientFrame::Typing {
        chat: koko_protocol::ChatId(9),
        on: true,
    });
    roundtrip_client(&ClientFrame::Ping);
}

#[test]
fn every_server_frame_roundtrips() {
    use koko_protocol::{ClientMsgId, Envelope, ErrorCode, ServerFrame, UserId};

    roundtrip_server(&ServerFrame::Hello {
        user: UserId(1),
        handle: "mira".into(),
        next_seq: 42,
        server_time: 1_700_000_000,
        resumed_messages: 3,
    });
    roundtrip_server(&ServerFrame::Envelope {
        env: Envelope {
            seq: 41,
            chat: koko_protocol::ChatId(2),
            sender: UserId(7),
            sender_handle: "kai".into(),
            kind: koko_protocol::MessageKind::Text,
            body: "self-host forever".as_bytes().to_vec(),
            created_at: 1_700_000_001,
        },
    });
    roundtrip_server(&ServerFrame::Sent {
        client_msg_id: ClientMsgId::from_bytes([9; 16]),
        seq: 42,
    });
    roundtrip_server(&ServerFrame::Typing {
        chat: koko_protocol::ChatId(2),
        from: UserId(3),
        from_handle: "anna".into(),
        on: false,
    });
    roundtrip_server(&ServerFrame::Error {
        code: ErrorCode::RateLimited,
        message: "slow down".into(),
    });
    roundtrip_server(&ServerFrame::Pong);
}

#[test]
fn body_bytes_survive_arbitrary_binary() {
    use koko_protocol::{ClientFrame, MessageKind};

    // 64 KiB of non-UTF-8 bytes: the body is opaque and must not be touched.
    let body: Vec<u8> = (0..65_536).map(|i| (i % 256) as u8).collect();
    let frame = ClientFrame::Send {
        client_msg_id: koko_protocol::ClientMsgId::from_bytes([1; 16]),
        chat: koko_protocol::ChatId(1),
        kind: MessageKind::File,
        body,
        created_at: 0,
    };
    let bytes = encode_client(&frame).unwrap();
    assert_eq!(decode_client(&bytes).unwrap(), frame);
}

#[test]
fn decoding_a_server_frame_as_client_fails_cleanly() {
    let bytes = encode_server(&koko_protocol::ServerFrame::Pong).unwrap();
    // Correct magic, wrong frame type: must error, never panic.
    assert!(decode_client(&bytes).is_err());
}
