//! # koko-protocol
//!
//! The single source of truth for the Koko wire format.
//!
//! Every crate in the workspace (server, native client, web client) links this
//! crate, which guarantees that client and server can never drift apart: the
//! envelope and frame types are compiled into both ends from one definition.
//!
//! Wire format (see `docs/PROTOCOL.md` for the full spec):
//!
//! * Transport: WebSocket binary messages (HTTP for auth, sync fallback and
//!   media).
//! * Framing: 4 magic bytes `KOK1` followed by a `bincode` (1.3, fixint,
//!   little-endian) encoded payload.
//! * Versioning: the magic byte `1` is the protocol version. A future
//!   incompatible change bumps it to `KOK2` and servers refuse old clients
//!   with [`ErrorCode::UnsupportedVersion`] instead of mis-decoding.
//!
//! Message bodies are **opaque bytes** by design: the server never needs to
//! understand them. Today the demo client sends UTF-8 text, tomorrow the same
//! field carries Signal-style ciphertext. The protocol does not change when
//! end-to-end encryption lands.
//!
//! **There is exactly one implementation of this crate.** The web client and
//! the native client must never grow their own envelope types — that rule is
//! the whole point of `koko-protocol`.

pub mod codec;
pub mod envelope;
pub mod frame;
pub mod ids;

pub use codec::{decode, encode, WireError, MAGIC, MAX_FRAME_SIZE};
pub use envelope::{Envelope, MessageKind};
pub use frame::{ClientFrame, ErrorCode, ServerFrame};
pub use ids::{ChatId, ClientMsgId, UserId};

/// Version of the protocol carried by the [`MAGIC`] constant.
pub const PROTOCOL_VERSION: u8 = 1;

/// Hard cap on a single message body carried over the WebSocket.
///
/// Larger payloads belong in the media store (HTTP upload, 25 MiB per file by
/// default); a message then references the media id in its small body.
pub const MAX_MESSAGE_BODY: usize = 256 * 1024;
