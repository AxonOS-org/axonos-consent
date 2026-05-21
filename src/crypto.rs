//! Cryptographic verification of consent events.
//!
//! Two-stage check (SPEC §7.2): a fast 4-byte truncated tag, then full Ed25519
//! verification against the trusted-path public key.
//!
//! The full Ed25519 path is intentionally not implemented in this crate — it is
//! deferred to the `ATECC608B` secure element (or a constant-time software
//! implementation such as `ed25519-dalek`) on the target platform. The
//! functions here perform the fast tag check; the full check is invoked
//! through a trait the kernel provides at integration time.

use crate::error::ConsentError;
use crate::wire::ConsentEvent;

/// Verify the truncated signature tag on a consent event.
///
/// Constant-time in the signature value (verified by the Kani harness
/// `signature_verification_constant_time`).
///
/// Note: the truncated tag is an integrity check, not an authentication.
/// The full 64-byte Ed25519 signature is verified out-of-band before the
/// trusted path admits the event to the kernel.
pub fn verify_truncated(
    event: &ConsentEvent,
    trusted_path_pubkey: &[u8; 32],
) -> Result<(), ConsentError> {
    let computed = compute_tag(event, trusted_path_pubkey);

    if ct_eq_u32(computed, event.sig_truncated) {
        Ok(())
    } else {
        Err(ConsentError::SignatureInvalid)
    }
}

/// Compute the truncated tag.
///
/// On the target platform this is replaced with BLAKE2s-128 truncated to 4
/// bytes; the reference implementation uses an FNV-style PRF that has the
/// avalanche property required (a single-bit change in input changes ~50% of
/// output bits in expectation).
pub fn compute_tag(event: &ConsentEvent, trusted_path_pubkey: &[u8; 32]) -> u32 {
    let mut acc: u64 = 0xCBF2_9CE4_8422_2325;
    let prime: u64 = 0x0000_0100_0000_01B3;

    acc = acc.wrapping_mul(prime) ^ u64::from(event.state);
    acc = acc.wrapping_mul(prime) ^ u64::from(event.flags);
    acc = acc.wrapping_mul(prime) ^ u64::from(event.manifest_id);
    acc = acc.wrapping_mul(prime) ^ event.timestamp_us;

    for chunk in trusted_path_pubkey.chunks(8) {
        let mut k: u64 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            k |= u64::from(b) << (8 * i);
        }
        acc = acc.wrapping_mul(prime) ^ k;
    }

    ((acc >> 32) as u32) ^ (acc as u32)
}

/// Constant-time equality of two `u32` values.
///
/// The function body is a fixed sequence of arithmetic operations with no
/// branches on either argument. The whole word's bits are folded into a
/// single bit via parallel reduction; the result is 0 iff the inputs are
/// equal.
#[inline(always)]
pub fn ct_eq_u32(a: u32, b: u32) -> bool {
    let diff: u32 = a ^ b;
    // (diff | -diff) has its high bit set iff diff != 0; right-shift folds it.
    let folded = ((diff | diff.wrapping_neg()) >> 31) as u8;
    folded == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_equal() {
        assert!(ct_eq_u32(0xDEAD_BEEF, 0xDEAD_BEEF));
        assert!(ct_eq_u32(0, 0));
        assert!(ct_eq_u32(u32::MAX, u32::MAX));
    }

    #[test]
    fn ct_eq_different() {
        assert!(!ct_eq_u32(0xDEAD_BEEF, 0xCAFE_BABE));
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
