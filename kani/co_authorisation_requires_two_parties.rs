//! Kani harness: the exposure-increasing transition `Suspended → Granted`
//! commits only when two *distinct* parties co-authorise it.
//!
//! The security core of dual control is isolated in the pure predicate
//! `co_authorisation_complete`; this harness proves that no single party can
//! ever satisfy it, for every combination of parties.
//!
//! See SPEC §13 and `src/dual_control.rs`.

use axonos_consent::dual_control::{co_authorisation_complete, is_exposure_increasing, Party};
use axonos_consent::ConsentState;

fn any_party() -> Party {
    if kani::any() {
        Party::Patient
    } else {
        Party::Guardian
    }
}

fn any_state() -> ConsentState {
    match kani::any::<u8>() % 3 {
        0 => ConsentState::Granted,
        1 => ConsentState::Suspended,
        _ => ConsentState::Withdrawn,
    }
}

#[kani::proof]
fn co_authorisation_requires_two_parties() {
    let pending = any_party();
    let incoming = any_party();

    // If the co-authorisation predicate is satisfied, the two parties differ.
    if co_authorisation_complete(pending, incoming) {
        assert!((pending as u8) != (incoming as u8));
    }

    // A single party can never co-authorise with itself, for either party.
    assert!(!co_authorisation_complete(Party::Patient, Party::Patient));
    assert!(!co_authorisation_complete(Party::Guardian, Party::Guardian));
}

#[kani::proof]
fn only_resume_is_exposure_increasing() {
    let from = any_state();
    let to = any_state();

    // The single exposure-increasing transition is Suspended → Granted.
    // Every other admissible transition is exposure-neutral or -reducing.
    if is_exposure_increasing(from, to) {
        assert!(matches!(from, ConsentState::Suspended));
        assert!(matches!(to, ConsentState::Granted));
    }
}
