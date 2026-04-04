//! # axonos-consent
//!
//! MMP Consent Extension v0.1.0 — Rust implementation for AxonOS.
//! Spec: <https://sym.bot/spec/mmp-consent>
//!
//! ## Zero-allocation guarantee
//!
//! The default build (`#![no_std]`, no features) is **fully allocation-free**.
//! All frame types use fixed-size buffers. No `String`, no `Vec`, no heap.
//!
//! The `alloc` feature enables `String`-backed reason fields for relay
//! boundary use. The `json` feature enables JSON codec (requires `alloc` + `std`).
//!
//! ## Critical path (M4F → A53 IPC)
//!
//! `ConsentState` + `ConsentEngine` + `codec::cbor` — all zero-alloc.
//! WCET target: <1µs for state transition + StimGuard lockout.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod state;
pub mod engine;
pub mod frames;
pub mod reason;
pub mod codec;
pub mod validate;

#[cfg(feature = "stim-guard")]
pub mod stim_guard;

pub use state::ConsentState;
pub use engine::ConsentEngine;
pub use frames::{ConsentFrame, Scope};
pub use reason::ReasonCode;
