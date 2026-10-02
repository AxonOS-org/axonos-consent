// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Dual control: one party can stop, only both can resume.

#![cfg(feature = "ed25519")]

mod common;
use common::*;

use axonos_consent::dual_control::DEFAULT_CO_AUTH_WINDOW_US;
use axonos_consent::wire::{ConsentRecord, FLAG_GUARDIAN};
use axonos_consent::{
    CoAuthOutcome, ConsentError, ConsentState, DualControlMachine, Ed25519Strict, Party, Persisted,
};

const WINDOW: u64 = 1_000;

fn machine(state: ConsentState) -> DualControlMachine<Ed25519Strict> {
    DualControlMachine::restore(
        MANIFEST,
        public(&patient()),
        public(&guardian()),
        Ed25519Strict,
        Persisted {
            state,
            patient_sequence: 0,
            guardian_sequence: 0,
        },
        WINDOW,
    )
    .expect("two distinct RFC 8032 keys")
}

fn by_patient(state: ConsentState, seq: u64) -> [u8; 96] {
    sign(&patient(), ConsentRecord::new(state, MANIFEST, seq, 0))
}

fn by_guardian(state: ConsentState, seq: u64) -> [u8; 96] {
    sign(
        &guardian(),
        ConsentRecord::new(state, MANIFEST, seq, 0).by_guardian(),
    )
}

#[test]
fn either_party_stops_alone() {
    for (frame, party) in [
        (by_patient(ConsentState::Withdrawn, 1), "patient"),
        (by_guardian(ConsentState::Withdrawn, 1), "guardian"),
    ] {
        let mut m = machine(ConsentState::Granted);
        assert_eq!(
            m.propose(&frame, 0),
            Ok(CoAuthOutcome::Applied(ConsentState::Withdrawn)),
            "{party}"
        );
    }
}

#[test]
fn resume_needs_both_parties_within_the_window() {
    let mut m = machine(ConsentState::Suspended);
    assert_eq!(
        m.propose(&by_patient(ConsentState::Granted, 1), 10),
        Ok(CoAuthOutcome::PendingCoAuth(ConsentState::Suspended))
    );
    assert_eq!(m.pending(), Some(Party::Patient));
    assert_eq!(
        m.propose(&by_guardian(ConsentState::Granted, 1), 10 + WINDOW),
        Ok(CoAuthOutcome::Applied(ConsentState::Granted))
    );
    assert_eq!(m.pending(), None);
}

#[test]
fn one_party_never_resumes() {
    let mut m = machine(ConsentState::Suspended);
    for seq in 1..=64 {
        assert_eq!(
            m.propose(&by_patient(ConsentState::Granted, seq), seq),
            Ok(CoAuthOutcome::PendingCoAuth(ConsentState::Suspended))
        );
    }
    assert_eq!(m.state(), ConsentState::Suspended);
}

#[test]
fn a_stale_first_half_is_rearmed_not_completed() {
    let mut m = machine(ConsentState::Suspended);
    m.propose(&by_patient(ConsentState::Granted, 1), 0)
        .expect("arm");
    assert_eq!(
        m.propose(&by_guardian(ConsentState::Granted, 1), WINDOW + 1),
        Ok(CoAuthOutcome::PendingCoAuth(ConsentState::Suspended))
    );
    assert_eq!(m.pending(), Some(Party::Guardian));
    assert_eq!(
        m.propose(&by_patient(ConsentState::Granted, 2), WINDOW + 2),
        Ok(CoAuthOutcome::Applied(ConsentState::Granted))
    );
}

/// The window runs on the kernel's clock. Timestamps written by the signers
/// cannot stretch it or shrink it.
#[test]
fn signer_timestamps_do_not_move_the_window() {
    let mut m = machine(ConsentState::Suspended);
    let early = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Granted, MANIFEST, 1, 0),
    );
    let late = sign(
        &guardian(),
        ConsentRecord::new(ConsentState::Granted, MANIFEST, 1, u64::MAX).by_guardian(),
    );
    m.propose(&early, 500).expect("arm");
    assert_eq!(
        m.propose(&late, 600),
        Ok(CoAuthOutcome::Applied(ConsentState::Granted))
    );

    let mut m = machine(ConsentState::Suspended);
    let close_a = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Granted, MANIFEST, 1, 7),
    );
    let close_b = sign(
        &guardian(),
        ConsentRecord::new(ConsentState::Granted, MANIFEST, 1, 7).by_guardian(),
    );
    m.propose(&close_a, 0).expect("arm");
    assert_eq!(
        m.propose(&close_b, 10 * WINDOW),
        Ok(CoAuthOutcome::PendingCoAuth(ConsentState::Suspended))
    );
}

#[test]
fn the_role_is_signed_and_cannot_be_relabelled() {
    let mut m = machine(ConsentState::Suspended);
    let mut relabelled = by_guardian(ConsentState::Suspended, 1);
    relabelled[5] &= !FLAG_GUARDIAN;
    assert_eq!(
        m.propose(&relabelled, 0),
        Err(ConsentError::SignatureInvalid)
    );
    let mut promoted = by_patient(ConsentState::Suspended, 1);
    promoted[5] |= FLAG_GUARDIAN;
    assert_eq!(m.propose(&promoted, 0), Err(ConsentError::SignatureInvalid));
}

#[test]
fn sequences_are_kept_per_party() {
    let mut m = machine(ConsentState::Granted);
    m.propose(&by_patient(ConsentState::Suspended, 3), 0)
        .expect("patient");
    m.propose(&by_guardian(ConsentState::Suspended, 1), 0)
        .expect("guardian, own counter");
    assert_eq!(
        m.propose(&by_patient(ConsentState::Suspended, 3), 0),
        Err(ConsentError::Replay)
    );
    assert_eq!(m.last_sequence(Party::Patient), 3);
    assert_eq!(m.last_sequence(Party::Guardian), 1);
    assert_eq!(m.persisted().guardian_sequence, 1);
}

#[test]
fn re_asserting_the_pause_cancels_a_pending_resume() {
    let mut m = machine(ConsentState::Suspended);
    m.propose(&by_patient(ConsentState::Granted, 1), 0)
        .expect("arm");
    m.propose(&by_guardian(ConsentState::Suspended, 1), 1)
        .expect("re-assert");
    assert_eq!(m.pending(), None);
    assert_eq!(
        m.propose(&by_guardian(ConsentState::Granted, 2), 2),
        Ok(CoAuthOutcome::PendingCoAuth(ConsentState::Suspended))
    );
}

#[test]
fn withdrawal_stays_terminal_under_dual_control() {
    let mut m = machine(ConsentState::Withdrawn);
    assert_eq!(
        m.propose(&by_patient(ConsentState::Granted, 1), 0),
        Err(ConsentError::InadmissibleTransition)
    );
    assert_eq!(
        m.propose(&by_guardian(ConsentState::Granted, 1), 0),
        Err(ConsentError::InadmissibleTransition)
    );
}

#[test]
fn configurations_the_specification_forbids_are_refused() {
    let key = public(&patient());
    assert!(matches!(
        DualControlMachine::new(MANIFEST, key, key, Ed25519Strict),
        Err(ConsentError::ConfigurationInvalid)
    ));
    assert!(matches!(
        DualControlMachine::restore(
            MANIFEST,
            key,
            public(&guardian()),
            Ed25519Strict,
            Persisted::FRESH,
            0
        ),
        Err(ConsentError::ConfigurationInvalid)
    ));
    let m = DualControlMachine::new(MANIFEST, key, public(&guardian()), Ed25519Strict)
        .expect("default window");
    assert_eq!(m.window_us(), DEFAULT_CO_AUTH_WINDOW_US);
}
