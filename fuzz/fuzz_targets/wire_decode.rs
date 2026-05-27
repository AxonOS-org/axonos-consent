#![no_main]
//! Fuzz target — the wire-format decoder.
//!
//! `ConsentEvent::from_bytes` is the crate's only untrusted-input surface: the
//! one function that consumes a buffer the kernel did not produce. This target
//! asserts the decoder is **total** — for any byte slice it returns either
//! `Ok` or a typed `Err`, and never panics or reads out of bounds.

use libfuzzer_sys::fuzz_target;
use axonos_consent::ConsentEvent;

fuzz_target!(|data: &[u8]| {
    match ConsentEvent::from_bytes(data) {
        Ok(event) => {
            // Acceptance implies the input was exactly 16 bytes (SPEC §6.3).
            assert_eq!(data.len(), 16, "decoder accepted a non-16-byte buffer");
            // A decoded event must always re-encode without panicking.
            let _ = event.to_bytes();
        }
        Err(_) => {
            // A typed refusal is the correct outcome for any rejected input.
        }
    }
});
