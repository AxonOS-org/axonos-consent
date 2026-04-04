//! Unified error taxonomy for axonos-consent.
//!
//! All errors across decode, validation, invariants, engine, and encoding
//! are representable through this type. Zero-alloc, Copy.
//!
//! ## Classification
//!
//! | Category | Severity | Examples |
//! |----------|----------|---------|
//! | Decode | Hard reject | Malformed CBOR, unsupported type |
//! | Validation | Hard reject | Zero timestamp, reason too long |
//! | Invariant | Hard reject (MUST) | WITHDRAWN → RESUME |
//! | Invariant | Soft warning (SHOULD) | Missing timestamp on withdraw |
//! | Engine | Operational | Peer not found, table full |
//! | Encode | Operational | Buffer too small |

use crate::codec::cbor::{DecodeError, EncodeError};
use crate::invariants::{InvariantViolation, InvariantWarning};
use crate::state::TransitionError;

/// Top-level error enum. Every error path in the crate maps here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Wire-level decode failure (malformed CBOR, bounds exceeded).
    Decode(DecodeError),
    /// Structural invariant violation (MUST-level, §10).
    Invariant(InvariantViolation),
    /// State machine transition rejected.
    Transition(TransitionError),
    /// Encode buffer too small.
    Encode(EncodeError),
}

impl From<DecodeError> for Error {
    fn from(e: DecodeError) -> Self { Error::Decode(e) }
}
impl From<InvariantViolation> for Error {
    fn from(e: InvariantViolation) -> Self { Error::Invariant(e) }
}
impl From<TransitionError> for Error {
    fn from(e: TransitionError) -> Self { Error::Transition(e) }
}
impl From<EncodeError> for Error {
    fn from(e: EncodeError) -> Self { Error::Encode(e) }
}
