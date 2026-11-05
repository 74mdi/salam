//! The WebSocket connector — one long-lived subscription task that owns the
//! socket, reconnects with backoff, and bridges iced ↔ tokio channels.

use futures_util::{SinkExt, StreamExt};
use koko_protocol::codec::{decode_server, encode_client};
use koko_protocol::{ClientFrame, ServerFrame};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Events surfaced to the UI.
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// The connector is up; send client frames through this handle.
    Ready(UnboundedSender<ClientFrame>),
    /// Handshake + auth done.
    Connected {
        /// Server's current cursor for this user.
        next_seq: u64,
    },
    /// Socket lost; the connector will retry (queued sends are kept).
    Disconnected,
    /// A server frame arrived.
    Server(ServerFrame),
}

/// Run the connector forever. `output` is how the UI hears about it.
pub async fn run(
    server: String,
    token: String,
    mut output: iced::futures::channel::mpsc::Sender<super::app::Message>,
) {
    // Channel for outgoing client frames. Unbounded: while offline, sends
    // queue here and flush on reconnect — the client's retry behavior.
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<ClientFrame>();
    let _ = output
        .send(super::app::Message::Ws(WsEvent::Ready(out_tx)))
        .await;

    let ws_url = server.replace("http", "ws").replace("https", "wss") + "/ws";
    let mut backoff_secs = 1u64;

    loop {
        log::info!("ws: connecting to {ws_url}");
        match tokio_tungstenite::connect_async(ws_url.as_str()).await {
            Ok((mut ws, _)) => {
                backoff_secs = 1;
                // Authenticate. resume_from stays None: the REST catch-up
                // after `Connected` is the authoritative gap-filler.
                let hello = ClientFrame::Hello {
                    token: token.clone(),
                    resume_from: None,
                };
                let mut authenticated = false;
                if let Ok(bytes) = encode_client(&hello) {
                    let _ = ws.send(WsMessage::Binary(bytes)).await;
                } else {
                    continue;
                }

                'conn: loop {
                    tokio::select! {
                        frame = out_rx.recv() => {
                            if let Some(frame) = frame {
                                if let Ok(bytes) = encode_client(&frame) {
                                    if ws.send(WsMessage::Binary(bytes)).await.is_err() {
                                        break 'conn;
                                    }
                                }
                            } else {
                                // UI dropped the handle; task is done.
                                let _ = ws.close(None).await;
                                return;
                            }
                        }
                        msg = ws.next() => {
                            match msg {
                                Some(Ok(WsMessage::Binary(bytes))) => {
                                    match decode_server(&bytes) {
                                        Ok(server_frame) => {
                                            if let ServerFrame::Hello { next_seq, .. } = &server_frame {
                                                if !authenticated {
                                                    authenticated = true;
                                                    let _ = output.send(super::app::Message::Ws(
                                                        WsEvent::Connected { next_seq: *next_seq },
                                                    )).await;
                                                }
                                            }
                                            let _ = output.send(
                                                super::app::Message::Ws(WsEvent::Server(server_frame)),
                                            ).await;
                                        }
                                        Err(e) => log::warn!("ws: bad frame: {e}"),
                                    }
                                }
                                Some(Ok(WsMessage::Ping(p))) => {
                                    let _ = ws.send(WsMessage::Pong(p)).await;
                                }
                                Some(Ok(_)) => {}
                                Some(Err(e)) => {
                                    log::info!("ws: error: {e}");
                                    break 'conn;
                                }
                                None => break 'conn,
                            }
                        }
                    }
                }
            }
            Err(e) => log::info!("ws: connect failed: {e}"),
        }

        let _ = output
            .send(super::app::Message::Ws(WsEvent::Disconnected))
            .await;
        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(15);
    }
}
