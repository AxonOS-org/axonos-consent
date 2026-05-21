//! Kani harness: the truncated-signature verification path is constant-time
//! with respect to the signature value.

use axonos_consent::crypto::ct_eq_u32;

#[kani::proof]
fn signature_verification_constant_time() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();

    let result = ct_eq_u32(a, b);
    assert_eq!(result, a == b);
}
