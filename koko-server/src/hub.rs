//! In-process fanout hub: a `tokio::sync::broadcast` channel plus the
//! connection counter. SQLite is the source of truth; the hub is only the
//! fast path to sockets that are online right now. If nobody is listening,
//! `publish` is a no-op — offline members catch up from `deliveries` on
//! their next connect.

use std::sync::atomic::{AtomicUsize, Ordering};

use koko_protocol::{ChatId, Envelope, UserId};
use tokio::sync::broadcast;

/// Events pushed to every live WS session (each filters by recipient).
#[derive(Debug, Clone)]
pub enum HubEvent {
    /// A message fanned out to one recipient.
    Message {
        /// Recipient.
        recipient: UserId,
        /// The message envelope (with that recipient's seq).
        envelope: Envelope,
    },
    /// A typing indicator relayed to one recipient.
    Typing {
        /// Recipient.
        recipient: UserId,
        /// Chat it happened in.
        chat: ChatId,
        /// Who is typing.
        from: UserId,
        /// Their handle.
        from_handle: String,
        /// Started or stopped.
        on: bool,
    },
}

/// Broadcast capacity. A session that falls behind (slow client) gets
/// `RecvError::Lagged` and is told to catch up over REST — the hub never
/// blocks SQLite writes on slow readers.
pub const BROADCAST_CAPACITY: usize = 2048;

/// The hub.
#[derive(Debug)]
pub struct Hub {
    tx: broadcast::Sender<HubEvent>,
    connections: AtomicUsize,
}

impl Hub {
    /// New hub with [`BROADCAST_CAPACITY`].
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Hub {
            tx,
            connections: AtomicUsize::new(0),
        }
    }

    /// Push an event to all subscribers (no-op when nobody is online).
    pub fn publish(&self, event: HubEvent) {
        // Err(SendError) simply means zero subscribers right now.
        let _ = self.tx.send(event);
    }

    /// Subscribe (one per WS session).
    pub fn subscribe(&self) -> broadcast::Receiver<HubEvent> {
        self.tx.subscribe()
    }

    /// Current live connection count.
    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::Relaxed)
    }

    /// Register a connection; returns the new count.
    pub fn inc_connections(&self) -> usize {
        self.connections.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Release a connection.
    pub fn dec_connections(&self) {
        self.connections.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Default for Hub {
    fn default() -> Self {
        Hub::new()
    }
}
