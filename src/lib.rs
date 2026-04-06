//! # axonos-consent
//!
//! MMP Consent Extension v0.1.0 — reference implementation.
//!
//! Spec: <https://sym.bot/spec/mmp-consent>
//! Protocol version: `CONSENT_PROTOCOL_VERSION = 1`
//!
//! ## Single entry point
//!
//! ```ignore
//! let result = engine.process_raw(peer_id, cbor_bytes, now_us)?;
//! ```
//!
//! This is the **only** function external code should call. It executes:
//! 1. CBOR decode (bounded, security-hardened)
//! 2. Invariant check (MUST violations → reject, SHOULD → warn)
//! 3. State transition (exhaustive 3×3 table, no wildcards)
//! 4. StimGuard callback (if withdrawal + feature enabled)
//!
//! ## Spec-to-code mapping
//!
//! | Spec § | Module | Purpose |
//! |--------|--------|---------|
//! | §3     | `frames` | ConsentWithdraw/Suspend/Resume |
//! | §3.4   | `reason` | ReasonCode registry |
//! | §4     | `state`  | ConsentState + `apply_frame()` (exhaustive) |
//! | §5.1   | `engine` | ConsentEngine + `process_raw()` |
//! | §6.1   | `engine::allows_cognitive_frames` | Frame gating |
//! | §6.4   | `state::to_gossip_bits` | 2-bit encoding |
//! | §7     | `codec::cbor` / `codec::json` | Wire format |
//! | §8     | `stim_guard` | DAC gate + timing contract |
//! | §10    | `invariants` | MUST/SHOULD/MAY enforcement |

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Protocol version. Wire-encoded in future handshake extensions.
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
pub use engine::{ConsentEngine, WithdrawAllResult};
pub use frames::{ConsentFrame, Scope};
pub use reason::ReasonCode;
pub use error::Error;
