// Copyright (c) 2026 Denis Yermakou / AxonOS
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This file is part of axonos-consent. See LICENSE-APACHE or LICENSE-MIT for
// source-code licensing.

//! # axonos-consent
//!
//! AxonOS-native deterministic neural consent runtime for `no_std` BCI systems.
//!
//! This crate implements the AxonOS Consent Specification v0.3.0 as a
//! platform-agnostic finite-state machine for permissioning `IntentObservation`
//! flow inside an AxonOS deployment.
//!
//! Consent is runtime safety state, not a UI checkbox.
//!
//! ## Evidence posture
//!
//! Current public claims are L1/L2 only unless explicitly linked to external
//! hardware trace artifacts under AOS-0012.
//!
//! This crate does not claim clinical deployment readiness, regulatory approval,
//! final AxonOS conformance, or L3 hardware timing validation.
//!
//! ## Quickstart
//!
//! ```rust,no_run
//! use axonos_consent::{ConsentMachine, ConsentState};
//!
//! let manifest_id: u16 = 1;
//! let trusted_path_pubkey = [0u8; 32];
//! let machine = ConsentMachine::new(manifest_id, trusted_path_pubkey);
//!
//! assert_eq!(machine.state(), ConsentState::Granted);
//! ```
//!
//! ## Runtime pipeline
//!
//! The intended runtime flow is:
//!
//! 1. bounded wire decode;
//! 2. invariant check;
//! 3. deterministic state transition;
//! 4. delivery decision;
//! 5. optional observation gate.
//!
//! ## AxonOS Standard mapping
//!
//! | Standard artifact | Relevance |
//! |---|---|
//! | AOS-0004 | Neural permissions |
//! | AOS-0005 | Consent semantics |
//! | AOS-0009 | Security and privacy threat model |
//! | AOS-0012 | Hardware validation protocol |
//!
//! ## License
//!
//! - Code: Apache-2.0 OR MIT.
//! - Specification text: CC-BY-SA-4.0.
//! - Test vectors: CC0-1.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unused_must_use)]

/// Constant-time placeholder and digest helpers used by the consent wire layer.
pub mod crypto;

/// Error types returned by the consent runtime.
pub mod error;

/// Observation-gate integration point for stream delivery.
pub mod interlock;

/// Consent finite-state machine and transition rules.
pub mod state;

/// Wire-level consent events and frame decoding.
pub mod wire;

pub use crate::error::ConsentError;
pub use crate::interlock::ObservationGate;
pub use crate::state::{ConsentMachine, ConsentState};
pub use crate::wire::ConsentEvent;

/// Specification version this crate implements.
pub const SPEC_VERSION: &str = "0.3.0";

/// Crate version from Cargo metadata.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
