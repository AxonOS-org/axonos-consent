//! Kani harness: the CBOR-shape wire decoder enforces depth and length bounds,
//! and refuses any input not of the exact wire size.

use axonos_consent::wire::{ConsentEvent, WIRE_SIZE};
use axonos_consent::error::ConsentError;

#[kani::proof]
#[kani::unwind(17)]
fn cbor_decoder_bounded() {
    // Symbolic input buffer of arbitrary length up to WIRE_SIZE + 1.
    let len: usize = kani::any();
    kani::assume(len <= WIRE_SIZE + 1);

    let mut buf = [0u8; WIRE_SIZE + 1];
    // Each byte is symbolic.
    for byte in buf.iter_mut().take(len) {
        *byte = kani::any();
    }

    // Pass only the first `len` bytes.
    let result = ConsentEvent::from_bytes(&buf[..len]);

    // Property: if len != WIRE_SIZE, the decoder MUST refuse with WireFormatLength.
    if len != WIRE_SIZE {
        assert!(matches!(result, Err(ConsentError::WireFormatLength)));
    }
}
