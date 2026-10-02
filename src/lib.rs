// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! # axonos-consent
//!
//! Consent enforcement for brain–computer interfaces: the reference
//! implementation of the [AxonOS Consent Specification](https://github.com/AxonOS-org/axonos-consent/blob/main/SPEC.md).
//!
//! A consent decision arrives from the trusted path as a 96-byte frame: a
//! 32-byte record and its Ed25519 signature. The machine admits it only if the
//! signature verifies under the trusted-path key, the sequence number is fresh,
//! and the transition is admissible; the new state then governs every
//! publication through one atomic word.
//!
//! ```
//! # #[cfg(feature = "ed25519")] {
//! use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict};
//! # use ed25519_dalek::{Signer, SigningKey};
//! # use axonos_consent::wire::{assemble, ConsentRecord};
//! # let signer = SigningKey::from_bytes(&[7u8; 32]);
//! # let trusted_key = signer.verifying_key().to_bytes();
//! # let sign = |r: ConsentRecord| assemble(&r, &signer.sign(&r.encode()).to_bytes());
//!
//! let mut machine = ConsentMachine::new(1, trusted_key, Ed25519Strict)?;
//! assert_eq!(machine.state(), ConsentState::Granted);
//!
//! // A signed withdrawal from the trusted path.
//! let frame = sign(ConsentRecord::new(ConsentState::Withdrawn, 1, 1, 0));
//! assert_eq!(machine.handle(&frame)?, ConsentState::Withdrawn);
//!
//! // From now on, no observation can be published.
//! assert!(machine.gate().try_publish().is_err());
//!
//! // The same frame again is a replay.
//! assert!(machine.handle(&frame).is_err());
//! # }
//! # Ok::<(), axonos_consent::ConsentError>(())
//! ```
//!
//! ## Layers
//!
//! | Module | Property | Evidence |
//! |:--|:--|:--|
//! | [`wire`] | decoding is total and canonical | Kani |
//! | [`auth`] | no transition without a valid signature | Kani |
//! | [`machine`] | no sequence number is admitted twice | Kani |
//! | [`state`] | `Withdrawn` is absorbing | Kani |
//! | [`gate`] | no publication after a withdrawal | loom |
//! | [`dual_control`] | one party can stop, only two can resume | Kani, tests |
//!
//! The crate is `#![no_std]`, forbids `unsafe`, never allocates and never
//! panics on any input.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unused_must_use)]

pub mod auth;
pub mod dual_control;
pub mod error;
pub mod gate;
pub mod machine;
pub mod state;
pub mod wire;

#[cfg(kani)]
mod proofs;

#[cfg(feature = "ed25519")]
pub use crate::auth::Ed25519Strict;
pub use crate::auth::{Authenticated, SignatureVerifier};
pub use crate::dual_control::{CoAuthOutcome, DualControlMachine};
pub use crate::error::ConsentError;
pub use crate::gate::{PublicationGate, Suppressed};
pub use crate::machine::{ConsentMachine, Persisted};
pub use crate::state::ConsentState;
pub use crate::wire::{ConsentRecord, Party};

/// The version of the AxonOS Consent Specification this crate implements.
pub const SPEC_VERSION: &str = "0.6.0";

/// The crate version.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
