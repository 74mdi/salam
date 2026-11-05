//! # koko-core
//!
//! Domain logic shared by the server, the native client and the web client:
//! identity rules, password hashing, session tokens, and the end-to-end
//! cipher abstraction.
//!
//! ## Unsafe policy
//!
//! `unsafe` is forbidden workspace-wide (see the root `Cargo.toml`).
//! The long-term plan carves out **one** exception — `koko-core`, for the
//! future `libsignal-client` FFI bindings — and every such block will carry
//! a documented safety argument. Until that lands, there is zero `unsafe`
//! anywhere in Koko.

pub mod e2e;
pub mod errors;
pub mod handles;
pub mod password;
pub mod time;
pub mod token;

pub use errors::KokoError;
pub use handles::{validate_display_name, validate_handle, HandleError};

/// Current semantic version reported by `/healthz` and the client UI.
pub const KOKO_VERSION: &str = env!("CARGO_PKG_VERSION");
