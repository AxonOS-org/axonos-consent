//! Dual codec for consent frames.
//!
//! - CBOR: local IPC between M4F and A53 (no string parsing on critical path)
//! - JSON: relay boundary transcoding (MMP reference implementations use JSON)

pub mod cbor;

#[cfg(feature = "json")]
pub mod json;
