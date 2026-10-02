// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Kani proofs: `cargo kani --no-default-features`.
//!
//! Each harness states one property of SPEC §13 and proves it for every value
//! of its symbolic inputs: every 32-byte record, every 96-byte frame, every
//! stored byte, every gate word, every sequence number and clock reading.
//! Signature verification is replaced by a verifier that answers `false`,
//! `true` or anything at all, so the proofs hold whatever the real verifier
//! decides; the verifier itself (Ed25519, RFC 8032) is the one assumption,
//! and it is tested against RFC 8032 and the conformance vectors instead.
//!
//! Unwind bounds. The only loops on these paths are the gate's
//! compare-and-swap, which completes in one iteration without contention, and
//! byte-array comparisons, which Kani's library models as loops of one
//! iteration per byte plus one: at most 33 here, for a 32-byte key or record.
//! Every bounded harness therefore uses 40. A bound below a loop's length stops
//! the proof at an unwinding assertion before it reaches its property — which
//! is what three harnesses did in 0.9.0, with a bound of 4.

use crate::auth::SignatureVerifier;
use crate::dual_control::DualControlMachine;
use crate::gate::{PublicationGate, COUNT_MASK};
use crate::machine::{ConsentMachine, Persisted};
use crate::state::{is_admissible_transition, ConsentState};
use crate::wire::{
    ConsentRecord, BODY_LEN, FLAGS_DEFINED_MASK, FLAG_GUARDIAN, FLAG_TERMINAL, FRAME_LEN, MAGIC,
    SIGNATURE_LEN,
};

/// Refuses every signature.
struct Never;
impl SignatureVerifier for Never {
    fn verify(&self, _: &[u8; 32], _: &[u8; BODY_LEN], _: &[u8; SIGNATURE_LEN]) -> bool {
        false
    }
}

/// Accepts or refuses, nondeterministically: the proof covers both answers.
struct Either;
impl SignatureVerifier for Either {
    fn verify(&self, _: &[u8; 32], _: &[u8; BODY_LEN], _: &[u8; SIGNATURE_LEN]) -> bool {
        kani::any()
    }
}

/// Accepts every signature.
struct Always;
impl SignatureVerifier for Always {
    fn verify(&self, _: &[u8; 32], _: &[u8; BODY_LEN], _: &[u8; SIGNATURE_LEN]) -> bool {
        true
    }
}

fn any_state() -> ConsentState {
    match kani::any::<u8>() % 3 {
        0 => ConsentState::Granted,
        1 => ConsentState::Suspended,
        _ => ConsentState::Withdrawn,
    }
}

fn sequence_of(frame: &[u8; FRAME_LEN]) -> u64 {
    u64::from_le_bytes([
        frame[8], frame[9], frame[10], frame[11], frame[12], frame[13], frame[14], frame[15],
    ])
}

/// SPEC §6.6 — decoding is total, and every accepted record is canonical and
/// satisfies the record invariants.
#[kani::proof]
fn wire_decode_is_total_and_canonical() {
    let body: [u8; BODY_LEN] = kani::any();
    if let Ok(record) = ConsentRecord::decode(&body) {
        assert!(record.encode() == body);
        assert!([body[0], body[1], body[2], body[3]] == MAGIC);
        assert!(record.flags() & !FLAGS_DEFINED_MASK == 0);
        assert!((record.flags() & FLAG_TERMINAL != 0) == record.state().is_terminal());
    }
}

/// SPEC §2.3 — a stored byte that is not `Granted` or `Suspended` reads as
/// `Withdrawn`; corruption can never read as `Granted`.
#[kani::proof]
fn stored_state_fails_closed() {
    let byte: u8 = kani::any();
    let state = ConsentState::from_stored(byte);
    if byte != 0x01 && byte != 0x02 {
        assert!(state == ConsentState::Withdrawn);
    }
    if state == ConsentState::Granted {
        assert!(byte == 0x01);
    }
}

/// SPEC §3.3 — nothing leaves `Withdrawn`.
#[kani::proof]
fn withdrawn_is_absorbing() {
    let target = any_state();
    if is_admissible_transition(ConsentState::Withdrawn, target) {
        assert!(target == ConsentState::Withdrawn);
    }
}

/// SPEC §7.1 — a frame whose signature does not verify changes nothing: not
/// the state, not the sequence, not the publication count.
#[kani::proof]
#[kani::unwind(40)]
fn no_transition_without_authentication() {
    let frame: [u8; FRAME_LEN] = kani::any();
    let persisted = Persisted {
        state: any_state(),
        patient_sequence: kani::any(),
        guardian_sequence: 0,
    };
    let Ok(mut machine) = ConsentMachine::restore(kani::any(), [7u8; 32], Never, persisted) else {
        return;
    };
    let before = (
        machine.state(),
        machine.last_sequence(),
        machine.gate().published(),
    );
    assert!(machine.handle(&frame).is_err());
    assert!(
        (
            machine.state(),
            machine.last_sequence(),
            machine.gate().published()
        ) == before
    );
}

/// SPEC §7.5 — no sequence number is admitted twice, and the consumed sequence
/// never moves backwards.
#[kani::proof]
#[kani::unwind(40)]
fn no_sequence_is_admitted_twice() {
    let frame: [u8; FRAME_LEN] = kani::any();
    let last: u64 = kani::any();
    let persisted = Persisted {
        state: any_state(),
        patient_sequence: last,
        guardian_sequence: 0,
    };
    let Ok(mut machine) = ConsentMachine::restore(kani::any(), [7u8; 32], Either, persisted) else {
        return;
    };
    let sequence = sequence_of(&frame);
    if machine.handle(&frame).is_ok() {
        assert!(sequence > last);
        assert!(machine.last_sequence() == sequence);
    }
    assert!(machine.last_sequence() >= last);
}

/// SPEC §3.3, §9 — a withdrawn machine stays withdrawn and publishes nothing,
/// whatever frame arrives.
#[kani::proof]
#[kani::unwind(40)]
fn withdrawn_machine_stays_withdrawn() {
    let frame: [u8; FRAME_LEN] = kani::any();
    let persisted = Persisted {
        state: ConsentState::Withdrawn,
        patient_sequence: kani::any(),
        guardian_sequence: 0,
    };
    let Ok(mut machine) = ConsentMachine::restore(kani::any(), [7u8; 32], Either, persisted) else {
        return;
    };
    let _ = machine.handle(&frame);
    assert!(machine.state() == ConsentState::Withdrawn);
    assert!(machine.gate().try_publish().is_err());
}

/// SPEC §9.2 — the gate commits a publication only while its word reads
/// `Granted`, and a refused publication leaves the word untouched.
#[kani::proof]
#[kani::unwind(40)]
fn gate_publishes_only_while_granted() {
    let raw: u32 = kani::any();
    let gate = PublicationGate::from_raw(raw);
    match gate.try_publish() {
        Ok(index) => {
            assert!(raw & 0b11 == ConsentState::Granted as u32);
            assert!(index == raw >> 2);
            assert!(gate.raw() == ((((raw >> 2) + 1) & COUNT_MASK) << 2) | (raw & 0b11));
        }
        Err(_) => {
            assert!(raw & 0b11 != ConsentState::Granted as u32);
            assert!(gate.raw() == raw);
        }
    }
}

/// SPEC §9.2 — once the gate holds `Withdrawn` it holds it for good, keeps its
/// count, and refuses every publication.
#[kani::proof]
#[kani::unwind(40)]
fn gate_withdrawal_is_absorbing() {
    let raw: u32 = kani::any();
    let gate = PublicationGate::from_raw(raw);
    gate.set_state(ConsentState::Withdrawn);
    gate.set_state(any_state());
    assert!(gate.state() == ConsentState::Withdrawn);
    assert!(gate.raw() >> 2 == raw >> 2);
    assert!(gate.try_publish().is_err());
}

/// SPEC §12.3 — starting from `Suspended`, two frames from the same party can
/// never resume the flow, whatever they contain and whenever they arrive.
#[kani::proof]
#[kani::unwind(40)]
fn one_party_cannot_resume() {
    let first: [u8; FRAME_LEN] = kani::any();
    let second: [u8; FRAME_LEN] = kani::any();
    kani::assume(first[5] & FLAG_GUARDIAN == second[5] & FLAG_GUARDIAN);
    let persisted = Persisted {
        state: ConsentState::Suspended,
        patient_sequence: kani::any(),
        guardian_sequence: kani::any(),
    };
    let window: u64 = kani::any();
    let Ok(mut machine) =
        DualControlMachine::restore(1, [1u8; 32], [2u8; 32], Either, persisted, window)
    else {
        return;
    };
    let _ = machine.propose(&first, kani::any());
    let _ = machine.propose(&second, kani::any());
    assert!(machine.state() != ConsentState::Granted);
}

/// SPEC §12.2 — one key can never serve as both parties.
#[kani::proof]
#[kani::unwind(40)]
fn one_key_cannot_be_both_parties() {
    let key: [u8; 32] = kani::any();
    let window: u64 = kani::any();
    assert!(DualControlMachine::restore(1, key, key, Always, Persisted::FRESH, window).is_err());
}
