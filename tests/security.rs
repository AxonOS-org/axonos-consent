// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The security boundary, attacked. Each test is one thing an adversary might
//! try; each must leave the machine exactly as it was.

#![cfg(feature = "ed25519")]

mod common;
use common::*;

use axonos_consent::wire::{ConsentRecord, BODY_LEN, FLAG_GUARDIAN, FRAME_LEN};
use axonos_consent::{
    ConsentError, ConsentMachine, ConsentState, Ed25519Strict, Persisted, Suppressed,
};
use ed25519_dalek::Signer;

fn machine() -> ConsentMachine<Ed25519Strict> {
    ConsentMachine::new(MANIFEST, public(&patient()), Ed25519Strict).expect("RFC 8032 key")
}

fn unchanged(m: &ConsentMachine<Ed25519Strict>, state: ConsentState, sequence: u64) {
    assert_eq!(m.state(), state);
    assert_eq!(m.last_sequence(), sequence);
}

#[test]
fn the_fixture_keys_are_the_rfc_8032_keys() {
    assert_eq!(public(&patient()), hex32(PATIENT_PUBLIC));
    assert_eq!(public(&guardian()), hex32(GUARDIAN_PUBLIC));
}

#[test]
fn a_signed_frame_is_admitted() {
    let mut m = machine();
    let frame = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Suspended, MANIFEST, 1, 0),
    );
    assert_eq!(m.handle(&frame), Ok(ConsentState::Suspended));
    unchanged(&m, ConsentState::Suspended, 1);
}

/// The defect of 0.8.0 and earlier: a tag computable from the public key was
/// accepted as proof. Knowing the public key must now be worth nothing.
#[test]
fn the_public_key_alone_forges_nothing() {
    let mut m = machine();
    let record = ConsentRecord::new(ConsentState::Withdrawn, MANIFEST, 1, 0).encode();
    let key = public(&patient());
    let mut candidates: Vec<[u8; 64]> = vec![[0u8; 64], [0xFF; 64]];
    let mut doubled = [0u8; 64];
    doubled[..32].copy_from_slice(&key);
    doubled[32..].copy_from_slice(&key);
    candidates.push(doubled);
    let other = ConsentRecord::new(ConsentState::Granted, MANIFEST, 9, 0).encode();
    candidates.push(patient().sign(&other).to_bytes());
    candidates.push(guardian().sign(&record).to_bytes());
    for signature in candidates {
        let mut frame = [0u8; FRAME_LEN];
        frame[..BODY_LEN].copy_from_slice(&record);
        frame[BODY_LEN..].copy_from_slice(&signature);
        assert_eq!(m.handle(&frame), Err(ConsentError::SignatureInvalid));
        unchanged(&m, ConsentState::Granted, 0);
    }
}

#[test]
fn every_single_bit_flip_is_refused() {
    let frame = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Withdrawn, MANIFEST, 1, 0),
    );
    for bit in 0..FRAME_LEN * 8 {
        let mut m = machine();
        let mut tampered = frame;
        tampered[bit / 8] ^= 1 << (bit % 8);
        assert!(
            m.handle(&tampered).is_err(),
            "bit {bit} flipped was admitted"
        );
        unchanged(&m, ConsentState::Granted, 0);
    }
}

/// RFC 8032 §5.1.7 malleability: adding the group order L to S produces a
/// second encoding of the same signature. Strict verification refuses it.
#[test]
fn a_non_canonical_signature_is_refused() {
    let mut frame = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Suspended, MANIFEST, 1, 0),
    );
    const L: [u8; 32] = [
        0xED, 0xD3, 0xF5, 0x5C, 0x1A, 0x63, 0x12, 0x58, 0xD6, 0x9C, 0xF7, 0xA2, 0xDE, 0xF9, 0xDE,
        0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
    ];
    let mut carry = 0u16;
    for i in 0..32 {
        let sum = u16::from(frame[BODY_LEN + 32 + i]) + u16::from(L[i]) + carry;
        frame[BODY_LEN + 32 + i] = sum as u8;
        carry = sum >> 8;
    }
    assert_eq!(carry, 0);
    let mut m = machine();
    assert_eq!(m.handle(&frame), Err(ConsentError::SignatureInvalid));
}

#[test]
fn a_replay_is_refused_and_survives_a_power_cycle() {
    let mut m = machine();
    let pause = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Suspended, MANIFEST, 5, 0),
    );
    m.handle(&pause).expect("first time");
    assert_eq!(m.handle(&pause), Err(ConsentError::Replay));
    let older = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Granted, MANIFEST, 4, 0),
    );
    assert_eq!(m.handle(&older), Err(ConsentError::Replay));

    let mut rebooted =
        ConsentMachine::restore(MANIFEST, public(&patient()), Ed25519Strict, m.persisted())
            .expect("restore");
    assert_eq!(rebooted.handle(&pause), Err(ConsentError::Replay));
    unchanged(&rebooted, ConsentState::Suspended, 5);
}

/// An authentic frame refused for its transition still spends its sequence
/// number, so it cannot be held back and replayed into a later state.
#[test]
fn a_refused_transition_still_consumes_its_sequence() {
    let mut m = ConsentMachine::restore(
        MANIFEST,
        public(&patient()),
        Ed25519Strict,
        Persisted {
            state: ConsentState::Withdrawn,
            patient_sequence: 0,
            guardian_sequence: 0,
        },
    )
    .expect("restore");
    let resume = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Granted, MANIFEST, 1, 0),
    );
    assert_eq!(m.handle(&resume), Err(ConsentError::InadmissibleTransition));
    assert_eq!(m.last_sequence(), 1);
    assert_eq!(m.handle(&resume), Err(ConsentError::Replay));
}

#[test]
fn the_sequence_is_spent_only_by_an_authentic_frame() {
    let mut m = machine();
    let mut forged = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Suspended, MANIFEST, 99, 0),
    );
    forged[FRAME_LEN - 1] ^= 0x80;
    assert_eq!(m.handle(&forged), Err(ConsentError::SignatureInvalid));
    unchanged(&m, ConsentState::Granted, 0);
}

#[test]
fn another_manifest_is_refused_without_spending_the_sequence() {
    let mut m = machine();
    let frame = sign(
        &patient(),
        ConsentRecord::new(ConsentState::Suspended, MANIFEST + 1, 1, 0),
    );
    assert_eq!(m.handle(&frame), Err(ConsentError::ManifestMismatch));
    unchanged(&m, ConsentState::Granted, 0);
}

#[test]
fn a_guardian_record_means_nothing_to_a_single_party_machine() {
    let mut m = machine();
    let record = ConsentRecord::new(ConsentState::Suspended, MANIFEST, 1, 0).by_guardian();
    assert_eq!(record.flags() & FLAG_GUARDIAN, FLAG_GUARDIAN);
    for key in [patient(), guardian()] {
        assert_eq!(
            m.handle(&sign(&key, record)),
            Err(ConsentError::SignatureInvalid)
        );
    }
    unchanged(&m, ConsentState::Granted, 0);
}

#[test]
fn a_small_order_key_is_refused_as_a_trust_anchor() {
    // The encoding of the identity point: a valid point of order one.
    let mut identity = [0u8; 32];
    identity[0] = 1;
    assert!(matches!(
        ConsentMachine::new(MANIFEST, identity, Ed25519Strict),
        Err(ConsentError::KeyInvalid)
    ));
}

#[test]
fn withdrawal_stops_publication_before_it_returns() {
    let mut m = machine();
    assert_eq!(m.gate().try_publish(), Ok(0));
    m.handle(&sign(
        &patient(),
        ConsentRecord::new(ConsentState::Withdrawn, MANIFEST, 1, 0),
    ))
    .expect("withdraw");
    assert_eq!(m.gate().try_publish(), Err(Suppressed::Withdrawn));
    assert_eq!(m.gate().published(), 1);
    assert_eq!(Suppressed::Withdrawn.abi_code(), 0x06);
}

#[test]
fn refusals_map_to_the_standard_abi_codes() {
    for (e, code) in [
        (ConsentError::WireFormatLength, 0x07),
        (ConsentError::BadMagic, 0x07),
        (ConsentError::ReservedDiscriminant, 0x07),
        (ConsentError::ReservedFlagBit, 0x07),
        (ConsentError::TerminalFlagMismatch, 0x07),
        (ConsentError::ReservedFieldNonZero, 0x07),
        (ConsentError::SignatureInvalid, 0x08),
        (ConsentError::Replay, 0x08),
        (ConsentError::KeyInvalid, 0x08),
        (ConsentError::ManifestMismatch, 0xFF),
        (ConsentError::InadmissibleTransition, 0xFF),
        (ConsentError::ConfigurationInvalid, 0xFF),
    ] {
        assert_eq!(e.to_abi_code(), code, "{e:?}");
    }
}
