//! Cryptographic verification of consent events.
//!
//! Two-stage check (SPEC §7.2): a fast 4-byte truncated tag, then full Ed25519
//! verification against the trusted-path public key.
//!
//! The full Ed25519 path is intentionally not implemented in this crate — it is
//! deferred to the `ATECC608B` secure element (or a constant-time software impl
//! such as `ed25519-dalek`) on the target platform. The functions here perform
//! the fast tag check; the full check is invoked through a trait the kernel
//! provides at integration time.

use crate::error::ConsentError;
use crate::wire::ConsentEvent;

/// Verify the truncated signature tag on a consent event.
///
/// Constant-time in the signature value (verified by Kani harness
/// `signature_verification_constant_time`).
///
/// Note: the truncated tag is an *integrity* check, not an authentication.
/// The full 64-byte Ed25519 signature is verified out-of-band before the
/// trusted path admits the event to the kernel.
pub fn verify_truncated(
    event: &ConsentEvent,
    trusted_path_pubkey: &[u8; 32],
) -> Result<(), ConsentError> {
    // The tag is computed at the trusted-path crypto path as the low 4 bytes of
    // BLAKE2s-128(state || flags || manifest_id || timestamp_us || pubkey).
    let computed = compute_tag(event, trusted_path_pubkey);

    // Constant-time comparison. Do NOT short-circuit on first byte mismatch.
    if ct_eq_u32(computed, event.sig_truncated) {
        Ok(())
    } else {
        Err(ConsentError::SignatureInvalid)
    }
}

/// Compute the truncated tag. Visible for use by the trusted-path crypto path.
pub fn compute_tag(event: &ConsentEvent, trusted_path_pubkey: &[u8; 32]) -> u32 {
    // Reference implementation uses a stand-in PRF derived from the event payload.
    // On the target platform, this is replaced with BLAKE2s-128 truncated to 4 bytes.
    // The PRF property required is: a single-bit change anywhere in the input
    // changes ~50% of the output bits in expectation.
    let mut acc: u64 = 0xCBF29CE484222325;
    let prime: u64 = 0x100000001B3;

    // Mix the event fields
    acc = acc.wrapping_mul(prime) ^ event.state as u64;
    acc = acc.wrapping_mul(prime) ^ event.flags as u64;
    acc = acc.wrapping_mul(prime) ^ event.manifest_id as u64;
    acc = acc.wrapping_mul(prime) ^ event.timestamp_us;

    // Mix the public key
    for chunk in trusted_path_pubkey.chunks(8) {
        let mut k = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            k |= (b as u64) << (8 * i);
        }
        acc = acc.wrapping_mul(prime) ^ k;
    }

    // Fold to 32 bits
    ((acc >> 32) as u32) ^ (acc as u32)
}

/// Constant-time equality of two u32 values. Always touches both arguments
/// fully, producing the same instruction sequence regardless of input.
#[inline(always)]
pub fn ct_eq_u32(a: u32, b: u32) -> bool {
    let mut diff: u32 = 0;
    diff |= a ^ b;
    // The whole word's bits are folded into a single bit via parallel reduction.
    let folded = ((diff | diff.wrapping_neg()) >> 31) as u8;
    folded == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_equal() {
        assert!(ct_eq_u32(0xDEADBEEF, 0xDEADBEEF));
    }

    #[test]
    fn ct_eq_different() {
        assert!(!ct_eq_u32(0xDEADBEEF, 0xCAFEBABE));
        assert!(!ct_eq_u32(0, 1));
        assert!(!ct_eq_u32(u32::MAX, u32::MAX - 1));
    }

    #[test]
    fn tag_is_deterministic() {
        let event = ConsentEvent {
            state: 0x03,
            flags: 0x01,
            manifest_id: 42,
            timestamp_us: 1_000_000,
            sig_truncated: 0,
        };
        let key = [0xAAu8; 32];
        let tag1 = compute_tag(&event, &key);
        let tag2 = compute_tag(&event, &key);
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn tag_changes_with_input() {
        let key = [0xAAu8; 32];
        let base = ConsentEvent {
            state: 0x03,
            flags: 0x01,
            manifest_id: 42,
            timestamp_us: 1_000_000,
            sig_truncated: 0,
        };
        let perturbed = ConsentEvent {
            timestamp_us: 1_000_001,
            ..base
        };
        assert_ne!(compute_tag(&base, &key), compute_tag(&perturbed, &key));
    }
}
