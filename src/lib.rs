//! # axonos-consent
//!
//! Protocol-level consent enforcement for AxonOS.
//!
//! This crate implements the [AxonOS Consent Specification v0.4.0](https://github.com/AxonOS-org/axonos-consent/blob/main/SPEC.md) —
//! a kernel-level finite-state machine with three states (`Granted`, `Suspended`,
//! `Withdrawn`) that mediates user permission for `IntentObservation` flow.
//!
//! The crate is `#![no_std]` and is built for ARMv8-M Cortex-M targets, but the
//! types are platform-agnostic and run unchanged on hosted Rust for unit testing.
//!
//! # Quickstart
//!
//! ```
//! use axonos_consent::{ConsentMachine, ConsentState};
//!
//! let manifest_id: u16 = 1;
//! let trusted_path_pubkey = [0u8; 32];
//! let machine = ConsentMachine::new(manifest_id, trusted_path_pubkey);
//! assert_eq!(machine.state(), ConsentState::Granted);
//! ```
//!
//! # Authorship
//!
//! Solo specification and reference implementation by **Denis Yermakou**.
//! AxonOS Project, Singapore.
//!
//! # License
//!
//! - Code: Apache-2.0 OR MIT
//! - Specification text: CC-BY-SA-4.0
//! - Test vectors: CC0-1.0

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unused_must_use)]

pub mod crypto;
pub mod error;
pub mod interlock;
pub mod state;
pub mod wire;

pub use crate::error::ConsentError;
pub use crate::interlock::ObservationGate;
pub use crate::state::{ConsentMachine, ConsentState};
pub use crate::wire::ConsentEvent;

/// Specification version this crate implements.
pub const SPEC_VERSION: &str = "0.4.0";

/// Crate version (independent of spec version after v0.3.0).
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
