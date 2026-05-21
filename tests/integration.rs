//! Integration tests exercising the public API of the consent crate.

use axonos_consent::crypto::compute_tag;
use axonos_consent::wire::{FLAG_REPLAY_TOLERANT, FLAG_TERMINAL};
use axonos_consent::{ConsentEvent, ConsentMachine, ConsentState};

fn make_event(state: u8, manifest_id: u16, pubkey: &[u8; 32], flags: u8) -> ConsentEvent {
    let mut event = ConsentEvent {
        state,
        flags,
        manifest_id,
        timestamp_us: 1_000_000,
        sig_truncated: 0,
    };
    event.sig_truncated = compute_tag(&event, pubkey);
    event
}

#[test]
fn fresh_machine_is_granted() {
    let pk = [0u8; 32];
    let m = ConsentMachine::new(1, pk);
    assert_eq!(m.state(), ConsentState::Granted);
}

#[test]
fn granted_to_suspended_to_granted_roundtrip() {
    let pk = [0xAAu8; 32];
    let mut m = ConsentMachine::new(7, pk);

    let suspend = make_event(0x02, 7, &pk, 0);
    assert_eq!(m.handle_event(suspend).unwrap(), ConsentState::Suspended);

    let resume = make_event(0x01, 7, &pk, 0);
    assert_eq!(m.handle_event(resume).unwrap(), ConsentState::Granted);
}

#[test]
fn withdraw_is_terminal_and_irreversible() {
    let pk = [0xAAu8; 32];
    let mut m = ConsentMachine::new(7, pk);

    let withdraw = make_event(0x03, 7, &pk, FLAG_TERMINAL);
    assert_eq!(m.handle_event(withdraw).unwrap(), ConsentState::Withdrawn);

    let restore = make_event(0x01, 7, &pk, 0);
    assert!(m.handle_event(restore).is_err());
    assert_eq!(m.state(), ConsentState::Withdrawn);

    let suspend = make_event(0x02, 7, &pk, 0);
    assert!(m.handle_event(suspend).is_err());
    assert_eq!(m.state(), ConsentState::Withdrawn);
}

#[test]
fn idempotent_reapplication_succeeds() {
    let pk = [0xAAu8; 32];
    let mut m = ConsentMachine::new(7, pk);

    let granted_again = make_event(0x01, 7, &pk, FLAG_REPLAY_TOLERANT);
    assert_eq!(
        m.handle_event(granted_again).unwrap(),
        ConsentState::Granted
    );
    assert_eq!(m.state(), ConsentState::Granted);
}

#[test]
fn wrong_manifest_id_refused() {
    let pk = [0xAAu8; 32];
    let mut m = ConsentMachine::new(7, pk);

    let event = make_event(0x02, 99, &pk, 0);
    assert!(m.handle_event(event).is_err());
    assert_eq!(m.state(), ConsentState::Granted);
}

#[test]
fn invalid_signature_refused() {
    let pk = [0xAAu8; 32];
    let mut m = ConsentMachine::new(7, pk);

    let mut event = make_event(0x02, 7, &pk, 0);
    event.sig_truncated = event.sig_truncated.wrapping_add(1);
    assert!(m.handle_event(event).is_err());
    assert_eq!(m.state(), ConsentState::Granted);
}
