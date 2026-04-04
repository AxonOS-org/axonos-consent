//! # axonos-consent
//!
//! MMP Consent Extension v0.1.0 — reference implementation.
//!
//! Spec: <https://sym.bot/spec/mmp-consent>
//! Protocol version: `CONSENT_PROTOCOL_VERSION = 1`
//!
//! ## Zero-allocation guarantee
//!
//! Default build (`no_std`, no features) is allocation-free.
//! All frame types use fixed-size buffers. WCET bounded.
//!
//! ## Spec-to-code mapping
//!
//! | Spec section | Module | Purpose |
//! |---|---|---|
//! | §3 Frame types | `frames` | ConsentWithdraw/Suspend/Resume |
//! | §3.4 Reason codes | `reason` | ReasonCode registry |
//! | §4 State machine | `state` | ConsentState + transitions |
//! | §5.1 Enforcement | `engine` | ConsentEngine per-peer |
//! | §6.1 Frame filtering | `engine::allows_cognitive_frames` | Gating |
//! | §6.4 Gossip | `state::to_gossip_bits` | 2-bit encoding |
//! | §7 Frame registry | `codec::cbor` / `codec::json` | Wire format |
//! | §8 BCI considerations | `stim_guard` | DAC gate |
//! | §10 Conformance | `invariants` | MUST/SHOULD/MAY |
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Protocol version. Encoded in future handshake extensions.
pub const CONSENT_PROTOCOL_VERSION: u8 = 1;

pub mod state;
pub mod engine;
pub mod frames;
pub mod reason;
pub mod codec;
pub mod invariants;
pub mod error;

#[cfg(feature = "stim-guard")]
pub mod stim_guard;

pub use state::ConsentState;
pub use engine::ConsentEngine;
pub use frames::{ConsentFrame, Scope};
pub use reason::ReasonCode;
