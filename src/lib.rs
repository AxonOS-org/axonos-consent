//! # axonos-consent
//!
//! Implementation of the MMP Consent Extension v0.1.0 for AxonOS.
//!
//! Specification: <https://sym.bot/spec/mmp-consent>
//! Base protocol: MMP v0.2.0 <https://sym.bot/spec/mmp>
//!
//! ## Architecture
//!
//! This crate implements the consent primitive at two levels:
//!
//! - **Protocol level**: ConsentState, ConsentEngine, frame codec (CBOR + JSON)
//! - **Hardware level**: StimGuard integration for bidirectional BCI
//!   (feature-gated behind `stim-guard`)
//!
//! ## Encoding Strategy
//!
//! - **Local IPC (M4F ↔ A53)**: CBOR — compact, no string parsing on critical path
//! - **Relay boundary (A53 ↔ mesh peers)**: JSON — compatible with MMP reference
//!   implementations (sym for Node.js, sym-swift for iOS/macOS)
//!
//! Frame types use string identifiers per MMP Section 7:
//! `"consent-withdraw"`, `"consent-suspend"`, `"consent-resume"`

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod state;
pub mod engine;
pub mod frames;
pub mod reason;
pub mod codec;

#[cfg(feature = "stim-guard")]
pub mod stim_guard;

pub use state::ConsentState;
pub use engine::ConsentEngine;
pub use frames::{ConsentWithdraw, ConsentSuspend, ConsentResume, ConsentFrame};
pub use reason::ReasonCode;
