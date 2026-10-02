// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! FSM invariants (SPEC §2, §3, §7.5, §9) under arbitrary streams of records.
//!
//! Authentication is deliberately out of the loop: the verifier here accepts
//! every signature, so the fuzzer explores the state machine, the sequence
//! rule and the gate rather than spending its time on Ed25519. The signature
//! path has its own target, `auth_forgery`.

#![no_main]

use axonos_consent::wire::{BODY_LEN, FRAME_LEN, SIGNATURE_LEN};
use axonos_consent::{ConsentMachine, ConsentState, SignatureVerifier};
use libfuzzer_sys::fuzz_target;

struct AcceptEverySignature;
impl SignatureVerifier for AcceptEverySignature {
    fn verify(&self, _: &[u8; 32], _: &[u8; BODY_LEN], _: &[u8; SIGNATURE_LEN]) -> bool {
        true
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(mut machine) = ConsentMachine::new(7, [0x5A; 32], AcceptEverySignature) else {
        return;
    };
    let mut withdrawn = false;
    for body in data.chunks_exact(BODY_LEN) {
        let mut frame = [0u8; FRAME_LEN];
        frame[..BODY_LEN].copy_from_slice(body);
        let before_sequence = machine.last_sequence();
        let before_published = machine.gate().published();
        let granted = machine.state() == ConsentState::Granted;
        let published = machine.gate().try_publish();
        assert_eq!(published.is_ok(), granted);
        if published.is_err() {
            assert_eq!(machine.gate().published(), before_published);
        }
        let _ = machine.handle(&frame);
        assert!(machine.last_sequence() >= before_sequence);
        if withdrawn {
            assert_eq!(machine.state(), ConsentState::Withdrawn);
        }
        withdrawn = machine.state() == ConsentState::Withdrawn;
    }
});
