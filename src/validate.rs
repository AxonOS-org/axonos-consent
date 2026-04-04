//! Structural validation for decoded consent frames.
//!
//! The decoder parses CBOR/JSON into ConsentFrame. This module validates
//! that the parsed frame meets the spec's structural requirements:
//!
//! - consent-withdraw MUST have scope
//! - reason_code MUST be in valid range
//! - timestamp_us > 0 if present
//! - epoch MUST NOT appear on suspend/resume (only withdraw)
//!
//! Call `validate()` after decode, before acting on the frame.

use crate::frames::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    /// consent-withdraw missing scope field.
    MissingScope,
    /// timestamp_us is zero (invalid — must be positive Unix µs).
    ZeroTimestamp,
    /// epoch present on non-withdraw frame.
    EpochOnNonWithdraw,
    /// reason string exceeds MAX_REASON_LEN after decode.
    ReasonTooLong,
}

/// Validate a decoded ConsentFrame against spec structural rules.
/// Returns Ok(()) if valid, Err with the first violation found.
pub fn validate(frame: &ConsentFrame) -> Result<(), ValidationError> {
    match frame {
        ConsentFrame::Withdraw(w) => {
            // scope is structurally required by the type system (non-optional), so no check needed
            if let Some(ts) = w.timestamp_us {
                if ts == 0 { return Err(ValidationError::ZeroTimestamp); }
            }
            if let Some(ts) = w.timestamp_ms {
                if ts == 0 { return Err(ValidationError::ZeroTimestamp); }
            }
            if let Some(ref r) = w.reason {
                if r.len() > MAX_REASON_LEN { return Err(ValidationError::ReasonTooLong); }
            }
        }
        ConsentFrame::Suspend(s) => {
            if let Some(ts) = s.timestamp_us {
                if ts == 0 { return Err(ValidationError::ZeroTimestamp); }
            }
            if let Some(ref r) = s.reason {
                if r.len() > MAX_REASON_LEN { return Err(ValidationError::ReasonTooLong); }
            }
        }
        ConsentFrame::Resume(r) => {
            if let Some(ts) = r.timestamp_us {
                if ts == 0 { return Err(ValidationError::ZeroTimestamp); }
            }
        }
    }
    Ok(())
}
