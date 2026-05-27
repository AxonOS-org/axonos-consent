#![no_main]
//! Fuzz target — encoder / decoder canonical-encoding symmetry.
//!
//! The AxonOS Consent wire format (SPEC §6) is *canonical*: every accepted
//! 16-byte buffer has exactly one interpretation, and re-encoding that
//! interpretation reproduces the buffer byte-for-byte. This target searches
//! for any input where decode→encode is not the identity — an asymmetry that
//! would let two distinct buffers denote the same consent event.

use libfuzzer_sys::fuzz_target;
use axonos_consent::ConsentEvent;

fuzz_target!(|data: &[u8]| {
    if let Ok(event) = ConsentEvent::from_bytes(data) {
        let reencoded = event.to_bytes();
        assert_eq!(
            &reencoded[..], data,
            "non-canonical encoding: an accepted buffer did not round-trip",
        );
        // Decoding the re-encoding must yield the identical event.
        let again = ConsentEvent::from_bytes(&reencoded)
            .expect("a buffer produced by to_bytes() must itself decode");
        assert_eq!(again, event, "decode after encode is not the identity");
    }
});
