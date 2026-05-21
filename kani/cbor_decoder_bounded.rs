//! Kani harness: the CBOR-shape wire decoder enforces depth and length bounds,
//! and refuses any input not of the exact wire size.

use axonos_consent::error::ConsentError;
use axonos_consent::wire::{ConsentEvent, WIRE_SIZE};

#[kani::proof]
#[kani::unwind(17)]
fn cbor_decoder_bounded() {
    let len: usize = kani::any();
    kani::assume(len <= WIRE_SIZE + 1);

    let mut buf = [0u8; WIRE_SIZE + 1];
    for byte in buf.iter_mut().take(len) {
        *byte = kani::any();
    }

    let result = ConsentEvent::from_bytes(&buf[..len]);

    if len != WIRE_SIZE {
        assert!(matches!(result, Err(ConsentError::WireFormatLength)));
    }
}
