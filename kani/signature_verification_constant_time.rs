//! Kani harness: `ct_eq_u32` is functionally equivalent to `==`.
//!
//! Scope, stated precisely: this proves a *functional* property only — that the
//! branchless comparison returns the correct answer for all inputs. It does
//! **not** prove constant-time execution. Timing and side-channel properties
//! are outside what a bounded model checker over Rust MIR can establish. See
//! SPEC §7.3, which records the requirement as unverified.

use axonos_consent::crypto::ct_eq_u32;

#[kani::proof]
fn signature_verification_constant_time() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();

    let result = ct_eq_u32(a, b);
    assert_eq!(result, a == b);
}
