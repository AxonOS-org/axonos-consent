//! Wire-format conformance tests against canonical vectors.

use axonos_consent::wire::{FLAG_TERMINAL, WIRE_SIZE};
use axonos_consent::{ConsentError, ConsentEvent};

#[test]
fn canonical_withdrawal_event_roundtrip() {
    let event = ConsentEvent {
        state: 0x03,
        flags: FLAG_TERMINAL,
        manifest_id: 1,
        timestamp_us: 1_700_000_000_000_000,
        sig_truncated: 0xDEAD_BEEF,
    };
    let bytes = event.to_bytes();

    assert_eq!(bytes[0], 0x03);
    assert_eq!(bytes[1], FLAG_TERMINAL);
    assert_eq!(&bytes[2..4], &[0x01, 0x00]);
    assert_eq!(&bytes[12..16], &0xDEAD_BEEFu32.to_le_bytes());

    let decoded = ConsentEvent::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn refuses_15_byte_buffer() {
    assert!(matches!(
        ConsentEvent::from_bytes(&[0u8; 15]),
        Err(ConsentError::WireFormatLength)
    ));
}

#[test]
fn refuses_17_byte_buffer() {
    assert!(matches!(
        ConsentEvent::from_bytes(&[0u8; 17]),
        Err(ConsentError::WireFormatLength)
    ));
}

#[test]
fn wire_size_is_16() {
    assert_eq!(WIRE_SIZE, 16);
}
